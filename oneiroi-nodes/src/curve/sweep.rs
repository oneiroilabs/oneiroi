//! The surface a profile leaves behind when it is moved along a curve.

use std::{f32::consts::TAU, marker::PhantomData};

use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3, Vec3Swizzles, Vec4};
use oneiroi_core::{
    node::{Caps, GenericPort, GpuKernel, GpuOutput, Inputs, NodeDef, NodeRegistry, Outputs, PortDecl},
    program::{BuildError, Program, ProgramBuilder, SlotId},
    types::{DataType, Field, FieldType, Layout, Scalar},
};

use super::{
    fractions,
    nurbs::{CubicNurbsSegmentCache, MovingFrame},
    profile::{PROFILE, Profile},
};
use crate::{
    generic::version,
    instance::Placement,
    primitive::CIRCLE,
    surface::{self, SURFACE, Surface, register_surface},
};

/// What a patch of a sweep is: a piece of the side, or a piece of the flat
/// cap at an end of the curve.
pub mod kind {
    /// A piece of the side, along a segment of the curve.
    pub const SIDE: u32 = 0;
    /// A cap at the start of the curve, from its middle out to the side.
    pub const START: u32 = 1;
    /// A cap at the end of the curve, from the side in to its middle.
    pub const END: u32 = 2;
    /// A cap which is not there, of a closed curve: all of it is a point.
    pub const NONE: u32 = 3;
}

/// How far a point `offset` across the curve, where it heads `tangent`,
/// is moved along it to lie on the plane through the curve with the normal
/// `miter`: where it meets its neighbour at a corner. Nothing without one.
fn cut(offset: Vec3, tangent: Vec3, miter: [f32; 4]) -> f32 {
    let miter = Vec4::from(miter).truncate();
    let facing = tangent.dot(miter);
    match facing.abs() < 1e-3 {
        true => 0.0,
        false => -offset.dot(miter) / facing,
    }
}

/// Where a cap is, and how far out its point at `u` is: a cap at the start
/// grows from its middle, one at the end shrinks to it, so both face out.
fn cap(segment: &CubicNurbsSegmentCache, kind: u32, u: f32) -> (MovingFrame, f32, Vec3) {
    let at_end = kind == kind::END;
    let frame = segment.frame(if at_end { 1.0 } else { 0.0 });
    let scale = match kind {
        kind::START => u,
        kind::END => 1.0 - u,
        _ => 0.0,
    };
    let facing = if at_end { frame.tangent } else { -frame.tangent };
    (frame, scale, facing)
}

/// Where a patch is along a curve and how it is cut and scaled: what a
/// [`Profile`] makes a patch of.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece {
    pub segment: CubicNurbsSegmentCache,
    /// The part of the segment it covers, from `0` to `1`.
    pub span: [f32; 2],
    /// Where along the whole curve it starts and ends.
    pub along: [f32; 2],
    /// See [`kind`].
    pub kind: u32,
    /// See [`TubePatch`].
    pub miters: [[f32; 4]; 2],
    /// The scale of the profile along it, `a + b u + c u² + d u³`.
    pub scale: [f32; 4],
}

impl Piece {
    /// All of `segment`, at its own size.
    pub fn whole(segment: CubicNurbsSegmentCache, along: [f32; 2], kind: u32, miters: [[f32; 4]; 2]) -> Self {
        Self {
            segment,
            span: [0.0, 1.0],
            along,
            kind,
            miters,
            scale: [1.0, 0.0, 0.0, 0.0],
        }
    }
}

/// The scale `a + b u + c u² + d u³` at `u`, and how fast it changes.
fn scale_at(scale: [f32; 4], u: f32) -> (f32, f32) {
    let [a, b, c, d] = scale;
    (a + u * (b + u * (c + u * d)), b + u * (2.0 * c + u * 3.0 * d))
}

/// The frame of the spine at `u` of a patch which covers `span` of
/// `segment`, and how fast the spine runs there per `u`.
fn spine_at(segment: &CubicNurbsSegmentCache, span: [f32; 2], u: f32) -> (MovingFrame, f32) {
    let (frame, velocity) = segment.frame_and_velocity(lerp(span[0], span[1], u));
    (frame, velocity.length() * (span[1] - span[0]))
}

/// The normal of a swept surface where the profile, at `offset` from the
/// spine with the normal `outwards` there, grows by `growth` per `u` while
/// the spine runs `speed` per `u` along `tangent`: it leans back where the
/// surface widens, like on a cone.
fn leaning(outwards: Vec3, offset: Vec3, tangent: Vec3, speed: f32, growth: f32) -> Vec3 {
    (outwards * speed - tangent * outwards.dot(offset) * growth)
        .try_normalize()
        .unwrap_or(outwards)
}

/// A circle swept along one segment of a curve, or a cap of the tube.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct TubePatch {
    segment: CubicNurbsSegmentCache,
    radius: f32,
    /// Where along the whole curve the segment starts and ends.
    along: [f32; 2],
    /// See [`kind`].
    kind: u32,
    /// The planes the tube is cut at, at the start of the segment and at
    /// its end, as their normals: where it meets the tube of the segment
    /// next to it at an angle. A normal of `0` cuts straight across.
    miters: [[f32; 4]; 2],
    /// See [`Piece`].
    span: [f32; 2],
    scale: [f32; 4],
}

impl TubePatch {
    pub fn new(segment: CubicNurbsSegmentCache, radius: f32, along: [f32; 2]) -> Self {
        Self::of(&Piece::whole(segment, along, kind::SIDE, [[0.0; 4]; 2]), radius)
    }

    pub fn of(piece: &Piece, radius: f32) -> Self {
        Self {
            segment: piece.segment,
            radius,
            along: piece.along,
            kind: piece.kind,
            miters: piece.miters,
            span: piece.span,
            scale: piece.scale,
        }
    }

    pub fn kind(&self) -> u32 {
        self.kind
    }

    /// The same tube put somewhere else, see [`Placement`].
    pub fn placed(&self, place: &Placement) -> Self {
        let turn = |miter: [f32; 4]| (place.rotation * Vec4::from(miter).truncate()).extend(0.0).to_array();
        Self {
            segment: self.segment.placed(place.rotation, place.scale, place.offset),
            radius: self.radius * place.scale,
            miters: self.miters.map(turn),
            ..*self
        }
    }
}

