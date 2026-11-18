//! Copies of a shape put onto points: turned to match each frame, scaled,
//! and moved there.

use std::marker::PhantomData;

use glam::{Mat3, Vec3};
use oneiroi_core::{
    node::{Caps, GenericPort, GpuKernel, GpuOutput, Inputs, NodeDef, NodeRegistry, Outputs, PortDecl},
    types::{DataType, PortDefault},
};
use rayon::prelude::*;

use crate::{
    curve::{
        EvaluatedFrame, Profile, SweptPatch, TubePatch,
        nurbs::{CubicNurbsSegmentCache, MovingFrame},
    },
    generic::version,
    primitive::Circle,
};

/// The kernel of a version: the curve functions, placing, `body` which
/// defines `Shape` and `placed` for the type, and the entries.
macro_rules! instance_wgsl {
    ($body:literal) => {
        concat!(
            include_str!("curve/curve.wgsl"),
            include_str!("curve/place.wgsl"),
            $body,
            "
fn instance_place(point: u32) -> Placement {
    let frame = in_points(point);
    var scale = 1.0;
    if (in_scale_len() > 0u) {
        scale = in_scale(0u);
    }
    return Placement(mat3x3<f32>(frame.normal, frame.binormal, frame.tangent), scale, frame.position);
}

fn instance_shape(index: u32) -> Shape {
    let per = in_shape_len();
    return placed(in_shape(index % per), instance_place(index / per));
}

fn instance_offset(group: u32) -> u32 {
    let groups = in_shape_groups();
    let points = in_points_len();
    if (group >= points * groups) {
        return points * in_shape_len();
    }
    return group / groups * in_shape_len() + in_shape_start(group % groups);
}
"
        )
    };
}

describe_trait! {
    /// Something which can be put somewhere else: turned, scaled and moved
    /// without changing its shape, like a curve or a surface.
    pub const PLACEABLE = "placeable" { label: "Shape", hue: Lime }
}

/// Where a copy of a shape goes: the shape is scaled, turned and then
/// moved, all about its origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Where x, y and z of the shape go. A rotation without any scale.
    pub rotation: Mat3,
    pub scale: f32,
    pub offset: Vec3,
}

impl Placement {
    /// Onto `frame`: x of the shape along its normal, y along its
    /// binormal and z along its tangent, the way a profile is swept. A
    /// circle put onto the frames along a curve rings the curve.
    pub fn onto(frame: &MovingFrame, scale: f32) -> Self {
        Self {
            rotation: Mat3::from_cols(frame.normal, frame.binormal, frame.tangent),
            scale,
            offset: frame.position,
        }
    }

    pub fn point(&self, point: Vec3) -> Vec3 {
        self.rotation * point * self.scale + self.offset
    }
}

/// A shape which can be put onto points.
pub trait Placeable: DataType + Sync + Send {
    fn placed(&self, place: &Placement) -> Self;

    /// The kernel which puts it onto points, made with `instance_wgsl!` of
    /// `fn placed(value: Shape, place: Placement) -> Shape` and `alias
    /// Shape` for the type.
    const INSTANCE_WGSL: &'static str;

    /// Some of the type to place, in groups, with the offsets of the
    /// groups, to compare both kernels with.
    fn examples() -> Vec<(Vec<Self>, Vec<u32>)>;
}

impl Placeable for CubicNurbsSegmentCache {
    fn placed(&self, place: &Placement) -> Self {
        CubicNurbsSegmentCache::placed(self, place.rotation, place.scale, place.offset)
    }

    const INSTANCE_WGSL: &'static str = instance_wgsl!("
alias Shape = CubicNurbsSegmentCache;

fn placed(value: Shape, place: Placement) -> Shape {
    return curve_placed(value, place);
}
");

    fn examples() -> Vec<(Vec<Self>, Vec<u32>)> {
        vec![
            crate::curve::example_curves(&[6, 9]),
            crate::curve::example_curves(&[5]),
        ]
    }
}