impl DataType for TubePatch {
    type ConfigurationOptions = ();

    const NAME: &'static str = "tube_patch";
    const WGSL: &'static str = "TubePatch";
    const SCALAR: Scalar = Scalar::F32;
    const LAYOUT: Layout = Layout::Struct(&[
        Field::nested("segment", CubicNurbsSegmentCache::INFO),
        Field::new("radius", FieldType::F32),
        Field::new("along", FieldType::Vec2),
        Field::new("kind", FieldType::U32),
        Field::new("miter_start", FieldType::Vec4),
        Field::new("miter_end", FieldType::Vec4),
        Field::new("span", FieldType::Vec2),
        Field::new("scale", FieldType::Vec4),
    ]);
}

describe_type! {
    /// A tube around a piece of a curve, what a circle sweeps out, or a cap at
    /// an end of it.
    TubePatch { label: "Tube", hue: Emerald }
}

impl Surface for TubePatch {
    fn point(&self, u: f32, v: f32) -> (Vec3, Vec3) {
        let around = Vec2::from_angle(v * TAU);
        if self.kind != kind::SIDE {
            let (frame, scale, facing) = cap(&self.segment, self.kind, u);
            let out = frame.normal * around.x + frame.binormal * around.y;
            return (frame.position + out * self.radius * scale * self.scale[0], facing);
        }
        let (frame, speed) = spine_at(&self.segment, self.span, u);
        let (scale, growth) = scale_at(self.scale, u);
        let outwards = frame.normal * around.x + frame.binormal * around.y;
        let offset = outwards * self.radius * scale;
        let along = lerp(cut(offset, frame.tangent, self.miters[0]), cut(offset, frame.tangent, self.miters[1]), u);
        let normal = leaning(outwards, outwards * self.radius, frame.tangent, speed, growth);
        (frame.position + offset + frame.tangent * along, normal)
    }

    fn uv(&self, u: f32, v: f32) -> Vec2 {
        match self.kind {
            kind::SIDE => Vec2::new(v, self.along[0] + (self.along[1] - self.along[0]) * u),
            _ => Vec2::new(v, self.along[0]),
        }
    }

    const WGSL: &'static str = concat!(include_str!("curve.wgsl"), include_str!("cut.wgsl"), include_str!("tube.wgsl"));
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// One segment of a profile swept along one segment of a curve, or a piece
/// of a cap.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct SweptPatch {
    spine: CubicNurbsSegmentCache,
    profile: CubicNurbsSegmentCache,
    /// Where along the whole curve the segment of the spine starts and ends.
    along: [f32; 2],
    /// The same for the segment of the profile.
    around: [f32; 2],
    /// See [`TubePatch`].
    miters: [[f32; 4]; 2],
    /// See [`kind`].
    kind: u32,
    /// See [`Piece`].
    span: [f32; 2],
    scale: [f32; 4],
}

impl SweptPatch {
    /// The same surface put somewhere else, see [`Placement`]. The profile
    /// lies in the frames of the spine, which turn along with it, so it
    /// only grows with the scale.
    pub fn placed(&self, place: &Placement) -> Self {
        let turn = |miter: [f32; 4]| (place.rotation * Vec4::from(miter).truncate()).extend(0.0).to_array();
        Self {
            spine: self.spine.placed(place.rotation, place.scale, place.offset),
            profile: self.profile.placed(glam::Mat3::IDENTITY, place.scale, Vec3::ZERO),
            miters: self.miters.map(turn),
            ..*self
        }
    }

    pub fn new(
        spine: CubicNurbsSegmentCache,
        profile: CubicNurbsSegmentCache,
        along: [f32; 2],
        around: [f32; 2],
    ) -> Self {
        Self::of(&Piece::whole(spine, along, kind::SIDE, [[0.0; 4]; 2]), profile, around)
    }

    pub fn of(piece: &Piece, profile: CubicNurbsSegmentCache, around: [f32; 2]) -> Self {
        Self {
            spine: piece.segment,
            profile,
            along: piece.along,
            around,
            miters: piece.miters,
            kind: piece.kind,
            span: piece.span,
            scale: piece.scale,
        }
    }

    pub fn kind(&self) -> u32 {
        self.kind
    }
}

impl DataType for SweptPatch {
    type ConfigurationOptions = ();

    const NAME: &'static str = "swept_patch";
    const WGSL: &'static str = "SweptPatch";
    const SCALAR: Scalar = Scalar::F32;
    const LAYOUT: Layout = Layout::Struct(&[
        Field::nested("spine", CubicNurbsSegmentCache::INFO),
        Field::nested("profile", CubicNurbsSegmentCache::INFO),
        Field::new("along", FieldType::Vec2),
        Field::new("around", FieldType::Vec2),
        Field::new("miter_start", FieldType::Vec4),
        Field::new("miter_end", FieldType::Vec4),
        Field::new("kind", FieldType::U32),
        Field::new("span", FieldType::Vec2),
        Field::new("scale", FieldType::Vec4),
    ]);
}

describe_type! {
    /// A piece of the surface a curve sweeps out along another one, or of a
    /// cap at an end of it.
    SweptPatch { label: "Swept surface", hue: Teal }
}

impl Surface for SweptPatch {
    fn point(&self, u: f32, v: f32) -> (Vec3, Vec3) {
        let (outline, _, tangent) = self.profile.evaluate(v);
        if self.kind != kind::SIDE {
            let (frame, scale, facing) = cap(&self.spine, self.kind, u);
            let across = frame.normal * outline.x + frame.binormal * outline.y;
            return (frame.position + across * scale * self.scale[0], facing);
        }
        let (frame, speed) = spine_at(&self.spine, self.span, u);
        let (scale, growth) = scale_at(self.scale, u);
        let tangent = tangent.xy().normalize_or_zero();
        // To the right of the direction the profile runs in.
        let outwards = Vec2::new(tangent.y, -tangent.x);
        let outwards = frame.normal * outwards.x + frame.binormal * outwards.y;
        let across = frame.normal * outline.x + frame.binormal * outline.y;
        let offset = across * scale;
        let along = lerp(cut(offset, frame.tangent, self.miters[0]), cut(offset, frame.tangent, self.miters[1]), u);
        (
            frame.position + offset + frame.tangent * along,
            leaning(outwards, across, frame.tangent, speed, growth),
        )
    }

    fn uv(&self, u: f32, v: f32) -> Vec2 {
        let around = self.around[0] + (self.around[1] - self.around[0]) * v;
        match self.kind {
            kind::SIDE => Vec2::new(around, self.along[0] + (self.along[1] - self.along[0]) * u),
            _ => Vec2::new(around, self.along[0]),
        }
    }

    const WGSL: &'static str = concat!(include_str!("curve.wgsl"), include_str!("cut.wgsl"), include_str!("swept.wgsl"));
}

/// Where each segment of one curve is cut at its ends: across where it meets
/// the one before or after at an angle, so that both meet on the plane
/// between them. A curve which ends where it starts is cut there too, and
/// has no caps.
pub(crate) fn miters(spine: &[CubicNurbsSegmentCache]) -> (bool, Vec<[[f32; 4]; 2]>) {
    let (Some(first), Some(last)) = (spine.first(), spine.last()) else {
        return (false, Vec::new());
    };
    let start = first.evaluate(0.0).0;
    let gap = start.distance(last.evaluate(1.0).0);
    let closed = spine.len() > 1 && gap <= 1e-5 * (1.0 + last.cumulative_length());
    let tangent = |segment: &CubicNurbsSegmentCache, u: f32| segment.evaluate(u).2;
    let between = |from: Vec3, to: Vec3| {
        let turn = from.dot(to);
        match turn > 1.0 - 1e-4 || turn < -1.0 + 1e-4 {
            true => [0.0; 4],
            false => (from + to).normalize().extend(0.0).to_array(),
        }
    };
    let count = spine.len();
    let cuts = (0..count)
        .map(|index| {
            let before = match index {
                0 if closed => Some(count - 1),
                0 => None,
                _ => Some(index - 1),
            };
            let after = match index + 1 == count {
                true if closed => Some(0),
                true => None,
                false => Some(index + 1),
            };
            let segment = &spine[index];
            [
                before.map_or([0.0; 4], |before| between(tangent(&spine[before], 1.0), tangent(segment, 0.0))),
                after.map_or([0.0; 4], |after| between(tangent(segment, 1.0), tangent(&spine[after], 0.0))),
            ]
        })
        .collect();
    (closed, cuts)
}

/// How much a profile is scaled along the spine: a curve whose `x` is how
/// far along the spine, as a fraction of its length, and whose `y` is the
/// scale there. Before it starts and after it ends its ends hold. Without
/// one the profile keeps its size.
pub(crate) struct Scaling<'a>(&'a [CubicNurbsSegmentCache]);

impl Scaling<'_> {
    /// The scale `along` the spine, and how fast it changes there: where
    /// two segments of it meet, as it goes on `after` it or as it came.
    fn at(&self, along: f32, after: bool) -> (f32, f32) {
        let (Some(first), Some(last)) = (self.0.first(), self.0.last()) else {
            return (1.0, 0.0);
        };
        let x = |segment: &CubicNurbsSegmentCache, u: f32| segment.evaluate(u).0.x;
        // Only before it starts and after it ends does it not change.
        if along < x(first, 0.0) {
            return (first.evaluate(0.0).0.y, 0.0);
        }
        if along > x(last, 1.0) {
            return (last.evaluate(1.0).0.y, 0.0);
        }
        let segment = match after {
            true => self.0.iter().rev().find(|segment| x(segment, 0.0) <= along).unwrap_or(first),
            false => self.0.iter().find(|segment| along <= x(segment, 1.0)).unwrap_or(last),
        };
        // Right at an end of the segment, or else `x` grows along it, so
        // halving finds where it is `along`.
        let u = if along == x(segment, 0.0) {
            0.0
        } else if along == x(segment, 1.0) {
            1.0
        } else {
            let (mut low, mut high) = (0.0f32, 1.0f32);
            for _ in 0..32 {
                let middle = (low + high) * 0.5;
                match x(segment, middle) < along {
                    true => low = middle,
                    false => high = middle,
                }
            }
            (low + high) * 0.5
        };
        let (point, velocity, _) = segment.evaluate(u);
        let slope = match velocity.x.abs() > 1e-12 {
            true => velocity.y / velocity.x,
            false => 0.0,
        };
        (point.y, slope)
    }

    /// Where along the spine its segments meet, where the scale can turn.
    fn joints(&self) -> impl Iterator<Item = f32> + '_ {
        self.0.iter().skip(1).map(|segment| segment.evaluate(0.0).0.x)
    }

    /// The scale along `span` of `segment`, which lies `along` of a spine
    /// `total` long, as `a + b u + c u² + d u³` in `u` of the piece: right
    /// at both ends, and turning as it does there.
    fn cubic(&self, segment: &CubicNurbsSegmentCache, span: [f32; 2], along: [f32; 2], total: f32) -> [f32; 4] {
        if self.0.is_empty() {
            return [1.0, 0.0, 0.0, 0.0];
        }
        // How fast the spine runs through the fractions per `u` of the piece.
        let rate = |t: f32| segment.evaluate(t).1.length() * (span[1] - span[0]) / total;
        let ((s0, slope0), (s1, slope1)) = (self.at(along[0], true), self.at(along[1], false));
        let (m0, m1) = (slope0 * rate(span[0]), slope1 * rate(span[1]));
        [s0, m0, 3.0 * (s1 - s0) - 2.0 * m0 - m1, 2.0 * (s0 - s1) + m0 + m1]
    }
}