/// The patches of the curves of `points` swept with `profile`, a group
/// per curve.
pub(crate) fn swept_example<P: Profile>(points: &[usize], profile: &[P]) -> (Vec<P::Patch>, Vec<u32>) {
    let (segments, offsets) = crate::curve::example_curves(points);
    let mut patches = Vec::new();
    let mut patch_offsets = vec![0];
    for curve in offsets.windows(2) {
        patches.extend(P::sweep(&segments[curve[0] as usize..curve[1] as usize], profile));
        patch_offsets.push(patches.len() as u32);
    }
    (patches, patch_offsets)
}

impl Placeable for TubePatch {
    fn placed(&self, place: &Placement) -> Self {
        TubePatch::placed(self, place)
    }

    const INSTANCE_WGSL: &'static str = instance_wgsl!("
alias Shape = TubePatch;

fn placed(value: Shape, place: Placement) -> Shape {
    var placed = value;
    placed.segment = curve_placed(value.segment, place);
    placed.radius = value.radius * place.scale;
    return placed;
}
");

    fn examples() -> Vec<(Vec<Self>, Vec<u32>)> {
        vec![swept_example(&[6, 5], &[Circle::new(0.25)])]
    }
}

impl Placeable for SweptPatch {
    fn placed(&self, place: &Placement) -> Self {
        SweptPatch::placed(self, place)
    }

    const INSTANCE_WGSL: &'static str = instance_wgsl!("
alias Shape = SweptPatch;

fn placed(value: Shape, place: Placement) -> Shape {
    var placed = value;
    placed.spine = curve_placed(value.spine, place);
    // The profile lies in the frames of the spine, which turn along: it
    // only grows with the scale.
    let unturned = Placement(mat3x3<f32>(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0)), place.scale, vec3<f32>(0.0));
    placed.profile = curve_placed(value.profile, unturned);
    return placed;
}
");

    fn examples() -> Vec<(Vec<Self>, Vec<u32>)> {
        let (profile, _) = crate::curve::example_curves(&[4]);
        let small: Vec<_> = profile
            .iter()
            .map(|segment| segment.placed(Mat3::IDENTITY, 0.1, Vec3::ZERO))
            .collect();
        vec![swept_example(&[5, 6], &small)]
    }
}

generic_node! {
    /// A copy of a shape on every point: turned to match the frame there,
    /// scaled, and moved onto it.
    ///
    /// The `x` of the shape goes along the normal of the frame, `y` along its
    /// binormal and `z` along its tangent, the way a profile is swept. A
    /// circle put onto the frames along a curve rings the curve.
    pub static INSTANCE = "instance.on_points" {
        label: "Instance on Points",
        category: "Instance",
        inputs {
            /// Where the copies go, like the frames of Sample Curve. Every point
            /// gets a copy of all of the shape.
            port GenericPort::exact::<EvaluatedFrame>("points");
            /// What is copied. Each of its groups stays a group of its own in every
            /// copy.
            port GenericPort::implementing("shape", PLACEABLE).grouped();
            /// How large the copies are, the same for all of them.
            port GenericPort::exact::<f32>("scale").with_default(PortDefault::F32(&[1.0])).one();
        }
        outputs {
            /// The copies, point after point, each with the groups of the shape.
            port GenericPort::implementing("instances", PLACEABLE).grouped();
        }
    }
}

struct Ports<P>(PhantomData<P>);

impl<P: Placeable> Ports<P> {
    const INPUTS: &'static [PortDecl] = &[
        PortDecl::new::<EvaluatedFrame>("points"),
        PortDecl::new::<P>("shape").grouped(),
        PortDecl::new::<f32>("scale").with_default(PortDefault::F32(&[1.0])).one(),
    ];
    const OUTPUTS: &'static [PortDecl] = &[PortDecl::new::<P>("instances").grouped()];
    const GPU: &'static [GpuKernel] = &[GpuKernel {
        wgsl: P::INSTANCE_WGSL,
        outputs: &[
            GpuOutput {
                entry: "instance_shape",
                len: |inputs| inputs.len(0) * inputs.len(1),
                words: None,
            },
            GpuOutput {
                entry: "instance_offset",
                len: |inputs| inputs.len(0) * inputs.group_count(1) + 1,
                words: Some(|inputs| offsets(inputs.len(0), &inputs.groups(1), inputs.len(1))),
            },
        ],
        requires: Caps::NONE,
        // Where the groups of the shape start.
        reads: &[3],
        accepts: None,
        examples: &[|| example::<P>(0, 3), || example::<P>(1, 1), || example::<P>(0, 0)],
        stages: &[],
    }];
}