/// The patches of one curve of the spine: every piece of the profile along
/// every segment, then the caps at its start and at its end if it is
/// `capped`. A closed curve has caps which are not there, so that every
/// curve has as many, and so does an end the profile is scaled to nothing
/// at.
///
/// A segment is cut where the scale has a joint, so it can turn there, into
/// `slots` pieces at most: every segment gets as many, and those it does not
/// need are not there. So where a patch is follows from its index alone,
/// which is what the GPU needs.
pub(crate) fn sweep_curve<P: Profile>(
    spine: &[CubicNurbsSegmentCache],
    profile: &[P],
    capped: bool,
    scale: &[CubicNurbsSegmentCache],
    slots: usize,
) -> Vec<P::Patch> {
    let scaling = Scaling(scale);
    let pieces = P::pieces(profile.len());
    let (closed, cuts) = miters(spine);
    let total = spine.last().map_or(0.0, |last| last.cumulative_length()).max(f32::MIN_POSITIVE);
    let mut patches = Vec::with_capacity(spine.len() * pieces + 2 * pieces);
    for ((segment, along), cuts) in spine.iter().zip(fractions(spine)).zip(&cuts) {
        let before = segment.cumulative_length() - segment.length();
        // Where it is cut, as `u` and as how far along the spine: right at
        // the joint of the scale, which the scale turns at.
        let mut cuts_at = vec![(0.0, along[0])];
        cuts_at.extend(
            scaling
                .joints()
                .filter(|joint| *joint > along[0] + 1e-6 && *joint < along[1] - 1e-6)
                .map(|joint| (segment.u_at_distance(joint * total - before), joint)),
        );
        cuts_at.push((1.0, along[1]));
        let last = cuts_at.len() - 2;
        let empty = Piece {
            scale: [0.0; 4],
            ..Piece::whole(*segment, [along[1]; 2], kind::NONE, [[0.0; 4]; 2])
        };
        let spans: Vec<_> = cuts_at.windows(2).map(|cut| ([cut[0].0, cut[1].0], [cut[0].1, cut[1].1])).collect();
        for (index, (span, along)) in spans.iter().copied().enumerate() {
            let piece = Piece {
                segment: *segment,
                span,
                along,
                kind: kind::SIDE,
                // Only where the segment itself ends can it be at a corner.
                miters: [if index == 0 { cuts[0] } else { [0.0; 4] }, if index == last { cuts[1] } else { [0.0; 4] }],
                scale: scaling.cubic(segment, span, along, total),
            };
            for at in 0..pieces {
                patches.push(P::patch(&piece, profile, at));
            }
        }
        for _ in spans.len()..slots {
            for at in 0..pieces {
                patches.push(P::patch(&empty, profile, at));
            }
        }
    }
    if capped {
        for (end, kind) in [(spine.first(), kind::START), (spine.last(), kind::END)] {
            for piece in 0..pieces {
                patches.push(match end {
                    Some(segment) => {
                        let along = if kind == kind::END { [1.0; 2] } else { [0.0; 2] };
                        let size = scaling.at(along[0], kind == kind::START).0;
                        let kind = if closed || size == 0.0 { kind::NONE } else { kind };
                        let cap = Piece {
                            scale: [size, 0.0, 0.0, 0.0],
                            ..Piece::whole(*segment, along, kind, [[0.0; 4]; 2])
                        };
                        P::patch(&cap, profile, piece)
                    }
                    None => P::Patch::zeroed(),
                });
            }
        }
    }
    patches
}

generic_node! {
    /// The surface a profile leaves behind when it is moved along a curve.
    ///
    /// The profile stays upright to the curve and twists as little as it can.
    /// It has to run counter-clockwise for the surface to face outwards. Where
    /// the curve has a corner, the surface is cut on the plane between both
    /// sides of it, so they meet without a gap.
    pub static SWEEP = "curve.sweep" {
        label: "Sweep",
        category: "Curve",
        inputs {
            /// The curves to move the profile along, a surface for each.
            port GenericPort::exact::<CubicNurbsSegmentCache>("spine").grouped();
            /// The cross-section, in the plane of `x` and `y`: `x` points along the
            /// normal of the curve, `y` along its binormal.
            port GenericPort::implementing("profile", PROFILE);
            /// Whether the ends of a curve are closed by flat caps, or left open. A
            /// curve which ends where it starts has no ends. A cap is flat across
            /// from the middle of the profile, which has to see all of it.
            port GenericPort::exact::<u32>("ends").choices(&["capped", "open"]);
            /// How big the profile is along the curve: a curve whose `x` is how far
            /// along, from `0` at its start to `1` at its end, and whose `y` is the
            /// scale there. From `(0, 1)` to `(1, 0)` it ends in a point. A curve
            /// of the spine takes the scale with its index, or the last one. None
            /// keeps the profile as it is.
            port GenericPort::exact::<CubicNurbsSegmentCache>("scale").grouped();
        }
        outputs {
            /// The swept surfaces, one per curve of the spine.
            port GenericPort::implementing("surface", SURFACE).grouped();
        }
    }
}

/// The sweep of `profile` along `spine`, with every other input of
/// [`SWEEP`] at its default. What a program written by hand calls.
pub fn swept(
    builder: &mut ProgramBuilder,
    registry: &NodeRegistry,
    spine: SlotId,
    profile: SlotId,
) -> Result<SlotId, BuildError> {
    Ok(crate::with_defaults(builder, registry, &SWEEP, &[spine, profile])?[0])
}

struct Ports<P>(PhantomData<P>);

impl<P: Profile> Ports<P> {
    const INPUTS: &'static [PortDecl] = &[
        PortDecl::new::<CubicNurbsSegmentCache>("spine").grouped(),
        PortDecl::new::<P>("profile"),
        PortDecl::new::<u32>("ends").choices(&["capped", "open"]),
        PortDecl::new::<CubicNurbsSegmentCache>("scale").grouped(),
    ];
    const OUTPUTS: &'static [PortDecl] = &[PortDecl::new::<P::Patch>("surface").grouped()];
    const GPU: &'static [GpuKernel] = &[GpuKernel {
        wgsl: P::SWEEP_WGSL,
        outputs: &[
            GpuOutput {
                entry: "sweep_patch",
                len: |inputs| {
                    let pieces = P::pieces(inputs.len(1));
                    inputs.len(0) * slots(inputs) * pieces + inputs.group_count(0) * 2 * caps::<P>(inputs)
                },
                words: None,
            },
            GpuOutput {
                entry: "sweep_offset",
                len: |inputs| inputs.group_count(0) + 1,
                // From where the curves of the spine start, which the CPU
                // has unless the spine came of a kernel which did not say.
                words: Some(|inputs| {
                    let (pieces, caps) = (P::pieces(inputs.len(1)) * slots(inputs), caps::<P>(inputs));
                    let mut offsets = vec![0];
                    for curve in inputs.groups(0) {
                        let patches = (curve.len() * pieces + 2 * caps) as u32;
                        offsets.push(offsets.last().unwrap() + patches);
                    }
                    offsets
                }),
            },
        ],
        requires: Caps::NONE,
        // Whether there are caps, where the curves of the spine start, and
        // how long the curves of the scale are.
        reads: &[2, 4, 5],
        accepts: None,
        examples: P::SWEEP_EXAMPLES,
        stages: &[],
    }];
}

/// Into how many pieces a segment of the spine can be cut at most: as many
/// as the longest curve of the scale has segments.
fn slots(inputs: &Inputs<'_>) -> usize {
    match inputs.len(3) {
        0 => 1,
        _ => inputs.groups(3).iter().map(|group| group.len()).max().unwrap_or(1).max(1),
    }
}

/// How many patches each cap of a curve has.
fn caps<P: Profile>(inputs: &Inputs<'_>) -> usize {
    match inputs.choice(2) {
        0 => P::pieces(inputs.len(1)),
        _ => 0,
    }
}

/// Makes `P` usable wherever a profile is asked for, by registering the
/// version of every node that takes one for it. The surface it sweeps out
/// is registered along with it.
pub fn register_profile<P: Profile>(registry: &mut NodeRegistry) {
    let node = version(&SWEEP, P::NAME, |id| NodeDef {
        id,
        inputs: Ports::<P>::INPUTS,
        outputs: Ports::<P>::OUTPUTS,
        version: 2,
        cpu: sweep::<P>,
        gpu: Ports::<P>::GPU,
    });
    registry.register_instance(&SWEEP, node);
    register_surface::<P::Patch>(registry);
}

/// Every curve of the spine is swept on its own, into a group of patches.
fn sweep<P: Profile>(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let spine = inputs.get::<CubicNurbsSegmentCache>(0);
    let profile = inputs.get::<P>(1);
    let capped = inputs.choice(2) == 0;
    let scale = inputs.get::<CubicNurbsSegmentCache>(3);
    let scales = inputs.groups(3);
    let slots = slots(inputs);
    let (patches, offsets) = crate::parallel::per_group(inputs.groups(0), |group, curve| {
        let scale = match scale.is_empty() {
            true => &[][..],
            false => &scale[scales[group.min(scales.len() - 1)].clone()],
        };
        sweep_curve(&spine[curve], P::select(profile, group), capped, scale, slots)
    });
    outputs.set(0, &patches);
    outputs.set_groups(0, &offsets);
}

/// A tube is not a node of its own but this graph: a circle swept along
/// the `spine`, and a mesh of it for whoever needs triangles. Its
/// parameters are `spine`, `radius` and the `tolerance` of the mesh, its
/// outputs the `surface` and the ones of [`MESH`](surface::MESH).
pub fn tube(registry: &NodeRegistry) -> Program {
    let mut b = ProgramBuilder::new("tube");
    let spine = b.param::<CubicNurbsSegmentCache>("spine", &[]);
    let radius = b.param::<f32>("radius", &[0.2]);
    let tolerance = b.param_one_of::<f32>("tolerance", surface::TOLERANCE);

    let circle = b.node(&CIRCLE, &[radius]).expect("a radius is a f32")[0];
    let swept = swept(&mut b, registry, spine, circle).expect("circles are registered as profiles");
    let mesh = surface::mesh(&mut b, registry, swept, tolerance)
        .expect("tubes are registered as surfaces");
    b.output("surface", swept);
    for (name, slot) in mesh {
        b.output(name, slot);
    }
    b.finish()
}

#[cfg(test)]
mod tests {
    use glam::Vec4;
    use oneiroi_core::{
        exec::{Engine, Instance},
        program::BuildError,
    };

    use oneiroi_core::types::DataType;

    use super::*;
    use crate::curve::{NURBS, nurbs::clamped_uniform_knots};

    const BENT: [[f32; 4]; 5] = [
        [0., 0., 0., 1.],
        [1., 2., 0., 1.],
        [2., 0., 1., 1.],
        [3., 2., 0., 1.],
        [4., 0., -1., 1.],
    ];

    fn registry() -> NodeRegistry {
        let mut registry = NodeRegistry::new();
        crate::register(&mut registry);
        registry
    }

    /// A spine with the tube graph called on it.
    fn tube_on(spine: &[[f32; 4]]) -> (Engine, Instance) {
        let mut engine = Engine::default();
        let tube = engine.add_program(tube(&registry()));

        let mut b = ProgramBuilder::new("test");
        let points = b.constant(spine);
        let knots = b.constant(&clamped_uniform_knots(spine.len()));
        let radius = b.param::<f32>("radius", &[0.25]);
        let tolerance = b.param_one_of::<f32>("tolerance", 0.01);
        let segments = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let outputs = b
            .call(
                engine.programs(),
                tube,
                &[Some(segments), Some(radius), Some(tolerance)],
            )
            .unwrap();
        let names = engine.programs().get(tube).outputs.iter().map(|o| o.name.clone());
        for (name, slot) in names.collect::<Vec<_>>().into_iter().zip(outputs) {
            b.output(name, slot);
        }

        let id = engine.add_program(b.finish());
        let mut instance = engine.instantiate(id);
        engine.evaluate([&mut instance]);
        (engine, instance)
    }

    /// A square with sharp corners as a NURBS curve, counter-clockwise.
    fn square() -> (Vec<[f32; 4]>, Vec<f32>) {
        // Small enough to fit through the bends of the spine.
        let corners = [(0.2, -0.2), (0.2, 0.2), (-0.2, 0.2), (-0.2, -0.2), (0.2, -0.2)];
        let mut points = vec![[0.2, -0.2, 0., 1.]];
        for pair in corners.windows(2) {
            let (a, b) = (Vec2::from(pair[0]), Vec2::from(pair[1]));
            for step in 1..=3 {
                let point = a.lerp(b, step as f32 / 3.0);
                points.push([point.x, point.y, 0., 1.]);
            }
        }
        let mut knots = vec![0.0; 4];
        for knot in [0.25, 0.5, 0.75] {
            knots.extend([knot; 3]);
        }
        knots.extend([1.0; 4]);
        (points, knots)
    }