/// Where every group of the copies starts: a copy of all groups of the
/// shape per point, and where the last one ends.
fn offsets(points: usize, groups: &[std::ops::Range<usize>], per: usize) -> Vec<u32> {
    let mut offsets = Vec::with_capacity(points * groups.len() + 1);
    for point in 0..points {
        offsets.extend(groups.iter().map(|group| (point * per + group.start) as u32));
    }
    offsets.push((points * per) as u32);
    offsets
}

fn instance<P: Placeable>(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let points = inputs.get::<EvaluatedFrame>(0);
    let shape = inputs.get::<P>(1);
    let scale = inputs.get::<f32>(2).first().copied().unwrap_or(1.0);
    let placed: Vec<P> = points
        .par_iter()
        .flat_map_iter(|frame| {
            let place = Placement::onto(&MovingFrame::from(*frame), scale);
            shape.iter().map(move |value| value.placed(&place))
        })
        .collect();
    outputs.set(0, &placed);
    outputs.set_groups(0, &offsets(points.len(), &inputs.groups(1), shape.len()));
}

/// Frames to put examples onto: turned every way, some of them far off.
fn example_frames(count: usize) -> Vec<EvaluatedFrame> {
    (0..count)
        .map(|i| {
            let angle = i as f32 * 0.9 + 0.3;
            let rotation = glam::Quat::from_euler(glam::EulerRot::XYZ, angle, angle * 0.7, -angle * 1.3);
            EvaluatedFrame::from(MovingFrame {
                position: Vec3::new(i as f32 * 3.0, -2.0, 1.0 + i as f32),
                tangent: rotation * Vec3::Z,
                normal: rotation * Vec3::X,
                binormal: rotation * Vec3::Y,
            })
        })
        .collect()
}

/// Example `which` of `P` put onto `points` frames, as words.
fn example<P: Placeable>(which: usize, points: usize) -> Vec<Vec<u32>> {
    let examples = P::examples();
    let (shape, offsets) = &examples[which.min(examples.len() - 1)];
    vec![
        bytemuck::cast_slice(&example_frames(points)).to_vec(),
        bytemuck::cast_slice(shape).to_vec(),
        vec![1.5f32.to_bits()],
        offsets.clone(),
    ]
}

/// Makes `P` something which can be put onto points.
pub fn register_placeable<P: Placeable>(registry: &mut NodeRegistry) {
    let node = version(&INSTANCE, P::NAME, |id| NodeDef {
        id,
        inputs: Ports::<P>::INPUTS,
        outputs: Ports::<P>::OUTPUTS,
        version: 1,
        cpu: instance::<P>,
        gpu: Ports::<P>::GPU,
    });
    registry.register_instance(&INSTANCE, node);
}

pub fn register(registry: &mut NodeRegistry) {
    register_placeable::<CubicNurbsSegmentCache>(registry);
    register_placeable::<TubePatch>(registry);
    register_placeable::<SweptPatch>(registry);
}

#[cfg(test)]
mod tests {
    use oneiroi_core::{exec::Engine, program::ProgramBuilder};

    use super::*;
    use crate::curve::{NURBS, SAMPLE_CURVE, nurbs::clamped_uniform_knots};

    const BENT: [[f32; 4]; 5] = [
        [0., 0., 0., 1.],
        [1., 2., 0., 2.],
        [2., 0., 1., 1.],
        [3., 2., 0., 0.5],
        [4., 0., -1., 1.],
    ];

    fn bent() -> Vec<CubicNurbsSegmentCache> {
        let points: Vec<_> = BENT.iter().map(|p| glam::Vec4::from_array(*p)).collect();
        crate::curve::nurbs::extract_segments(&points, &clamped_uniform_knots(points.len()))
    }

    fn frame() -> MovingFrame {
        MovingFrame::from(example_frames(3)[2])
    }