    fn assert_faces_outwards(engine: &Engine, instance: &Instance) {
        let positions = engine.read::<Vec3>(instance, "positions").unwrap();
        let normals = engine.read::<Vec3>(instance, "normals").unwrap();
        let indices = engine.read::<u32>(instance, "indices").unwrap();
        assert!(!indices.is_empty());

        for triangle in indices.chunks_exact(3) {
            let [a, b, c] = [0, 1, 2].map(|i| triangle[i] as usize);
            let face = (positions[b] - positions[a]).cross(positions[c] - positions[a]);
            // Where a cap meets in its middle, triangles have no area.
            if face.length() > 1e-9 {
                assert!(face.dot(normals[a] + normals[b] + normals[c]) > 0.0);
            }
        }
    }

    /// How fine the mesh of the tube is.
    fn tube_detail(engine: &Engine, instance: &Instance) -> (usize, usize) {
        let patches = engine.read::<TubePatch>(instance, "surface").unwrap();
        let tolerance = engine.read::<f32>(instance, "tolerance");
        surface::detail(patches, tolerance.map_or(0.01, |t| t[0]))
    }

    #[test]
    fn a_tube_is_a_grid_per_segment() {
        let (engine, instance) = tube_on(&BENT);
        let positions = engine.read::<Vec3>(&instance, "positions").unwrap();
        let normals = engine.read::<Vec3>(&instance, "normals").unwrap();
        let uvs = engine.read::<Vec2>(&instance, "uvs").unwrap();
        let indices = engine.read::<u32>(&instance, "indices").unwrap();

        // 2 segments and 2 caps, each with rings + 1 rings of sides + 1
        // vertices.
        let (rings, sides) = tube_detail(&engine, &instance);
        let ring = sides + 1;
        assert!(rings > 1 && sides > 8, "{rings} {sides}");
        assert_eq!(positions.len(), 4 * (rings + 1) * ring);
        assert_eq!(indices.len(), 4 * rings * sides * 6);
        assert!(indices.iter().all(|i| (*i as usize) < positions.len()));

        // The seam closes and the texture runs once along and around.
        for ring in positions.chunks_exact(ring) {
            assert!(ring[0].distance(ring[sides]) < 1e-4);
        }
        assert_eq!(uvs[0], Vec2::ZERO);
        assert!(uvs[uvs.len() - 1].distance(Vec2::ONE) < 1e-4);

        // Where the two segments meet they agree on everything, so the
        // tube is smooth there although they share no vertices.
        for side in 0..ring {
            let (end, start) = (rings * ring + side, (rings + 1) * ring + side);
            assert!(positions[end].distance(positions[start]) < 1e-4);
            assert!(normals[end].distance(normals[start]) < 1e-3);
            assert!(uvs[end].distance(uvs[start]) < 1e-4);
        }

        assert_faces_outwards(&engine, &instance);
    }

    #[test]
    fn the_mesh_stays_within_the_tolerance() {
        for tolerance in [0.05, 0.01, 0.002] {
            let (mut engine, mut instance) = tube_on(&BENT);
            engine.set_param(&mut instance, "tolerance", &[tolerance]).unwrap();
            engine.evaluate([&mut instance]);
            let patches = engine.read::<TubePatch>(&instance, "surface").unwrap();
            let (rings, sides) = surface::detail(patches, tolerance);
            let positions = engine.read::<Vec3>(&instance, "positions").unwrap();

            // The middle of every quad of the mesh against the surface.
            let mut worst = 0.0f32;
            for (index, patch) in patches.iter().enumerate() {
                let grid = &positions[index * (rings + 1) * (sides + 1)..];
                for ring in 0..rings {
                    for side in 0..sides {
                        let at = |r: usize, s: usize| grid[(ring + r) * (sides + 1) + side + s];
                        // Both triangles share the diagonal between these.
                        let diagonal = (at(0, 1) + at(1, 0)) * 0.5;
                        let u = (ring as f32 + 0.5) / rings as f32;
                        let v = (side as f32 + 0.5) / sides as f32;
                        worst = worst.max(diagonal.distance(patch.point(u, v).0));
                    }
                }
            }
            assert!(worst <= tolerance, "{worst} is further than {tolerance}");
        }
    }

    #[test]
    fn a_tube_keeps_its_radius() {
        let line: Vec<_> = (0..5).map(|i| [i as f32, 0., 0., 1.]).collect();
        let (engine, instance) = tube_on(&line);
        for position in engine.read::<Vec3>(&instance, "positions").unwrap() {
            let on_axis = Vec4::new(position.x, 0., 0., 1.).truncate();
            let off = position.distance(on_axis);
            // On the side, or on a cap at either end, inside it.
            let capped = (position.x.abs() < 1e-4 || (position.x - 4.0).abs() < 1e-4) && off <= 0.25 + 1e-4;
            assert!((off - 0.25).abs() < 1e-4 || capped, "{position}");
        }
    }

    #[test]
    fn detail_only_meshes_again() {
        let (mut engine, mut instance) = tube_on(&BENT);
        // A new circle, a new surface and a new mesh, but the same curve.
        engine.set_param(&mut instance, "radius", &[0.5f32]).unwrap();
        assert_eq!(engine.evaluate([&mut instance]), 3);

        // The surface is what it was, only its mesh gets finer.
        let coarse = engine.read::<Vec3>(&instance, "positions").unwrap().len();
        engine.set_param(&mut instance, "tolerance", &[0.001f32]).unwrap();
        assert_eq!(engine.evaluate([&mut instance]), 1);
        assert!(engine.read::<Vec3>(&instance, "positions").unwrap().len() > coarse);
    }

    #[test]
    fn a_surface_is_small() {
        let registry = registry();
        let mut b = ProgramBuilder::new("surface");
        let points = b.constant(&BENT);
        let knots = b.constant(&clamped_uniform_knots(5));
        let radius = b.constant::<f32>(&[0.25]);
        let spine = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let circle = b.node(&CIRCLE, &[radius]).unwrap()[0];
        let surface = swept(&mut b, &registry, spine, circle).unwrap();
        b.output("surface", surface);

        let mut engine = Engine::default();
        let id = engine.add_program(b.finish());
        let mut instance = engine.instantiate(id);
        engine.evaluate([&mut instance]);

        // One patch per segment of the curve and one per cap, whatever it
        // is drawn with.
        let patches = engine.read::<TubePatch>(&instance, "surface").unwrap();
        assert_eq!(patches.len(), 4);
        assert_eq!(size_of::<TubePatch>(), 42 * 4);
        let (position, normal) = patches[0].point(0.0, 0.25);
        assert!(position.distance(Vec3::ZERO) - 0.25 < 1e-4);
        assert!((normal.length() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn a_curve_is_a_profile_too() {
        let registry = registry();
        let (square_points, square_knots) = square();

        let mut b = ProgramBuilder::new("beam");
        let points = b.constant(&BENT);
        let knots = b.constant(&clamped_uniform_knots(5));
        let spine = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let points = b.constant(&square_points);
        let knots = b.constant(&square_knots);
        let profile = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let surface = swept(&mut b, &registry, spine, profile).unwrap();
        let tolerance = b.constant::<f32>(&[0.01]);
        b.output("surface", surface);
        for (name, slot) in surface::mesh(&mut b, &registry, surface, tolerance).unwrap() {
            b.output(name, slot);
        }

        let mut engine = Engine::default();
        let id = engine.add_program(b.finish());
        let mut instance = engine.instantiate(id);
        engine.evaluate([&mut instance]);

        // A patch for each of the 4 edges along each of the 2 segments,
        // and for each of them in both caps.
        assert_eq!(engine.read::<SweptPatch>(&instance, "surface").unwrap().len(), 16);
        let patches = engine.read::<SweptPatch>(&instance, "surface").unwrap();
        let (rings, sides) = surface::detail(patches, 0.01);
        let positions = engine.read::<Vec3>(&instance, "positions").unwrap();
        let normals = engine.read::<Vec3>(&instance, "normals").unwrap();
        // The edges are straight, so across them a single step is enough.
        assert_eq!(sides, 1);
        let per = (rings + 1) * (sides + 1);
        assert_eq!(positions.len(), 16 * per);
        assert_eq!(engine.read::<u32>(&instance, "indices").unwrap().len(), 16 * rings * 6);

        // A corner is one position with the normals of both edges: the
        // end of the first edge and the start of the second one.
        assert!(positions[sides].distance(positions[per]) < 1e-4);
        assert!(normals[sides].dot(normals[per]).abs() < 1e-3);
        // Along an edge the normal does not change.
        assert!(normals[0].distance(normals[sides]) < 1e-3);

        assert_faces_outwards(&engine, &instance);
    }

    #[test]
    fn every_curve_is_swept_on_its_own() {
        // Two curves in one list of points, told apart by their groups:
        // a straight one along x, and the bent one far away from it.
        let mut points: Vec<[f32; 4]> = (0..4).map(|i| [i as f32, 0., 0., 1.]).collect();
        points.extend(BENT.iter().map(|[x, y, z, w]| [*x, y + 10.0, *z, *w]));

        let mut b = ProgramBuilder::new("two curves");
        let points = b.param_grouped("points", <[f32; 4]>::INFO, bytemuck::cast_slice(&points));
        let knots = b.constant::<f32>(&[]);
        let spine = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let radius = b.constant::<f32>(&[0.25, 0.5]);
        let circle = b.node(&CIRCLE, &[radius]).unwrap()[0];
        let surface = swept(&mut b, &registry(), spine, circle).unwrap();
        b.output("spine", spine);
        b.output("surface", surface);

        let mut engine = Engine::default();
        let id = engine.add_program(b.finish());
        let mut instance = engine.instantiate(id);
        engine.set_param(&mut instance, "points#groups", &[0u32, 4, 9]).unwrap();
        engine.evaluate([&mut instance]);

        // A segment for the first curve and two for the second, each
        // curve a group of its own all the way through, with its caps.
        assert_eq!(engine.groups(&instance, "spine"), Some(&[0, 1, 3][..]));
        assert_eq!(engine.groups(&instance, "surface"), Some(&[0, 3, 7][..]));
        let patches = engine.read::<TubePatch>(&instance, "surface").unwrap();
        assert_eq!(patches.len(), 7);
        let kinds: Vec<_> = patches.iter().map(TubePatch::kind).collect();
        assert_eq!(kinds, [kind::SIDE, kind::START, kind::END, kind::SIDE, kind::SIDE, kind::START, kind::END]);

        // Each curve gets its own circle, and the texture runs along each
        // curve from its start to its end.
        let radius = |patch: &TubePatch| {
            let (position, _) = patch.point(0.5, 0.0);
            position.distance(patch.point(0.5, 0.5).0) / 2.0
        };
        assert!((radius(&patches[0]) - 0.25).abs() < 1e-3);
        assert!((radius(&patches[3]) - 0.5).abs() < 1e-3);
        assert_eq!(patches[0].uv(1.0, 0.0).y, 1.0);
        assert_eq!(patches[3].uv(0.0, 0.0).y, 0.0);
        assert!((patches[4].uv(1.0, 0.0).y - 1.0).abs() < 1e-5);
    }

    /// Where two patches along a curve meet, the end of the one and the
    /// start of the next are the same points all around.
    fn assert_meets<S: Surface>(ending: &S, starting: &S) {
        for step in 0..=24 {
            let v = step as f32 / 24.0;
            let (a, b) = (ending.point(1.0, v).0, starting.point(0.0, v).0);
            assert!(a.distance(b) < 1e-4, "{a} and {b} at {v}");
        }
    }

    #[test]
    fn corners_meet_without_a_gap() {
        use crate::{curve::polyline, primitive::Circle};
        let corner = [Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0), Vec3::new(2.0, 1.5, 0.5), Vec3::new(0.0, 2.0, 0.0)];
        let open = polyline(&corner, false);
        let (square_points, square_knots) = square();
        let points: Vec<_> = square_points.iter().map(|p| Vec4::from_array(*p)).collect();
        let square = crate::curve::nurbs::extract_segments(&points, &square_knots);

        // A tube and a bar around two corners: each side of a corner reaches
        // the plane between both, so they meet all around.
        let tubes = sweep_curve(&open, &[Circle::new(0.2)], false, &[], 1);
        assert_eq!(tubes.len(), 3);
        for pair in tubes.windows(2) {
            assert_meets(&pair[0], &pair[1]);
        }
        let bars = sweep_curve(&open, &square, false, &[], 1);
        assert_eq!(bars.len(), 3 * 4);
        for (segment, edge) in (0..2).flat_map(|segment| (0..4).map(move |edge| (segment, edge))) {
            assert_meets(&bars[segment * 4 + edge], &bars[(segment + 1) * 4 + edge]);
        }

        // A closed curve is cut where it ends as well, and its caps are not
        // there.
        let ring = polyline(&[Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0), Vec3::Y], true);
        let tubes = sweep_curve(&ring, &[Circle::new(0.1)], true, &[], 1);
        assert_eq!(tubes.len(), 4 + 2);
        assert_meets(&tubes[3], &tubes[0]);
        assert!(tubes[4..].iter().all(|cap| cap.kind() == kind::NONE));
        let point = tubes[4].point(0.5, 0.3).0;
        assert_eq!(point, tubes[4].point(0.9, 0.7).0, "a cap which is not there is a point");
    }

    #[test]
    fn caps_close_the_ends() {
        use crate::{curve::polyline, primitive::Circle};
        let line = polyline(&[Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0)], false);
        let patches = sweep_curve(&line, &[Circle::new(0.5)], true, &[], 1);
        let [side, start, end] = [patches[0], patches[1], patches[2]];
        assert_eq!((start.kind(), end.kind()), (kind::START, kind::END));
        for step in 0..=12 {
            let v = step as f32 / 12.0;
            // From the middle of each end out to where the side is.
            assert!(start.point(0.0, v).0.distance(Vec3::ZERO) < 1e-5);
            assert!(start.point(1.0, v).0.distance(side.point(0.0, v).0) < 1e-5);
            assert!(end.point(1.0, v).0.distance(Vec3::new(3.0, 0.0, 0.0)) < 1e-5);
            assert!(end.point(0.0, v).0.distance(side.point(1.0, v).0) < 1e-5);
            // Facing away from the tube.
            assert!(start.point(0.5, v).1.distance(-Vec3::X) < 1e-5);
            assert!(end.point(0.5, v).1.distance(Vec3::X) < 1e-5);
        }
        assert_eq!(sweep_curve(&line, &[Circle::new(0.5)], false, &[], 1).len(), 1);
    }