    /// A placed curve is the curve turned, scaled and moved, frames and
    /// lengths included.
    #[test]
    fn a_placed_curve_is_the_same_curve_somewhere_else() {
        let place = Placement::onto(&frame(), 2.5);
        for segment in bent() {
            let placed = Placeable::placed(&segment, &place);
            assert!((placed.length() - segment.length() * 2.5).abs() < 1e-4);
            for step in 0..=8 {
                let u = step as f32 / 8.0;
                let (before, after) = (segment.frame(u), placed.frame(u));
                assert!(after.position.distance(place.point(before.position)) < 1e-4, "{u}");
                assert!(after.tangent.distance(place.rotation * before.tangent) < 1e-4, "{u}");
                assert!(after.normal.distance(place.rotation * before.normal) < 1e-3, "{u}");
            }
        }
    }

    #[test]
    fn a_frame_puts_x_along_the_normal_and_z_along_the_tangent() {
        let frame = frame();
        let place = Placement::onto(&frame, 1.0);
        assert!(place.point(Vec3::X).distance(frame.position + frame.normal) < 1e-5);
        assert!(place.point(Vec3::Y).distance(frame.position + frame.binormal) < 1e-5);
        assert!(place.point(Vec3::Z).distance(frame.position + frame.tangent) < 1e-5);
    }

    #[test]
    fn tubes_keep_their_radius_relative_to_their_size() {
        let (tubes, _) = swept_example(&[6], &[Circle::new(0.25)]);
        let place = Placement::onto(&frame(), 2.0);
        for tube in &tubes {
            let placed = tube.placed(&place);
            for (u, v) in [(0.0, 0.0), (0.5, 0.3), (1.0, 0.75)] {
                use crate::surface::Surface;
                let (before, normal) = tube.point(u, v);
                let (after, placed_normal) = placed.point(u, v);
                assert!(after.distance(place.point(before)) < 1e-3, "{u} {v}");
                assert!(placed_normal.distance(place.rotation * normal) < 1e-3, "{u} {v}");
            }
        }
    }

    /// Curves along a curve: a copy of both curves of the shape per frame,
    /// each a group of its own.
    #[test]
    fn every_point_gets_every_group_of_the_shape() {
        let mut registry = NodeRegistry::new();
        crate::register(&mut registry);

        let mut b = ProgramBuilder::new("instances");
        let spine = b.constant(&BENT);
        let knots = b.constant::<f32>(&[]);
        let spine = b.node(&NURBS, &[spine, knots]).unwrap()[0];
        let count = b.constant(&[4u32]);
        let spacing = b.constant(&[0.0f32]);
        let t = b.constant::<f32>(&[]);
        let frames = b.node(&SAMPLE_CURVE, &[spine, count, spacing, t]).unwrap()[0];
        let (shape, shape_offsets) = crate::curve::example_curves(&[5, 6]);
        let shape_slot = b.param_grouped("shape", CubicNurbsSegmentCache::INFO, &[]);
        let scale = b.constant(&[0.5f32]);
        let placed = b.generic(&registry, &INSTANCE, &[frames, shape_slot, scale]).unwrap()[0];
        b.output("instances", placed);
        b.output("frames", frames);

        let mut engine = Engine::default();
        let id = engine.add_program(b.finish());
        let mut instance = engine.instantiate(id);
        engine.set_param(&mut instance, "shape", &shape).unwrap();
        engine
            .set_param_words(&mut instance, "shape#groups", &shape_offsets)
            .unwrap();
        engine.evaluate([&mut instance]);

        let placed = engine.read::<CubicNurbsSegmentCache>(&instance, "instances").unwrap().to_vec();
        let groups = engine.groups(&instance, "instances").unwrap().to_vec();
        let frames = engine.read::<EvaluatedFrame>(&instance, "frames").unwrap().to_vec();
        assert_eq!(frames.len(), 4);
        assert_eq!(placed.len(), 4 * shape.len());
        let per = shape.len() as u32;
        let first = shape_offsets[1];
        assert_eq!(groups, [0, first, per, per + first, 2 * per, 2 * per + first, 3 * per, 3 * per + first, 4 * per]);

        let place = Placement::onto(&MovingFrame::from(frames[2]), 0.5);
        for (segment, copy) in shape.iter().zip(&placed[2 * shape.len()..]) {
            assert_eq!(*copy, Placeable::placed(segment, &place));
        }
    }
}