    /// A line along `x`, `length` long, swept by a circle of `radius`
    /// scaled along it through `scale` as a polyline.
    fn scaled_tube(length: f32, radius: f32, scale: &[Vec3]) -> Vec<TubePatch> {
        use crate::{curve::polyline, primitive::Circle};
        let line = polyline(&[Vec3::ZERO, Vec3::new(length, 0.0, 0.0)], false);
        let scale = polyline(scale, false);
        sweep_curve(&line, &[Circle::new(radius)], true, &scale, scale.len())
    }

    #[test]
    fn a_scale_down_to_nothing_makes_a_spike() {
        let patches = scaled_tube(3.0, 0.5, &[Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 0.0, 0.0)]);
        let [side, start, end] = [patches[0], patches[1], patches[2]];
        assert_eq!((start.kind(), end.kind()), (kind::START, kind::NONE), "nothing to cap at the tip");
        // The normal of a cone leans towards its tip.
        let slope = (0.5f32 / 3.0).atan();
        for (u, v) in [(0.0, 0.0), (0.3, 0.25), (0.7, 0.6), (0.95, 0.9)] {
            let (point, normal) = side.point(u, v);
            let radius = Vec2::new(point.y, point.z).length();
            assert!((radius - 0.5 * (1.0 - point.x / 3.0)).abs() < 1e-3, "{point} at {u}");
            assert!((normal.x - slope.sin()).abs() < 1e-3, "{normal}");
            assert!((normal.length() - 1.0).abs() < 1e-4);
        }
        assert!(side.point(1.0, 0.3).0.distance(Vec3::new(3.0, 0.0, 0.0)) < 1e-4);
    }

    #[test]
    fn a_scale_turns_where_its_curve_has_a_corner() {
        // Full size until 80 % of the way, then down to a fifth.
        let scale = [Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.8, 1.0, 0.0), Vec3::new(1.0, 0.2, 0.0)];
        let patches = scaled_tube(10.0, 1.0, &scale);
        // The segment is cut there, into two patches.
        let kinds: Vec<_> = patches.iter().map(TubePatch::kind).collect();
        assert_eq!(kinds, [kind::SIDE, kind::SIDE, kind::START, kind::END]);
        let radius = |patch: &TubePatch, u: f32| {
            let point = patch.point(u, 0.0).0;
            (point.x, Vec2::new(point.y, point.z).length())
        };
        for u in [0.0, 0.5, 1.0] {
            let (x, r) = radius(&patches[0], u);
            assert!((r - 1.0).abs() < 1e-3 && x <= 8.0 + 1e-3, "{x} {r}");
        }
        for u in [0.0, 0.5, 1.0] {
            let (x, r) = radius(&patches[1], u);
            let expected = 1.0 - 0.8 * (x - 8.0) / 2.0;
            assert!((r - expected).abs() < 1e-3, "{x} {r} {expected}");
        }
        // Both pieces meet, and the cap at the end has the size there.
        assert_meets(&patches[0], &patches[1]);
        assert!((radius(&patches[3], 0.0).1 - 0.2).abs() < 1e-3);
        assert_eq!(patches[0].uv(1.0, 0.0).y, patches[1].uv(0.0, 0.0).y);
    }

    #[test]
    fn only_profiles_can_be_swept() {
        let registry = registry();
        let mut b = ProgramBuilder::new("broken");
        let spine = b.param::<CubicNurbsSegmentCache>("spine", &[]);
        let not_a_profile = b.constant(&[Vec3::ONE]);
        assert_eq!(
            swept(&mut b, &registry, spine, not_a_profile),
            Err(BuildError::Trait {
                target: "curve.sweep".into(),
                port: "profile".into(),
                expected: "profile",
                found: "vec3",
            })
        );
        // And only surfaces can be turned into a mesh.
        let tolerance = b.constant::<f32>(&[0.01]);
        assert!(matches!(
            surface::mesh(&mut b, &registry, spine, tolerance),
            Err(BuildError::Trait { .. })
        ));

        let names = |tr| {
            let mut names: Vec<_> = registry.implementors(tr).iter().map(|ty| ty.name).collect();
            names.sort();
            names
        };
        assert_eq!(names(PROFILE), ["circle", "cubic_nurbs_segment"]);
        assert_eq!(names(SURFACE), ["swept_patch", "tube_patch"]);
    }
}
