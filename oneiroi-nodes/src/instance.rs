//! Instances of shapes: rows which each put a group of a shape somewhere,
//! turned, scaled and moved. The shape itself is never copied, instances
//! refer to it, see [`TypeInfo::instances`].
//!
//! - [`INSTANCE`] puts a shape onto points,
//! - [`TRANSFORM`] puts it somewhere else, or moves instances further,
//! - [`REALIZE`] makes the copies, for whatever needs the shapes where
//!   they are. A port which takes a shape gets it in front of itself where
//!   instances are linked to it.

use std::marker::PhantomData;

use glam::{Mat3, Quat, Vec3};
use oneiroi_core::{
    node::{Caps, Drawable, GenericPort, GpuKernel, GpuOutput, Inputs, NodeDef, NodeRegistry, Outputs, PortDecl},
    types::{DataType, InstanceRow, InstanceTotal, PortDefault, Similarity, TypeInfo},
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

describe_trait! {
    /// Something which can be put somewhere else without changing it, like a
    /// curve, a surface or a circle, and instances of it.
    pub const PLACEABLE = "placeable" { label: "Shape", hue: Lime }
}

/// Where a copy of a shape goes, as a matrix: the shape is scaled, turned
/// and then moved, all about its origin.
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

    pub fn of(transform: &Similarity) -> Self {
        Self {
            rotation: Mat3::from_quat(transform.rotation()),
            scale: transform.scale,
            offset: transform.offset(),
        }
    }

    pub fn point(&self, point: Vec3) -> Vec3 {
        self.rotation * point * self.scale + self.offset
    }
}

/// The rotation which takes x, y and z to `x`, `y` and `z`, which have to
/// be at right angles. The same as `quat_from_basis` on the GPU, so both
/// come out with the same sign.
pub fn quat_from_basis(x: Vec3, y: Vec3, z: Vec3) -> Quat {
    let trace = x.x + y.y + z.z;
    let q = if trace > 0.0 {
        let s = 0.5 / (trace + 1.0).sqrt();
        Quat::from_xyzw((y.z - z.y) * s, (z.x - x.z) * s, (x.y - y.x) * s, 0.25 / s)
    } else if x.x > y.y && x.x > z.z {
        let s = 2.0 * (1.0 + x.x - y.y - z.z).max(0.0).sqrt();
        Quat::from_xyzw(0.25 * s, (y.x + x.y) / s, (z.x + x.z) / s, (y.z - z.y) / s)
    } else if y.y > z.z {
        let s = 2.0 * (1.0 + y.y - x.x - z.z).max(0.0).sqrt();
        Quat::from_xyzw((y.x + x.y) / s, 0.25 * s, (z.y + y.z) / s, (z.x - x.z) / s)
    } else {
        let s = 2.0 * (1.0 + z.z - x.x - y.y).max(0.0).sqrt();
        Quat::from_xyzw((z.x + x.z) / s, (z.y + y.z) / s, 0.25 * s, (x.y - y.x) / s)
    };
    q.normalize()
}

/// Turning by `degrees` around x, then y, then z, seen from the turned axes.
pub fn quat_from_degrees(degrees: Vec3) -> Quat {
    let radians = degrees * (std::f32::consts::PI / 180.0);
    (Quat::from_rotation_x(radians.x) * Quat::from_rotation_y(radians.y) * Quat::from_rotation_z(radians.z)).normalize()
}

/// Where a shape put onto `frame` goes, see [`Placement::onto`].
pub fn onto_frame(frame: &EvaluatedFrame, scale: f32) -> Similarity {
    let frame = MovingFrame::from(*frame);
    Similarity::new(quat_from_basis(frame.normal, frame.binormal, frame.tangent), scale, frame.position)
}

/// A shape which can be placed by reference.
pub trait Shape: DataType + Sync + Send {
    /// The type of instances of it.
    const INSTANCES: TypeInfo;
    /// What instances of it are called, for people.
    const INSTANCES_LABEL: &'static str;

    /// Some of it, in groups, with the offsets of the groups, to compare
    /// both kernels with.
    fn examples() -> Vec<(Vec<Self>, Vec<u32>)>;
}

/// A shape which can also be copied to where its instances are, see
/// [`REALIZE`].
pub trait Placeable: Shape {
    fn placed(&self, place: &Placement) -> Self;

    /// The kernel of [`REALIZE`] for it, made with `realize_wgsl!` of `fn
    /// placed(value: Shape, place: Placement) -> Shape` and `alias Shape`
    /// for the type.
    const REALIZE_WGSL: &'static str;
}

/// The kernel of [`REALIZE`]: the curve functions, placing, `body` which
/// defines `Shape` and `placed` for the type, and the entries.
macro_rules! realize_wgsl {
    ($body:literal) => {
        concat!(
            include_str!("curve/curve.wgsl"),
            include_str!("curve/place.wgsl"),
            include_str!("similarity.wgsl"),
            $body,
            "
fn placement_of(t: Similarity) -> Placement {
    let rotation = mat3x3<f32>(
        quat_rotate(t.rotation, vec3<f32>(1.0, 0.0, 0.0)),
        quat_rotate(t.rotation, vec3<f32>(0.0, 1.0, 0.0)),
        quat_rotate(t.rotation, vec3<f32>(0.0, 0.0, 1.0)),
    );
    return Placement(rotation, t.scale, t.offset);
}

fn realize_shapes(index: u32) -> Shape {
    let row = in_instances(in_instances_row_of(index));
    let element = in_instances_prototype(in_instances_prototype_start(row.group) + index - row.first);
    return placed(element, placement_of(row.transform));
}

fn realize_offsets(group: u32) -> u32 {
    let rows = in_instances_len();
    if (rows == 0u) {
        return 0u;
    }
    if (group < rows) {
        return in_instances(group).first;
    }
    return in_instances_total();
}
"
        )
    };
}

impl Shape for CubicNurbsSegmentCache {
    const INSTANCES: TypeInfo =
        TypeInfo::instances("instances<cubic_nurbs_segment>", &<CubicNurbsSegmentCache as DataType>::INFO);
    const INSTANCES_LABEL: &'static str = "Curve instances";

    fn examples() -> Vec<(Vec<Self>, Vec<u32>)> {
        vec![crate::curve::example_curves(&[6, 9]), crate::curve::example_curves(&[5])]
    }
}

impl Placeable for CubicNurbsSegmentCache {
    fn placed(&self, place: &Placement) -> Self {
        CubicNurbsSegmentCache::placed(self, place.rotation, place.scale, place.offset)
    }

    const REALIZE_WGSL: &'static str = realize_wgsl!(
        "
alias Shape = CubicNurbsSegmentCache;

fn placed(value: Shape, place: Placement) -> Shape {
    return curve_placed(value, place);
}
"
    );
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

impl Shape for TubePatch {
    const INSTANCES: TypeInfo = TypeInfo::instances("instances<tube_patch>", &<TubePatch as DataType>::INFO);
    const INSTANCES_LABEL: &'static str = "Tube instances";

    fn examples() -> Vec<(Vec<Self>, Vec<u32>)> {
        vec![swept_example(&[6, 5], &[Circle::new(0.25)])]
    }
}

impl Placeable for TubePatch {
    fn placed(&self, place: &Placement) -> Self {
        TubePatch::placed(self, place)
    }

    const REALIZE_WGSL: &'static str = realize_wgsl!(
        "
alias Shape = TubePatch;

fn placed(value: Shape, place: Placement) -> Shape {
    var placed = value;
    placed.segment = curve_placed(value.segment, place);
    placed.radius = value.radius * place.scale;
    return placed;
}
"
    );
}

impl Shape for SweptPatch {
    const INSTANCES: TypeInfo = TypeInfo::instances("instances<swept_patch>", &<SweptPatch as DataType>::INFO);
    const INSTANCES_LABEL: &'static str = "Swept surface instances";

    fn examples() -> Vec<(Vec<Self>, Vec<u32>)> {
        let (profile, _) = crate::curve::example_curves(&[4]);
        let small: Vec<_> = profile
            .iter()
            .map(|segment| segment.placed(Mat3::IDENTITY, 0.1, Vec3::ZERO))
            .collect();
        vec![swept_example(&[5, 6], &small)]
    }
}

impl Placeable for SweptPatch {
    fn placed(&self, place: &Placement) -> Self {
        SweptPatch::placed(self, place)
    }

    const REALIZE_WGSL: &'static str = realize_wgsl!(
        "
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
"
    );
}

/// A circle has no place of its own, it is around the origin in the plane
/// of x and y. Instances of it can be anywhere, but there is nothing to
/// copy them into.
impl Shape for Circle {
    const INSTANCES: TypeInfo = TypeInfo::instances("instances<circle>", &<Circle as DataType>::INFO);
    const INSTANCES_LABEL: &'static str = "Circle instances";

    fn examples() -> Vec<(Vec<Self>, Vec<u32>)> {
        vec![(vec![Circle::new(0.5), Circle::new(1.0), Circle::new(2.0)], vec![0, 1, 3])]
    }
}

generic_node! {
    /// A copy of a shape on every point: turned to match the frame there,
    /// scaled, and moved onto it.
    ///
    /// The `x` of the shape goes along the normal of the frame, `y` along its
    /// binormal and `z` along its tangent, the way a profile is swept. A
    /// circle put onto the frames along a curve rings the curve.
    ///
    /// Nothing of the shape is copied: the copies are instances which refer to
    /// it, however many there are.
    pub static INSTANCE = "instance.on_points" {
        label: "Instance on Points",
        category: "Instance",
        inputs {
            /// Where the copies go, like the frames of Sample Curve. Every point
            /// gets a copy of all of the shape.
            port GenericPort::exact::<EvaluatedFrame>("points");
            /// What is copied: a shape, or instances, which are then put onto
            /// every point together. Each group of the shape is an instance of
            /// its own.
            port GenericPort::implementing("shape", PLACEABLE).grouped();
            /// How large the copies are, the same for all of them.
            port GenericPort::exact::<f32>("scale").with_default(PortDefault::F32(&[1.0])).one();
        }
        outputs {
            /// The copies, a group of them per point.
            port GenericPort::implementing("instances", PLACEABLE).grouped();
        }
    }
}

generic_node! {
    /// Puts a shape somewhere else: scaled, turned and then moved. Given
    /// instances, it does that to all of them, on top of where they are.
    ///
    /// Nothing of the shape is copied, it is only said where it goes.
    pub static TRANSFORM = "instance.transform" {
        label: "Transform",
        category: "Instance",
        inputs {
            /// What is put somewhere else, a shape or instances.
            port GenericPort::implementing("shape", PLACEABLE).grouped();
            /// How far it is moved.
            port GenericPort::exact::<Vec3>("translate").with_default(PortDefault::F32(&[0.0; 3])).one();
            /// How far it is turned, in degrees: around x, then y, then z.
            port GenericPort::exact::<Vec3>("rotate").with_default(PortDefault::F32(&[0.0; 3])).one();
            /// How large it gets, the same in every direction.
            port GenericPort::exact::<f32>("scale").with_default(PortDefault::F32(&[1.0])).one();
        }
        outputs {
            /// Where it is now, an instance per group of the shape.
            port GenericPort::implementing("instances", PLACEABLE).grouped();
        }
    }
}

generic_node! {
    /// Copies of the shape where its instances are, for whatever needs the
    /// shapes themselves.
    ///
    /// This costs work and room for every element of every copy, which
    /// instances do not. It happens on its own where instances are linked to
    /// a port which takes a shape.
    pub static REALIZE = "instance.realize" {
        label: "Realize Instances",
        category: "Instance",
        inputs {
            /// The instances to copy the shape for.
            port GenericPort::implementing("instances", PLACEABLE);
        }
        outputs {
            /// The copies, a group per instance.
            port GenericPort::implementing("shapes", PLACEABLE).grouped();
        }
    }
}

/// Where the frames of examples go: turned every way, some of them far off.
fn example_frames(count: usize) -> Vec<EvaluatedFrame> {
    (0..count)
        .map(|i| {
            let angle = i as f32 * 0.9 + 0.3;
            let rotation = Quat::from_euler(glam::EulerRot::XYZ, angle, angle * 0.7, -angle * 1.3);
            EvaluatedFrame::from(MovingFrame {
                position: Vec3::new(i as f32 * 3.0, -2.0, 1.0 + i as f32),
                tangent: rotation * Vec3::Z,
                normal: rotation * Vec3::X,
                binormal: rotation * Vec3::Y,
            })
        })
        .collect()
}

/// Instances of every group of `offsets`, each somewhere else.
fn example_rows(len: usize, offsets: &[u32]) -> Vec<InstanceRow> {
    let groups = group_ranges(len, offsets);
    let mut first = 0;
    groups
        .iter()
        .enumerate()
        .map(|(group, range)| {
            let angle = group as f32 * 1.1 + 0.2;
            let row = InstanceRow {
                transform: Similarity::new(
                    Quat::from_euler(glam::EulerRot::XYZ, -angle, angle * 0.5, angle),
                    0.5 + group as f32 * 0.25,
                    Vec3::new(group as f32, 1.0, -2.0 * group as f32),
                ),
                group: group as u32,
                first,
            };
            first += range.len() as u32;
            row
        })
        .collect()
}

fn group_ranges(len: usize, offsets: &[u32]) -> Vec<std::ops::Range<usize>> {
    match offsets.is_empty() {
        true => vec![0..len],
        false => offsets
            .windows(2)
            .map(|pair| (pair[0] as usize).min(len)..(pair[1] as usize).min(len))
            .collect(),
    }
}

fn words<T: bytemuck::Pod>(values: &[T]) -> Vec<u32> {
    bytemuck::cast_slice(values).to_vec()
}

/// The words of instances of example `which` of `S`: the rows, their
/// offsets, the shape, its offsets and the total of the instances.
fn instances_example<S: Shape>(which: usize) -> (Vec<u32>, Vec<u32>, Vec<u32>, Vec<u32>, Vec<u32>) {
    let examples = S::examples();
    let (shape, offsets) = &examples[which.min(examples.len() - 1)];
    let rows = example_rows(shape.len(), offsets);
    let total = group_ranges(shape.len(), offsets).iter().map(|range| range.len() as u32).sum();
    (words(&rows), Vec::new(), words(shape), offsets.clone(), InstanceTotal::single(rows.len() as u32, total).words())
}

/// The rows of instances of a shape on every point: one per point and
/// group of the shape.
fn instance<S: Shape>(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let points = inputs.get::<EvaluatedFrame>(0);
    let groups = inputs.groups(1);
    let covered = covered(inputs, 1);
    let starts = group_firsts(&groups);
    let scale = inputs.get::<f32>(2).first().copied().unwrap_or(1.0);
    let rows: Vec<InstanceRow> = points
        .par_iter()
        .enumerate()
        .flat_map_iter(|(point, frame)| {
            let transform = onto_frame(frame, scale);
            starts.iter().enumerate().map(move |(group, start)| InstanceRow {
                transform,
                group: group as u32,
                first: point as u32 * covered + start,
            })
        })
        .collect();
    outputs.set(0, &rows);
    outputs.set_groups(0, &per_point(points.len(), groups_count(inputs)));
    outputs.set_total(0, instance_total(inputs));
}

fn groups_count(inputs: &Inputs<'_>) -> usize {
    inputs.group_count(1)
}

/// How many elements the groups of input `index` cover together: as many
/// as one copy of all of them has.
fn covered(inputs: &Inputs<'_>, index: usize) -> u32 {
    inputs.groups(index).iter().map(|range| range.len() as u32).sum()
}

/// Where the copy of each group starts in a copy of all of them.
fn group_firsts(groups: &[std::ops::Range<usize>]) -> Vec<u32> {
    groups
        .iter()
        .scan(0, |first, range| {
            let start = *first;
            *first += range.len() as u32;
            Some(start)
        })
        .collect()
}

/// A block of instances per point of a shape, each with a row per group.
fn instance_total(inputs: &Inputs<'_>) -> InstanceTotal {
    let groups = inputs.group_count(1) as u32;
    InstanceTotal::repeated(inputs.len(0) as u32, InstanceTotal::single(groups, covered(inputs, 1)), groups)
}

/// A block of the given instances per point.
fn instance_instances_total(inputs: &Inputs<'_>) -> InstanceTotal {
    InstanceTotal::repeated(inputs.len(0) as u32, inputs.total(1), inputs.len(1) as u32)
}

/// Offsets of `points` groups of `each` rows.
fn per_point(points: usize, each: usize) -> Vec<u32> {
    (0..=points).map(|point| (point * each) as u32).collect()
}

/// Instances on every point, all of them together on each.
fn instance_instances<S: Shape>(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let points = inputs.get::<EvaluatedFrame>(0);
    let given = inputs.get::<InstanceRow>(1);
    let scale = inputs.get::<f32>(2).first().copied().unwrap_or(1.0);
    let total = inputs.total(1).elements;
    let rows: Vec<InstanceRow> = points
        .par_iter()
        .enumerate()
        .flat_map_iter(|(point, frame)| {
            let frame = onto_frame(frame, scale);
            given.iter().map(move |row| InstanceRow {
                transform: frame.then(&row.transform),
                group: row.group,
                first: point as u32 * total + row.first,
            })
        })
        .collect();
    outputs.set(0, &rows);
    outputs.set_groups(0, &per_point(points.len(), given.len()));
    outputs.set_total(0, instance_instances_total(inputs));
}

/// The transform [`TRANSFORM`] is given.
fn given_transform(inputs: &Inputs<'_>) -> Similarity {
    let translate = inputs.get::<Vec3>(1).first().copied().unwrap_or(Vec3::ZERO);
    let rotate = inputs.get::<Vec3>(2).first().copied().unwrap_or(Vec3::ZERO);
    let scale = inputs.get::<f32>(3).first().copied().unwrap_or(1.0);
    Similarity::new(quat_from_degrees(rotate), scale, translate)
}

/// An instance of every group of a shape, all of them put the same way.
fn transform<S: Shape>(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let transform = given_transform(inputs);
    let rows: Vec<InstanceRow> = inputs
        .groups(0)
        .iter()
        .enumerate()
        .scan(0, |first, (group, range)| {
            let row = InstanceRow {
                transform,
                group: group as u32,
                first: *first,
            };
            *first += range.len() as u32;
            Some(row)
        })
        .collect();
    outputs.set_total(0, InstanceTotal::single(rows.len() as u32, covered(inputs, 0)));
    outputs.set(0, &rows);
}

/// Instances put further: the transform on top of where they are.
fn transform_instances<S: Shape>(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let transform = given_transform(inputs);
    let rows: Vec<InstanceRow> = inputs
        .get::<InstanceRow>(0)
        .iter()
        .map(|row| InstanceRow {
            transform: transform.then(&row.transform),
            ..*row
        })
        .collect();
    outputs.set(0, &rows);
    outputs.set_total(0, inputs.total(0));
}

/// Every copy of the shape, a group per instance.
fn realize<P: Placeable>(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let rows = inputs.get::<InstanceRow>(0);
    let shape = inputs.prototype::<P>(0);
    let groups = inputs.prototype_groups(0);
    let copies: Vec<Vec<P>> = rows
        .par_iter()
        .map(|row| {
            let place = Placement::of(&row.transform);
            let range = groups.get(row.group as usize).cloned().unwrap_or(0..0);
            shape[range].iter().map(|value| value.placed(&place)).collect()
        })
        .collect();
    let mut offsets = Vec::with_capacity(copies.len() + 1);
    let mut placed = Vec::new();
    for copy in copies {
        offsets.push(placed.len() as u32);
        placed.extend(copy);
    }
    offsets.push(placed.len() as u32);
    outputs.set(0, &placed);
    outputs.set_groups(0, &offsets);
}

struct Ports<S>(PhantomData<S>);

impl<S: Shape> Ports<S> {
    const INSTANCE: &'static [PortDecl] = &[
        PortDecl::new::<EvaluatedFrame>("points"),
        PortDecl::new::<S>("shape").grouped(),
        PortDecl::new::<f32>("scale").with_default(PortDefault::F32(&[1.0])).one(),
    ];
    const INSTANCE_INSTANCES: &'static [PortDecl] = &[
        PortDecl::new::<EvaluatedFrame>("points"),
        PortDecl::of("shape", S::INSTANCES).grouped(),
        PortDecl::new::<f32>("scale").with_default(PortDefault::F32(&[1.0])).one(),
    ];
    const INSTANCE_OUT: &'static [PortDecl] = &[PortDecl::of("instances", S::INSTANCES).grouped().instances_of(1)];

    const TRANSFORM: &'static [PortDecl] = &[
        PortDecl::new::<S>("shape").grouped(),
        PortDecl::new::<Vec3>("translate").with_default(PortDefault::F32(&[0.0; 3])).one(),
        PortDecl::new::<Vec3>("rotate").with_default(PortDefault::F32(&[0.0; 3])).one(),
        PortDecl::new::<f32>("scale").with_default(PortDefault::F32(&[1.0])).one(),
    ];
    const TRANSFORM_INSTANCES: &'static [PortDecl] = &[
        PortDecl::of("shape", S::INSTANCES).grouped(),
        PortDecl::new::<Vec3>("translate").with_default(PortDefault::F32(&[0.0; 3])).one(),
        PortDecl::new::<Vec3>("rotate").with_default(PortDefault::F32(&[0.0; 3])).one(),
        PortDecl::new::<f32>("scale").with_default(PortDefault::F32(&[1.0])).one(),
    ];
    const TRANSFORM_OUT: &'static [PortDecl] = &[PortDecl::of("instances", S::INSTANCES).grouped().instances_of(0)];
    const TRANSFORM_INSTANCES_OUT: &'static [PortDecl] =
        &[PortDecl::of("instances", S::INSTANCES).grouped_like(0).instances_of(0)];

    const INSTANCE_GPU: &'static [GpuKernel] = &[GpuKernel {
        wgsl: concat!(
            include_str!("similarity.wgsl"),
            include_str!("instance/frame.wgsl"),
            "
fn instance_rows(index: u32) -> InstanceRow {
    let groups = in_shape_groups();
    let point = index / groups;
    let group = index % groups;
    let covered = in_shape_end(groups - 1u) - in_shape_start(0u);
    return InstanceRow(instance_frame(point), group, point * covered + in_shape_start(group) - in_shape_start(0u));
}

fn instance_offsets(point: u32) -> u32 {
    return point * in_shape_groups();
}
"
        ),
        outputs: &[
            GpuOutput {
                entry: "instance_rows",
                len: |inputs| inputs.len(0) * inputs.group_count(1),
                words: None,
            },
            GpuOutput {
                entry: "instance_offsets",
                len: |inputs| inputs.len(0) + 1,
                words: Some(|inputs| per_point(inputs.len(0), inputs.group_count(1))),
            },
            GpuOutput {
                entry: "",
                len: |_| 1,
                words: Some(|inputs| instance_total(inputs).words()),
            },
        ],
        requires: Caps::NONE,
        // The offsets of the shape, for the total.
        reads: &[3],
        accepts: None,
        examples: &[
            || {
                let (shape, offsets) = S::examples().remove(0);
                vec![words(&example_frames(3)), words(&shape), words(&[1.5f32]), offsets]
            },
            || {
                let (shape, _) = S::examples().remove(0);
                vec![words(&example_frames(2)), words(&shape), Vec::new(), Vec::new()]
            },
            || {
                let (shape, offsets) = S::examples().remove(0);
                vec![Vec::new(), words(&shape), Vec::new(), offsets]
            },
        ],
        stages: &[],
    }];

    const INSTANCE_INSTANCES_GPU: &'static [GpuKernel] = &[GpuKernel {
        wgsl: concat!(
            include_str!("similarity.wgsl"),
            include_str!("instance/frame.wgsl"),
            "
fn instance_rows(index: u32) -> InstanceRow {
    let rows = in_shape_len();
    let point = index / rows;
    let row = in_shape(index % rows);
    return InstanceRow(similarity_then(instance_frame(point), row.transform), row.group, point * in_shape_total() + row.first);
}

fn instance_offsets(point: u32) -> u32 {
    return point * in_shape_len();
}
"
        ),
        outputs: &[
            GpuOutput {
                entry: "instance_rows",
                len: |inputs| inputs.len(0) * inputs.len(1),
                words: None,
            },
            GpuOutput {
                entry: "instance_offsets",
                len: |inputs| inputs.len(0) + 1,
                words: Some(|inputs| per_point(inputs.len(0), inputs.len(1))),
            },
            GpuOutput {
                entry: "",
                len: |_| 1,
                words: Some(|inputs| instance_instances_total(inputs).words()),
            },
        ],
        requires: Caps::NONE,
        // The total of the instances which are put onto the points.
        reads: &[6],
        accepts: None,
        examples: &[
            || {
                let (rows, groups, shape, offsets, total) = instances_example::<S>(0);
                vec![words(&example_frames(3)), rows, words(&[2.0f32]), groups, shape, offsets, total]
            },
            || {
                let (rows, groups, shape, offsets, total) = instances_example::<S>(1);
                vec![words(&example_frames(1)), rows, Vec::new(), groups, shape, offsets, total]
            },
        ],
        stages: &[],
    }];

    const TRANSFORM_GPU: &'static [GpuKernel] = &[GpuKernel {
        wgsl: concat!(
            include_str!("similarity.wgsl"),
            include_str!("instance/given.wgsl"),
            "
fn transform_rows(group: u32) -> InstanceRow {
    // Where the groups before this one end, together.
    return InstanceRow(transform_given(), group, in_shape_start(group) - in_shape_start(0u));
}

fn transform_offsets(group: u32) -> u32 {
    return 0u;
}
"
        ),
        outputs: &[
            GpuOutput {
                entry: "transform_rows",
                len: |inputs| inputs.group_count(0),
                words: None,
            },
            // A single group of all instances.
            GpuOutput {
                entry: "transform_offsets",
                len: |_| 0,
                words: Some(|_| Vec::new()),
            },
            GpuOutput {
                entry: "",
                len: |_| 1,
                words: Some(|inputs| InstanceTotal::single(inputs.group_count(0) as u32, covered(inputs, 0)).words()),
            },
        ],
        requires: Caps::NONE,
        // The offsets of the shape, for the total.
        reads: &[4],
        accepts: None,
        examples: &[
            || {
                let (shape, offsets) = S::examples().remove(0);
                let (translate, rotate) = (words(&[Vec3::new(1.0, -2.0, 3.0)]), words(&[Vec3::new(30.0, -45.0, 120.0)]));
                vec![words(&shape), translate, rotate, words(&[0.5f32]), offsets]
            },
            || {
                let (shape, _) = S::examples().remove(0);
                vec![words(&shape), Vec::new(), Vec::new(), Vec::new(), Vec::new()]
            },
        ],
        stages: &[],
    }];

    const TRANSFORM_INSTANCES_GPU: &'static [GpuKernel] = &[GpuKernel {
        wgsl: concat!(
            include_str!("similarity.wgsl"),
            include_str!("instance/given.wgsl"),
            "
fn transform_rows(index: u32) -> InstanceRow {
    let row = in_shape(index);
    return InstanceRow(similarity_then(transform_given(), row.transform), row.group, row.first);
}
"
        ),
        outputs: &[
            GpuOutput {
                entry: "transform_rows",
                len: |inputs| inputs.len(0),
                words: None,
            },
            GpuOutput {
                entry: "",
                len: |_| 1,
                words: Some(|inputs| inputs.total(0).words()),
            },
        ],
        requires: Caps::NONE,
        // The total of the instances which are put further.
        reads: &[7],
        accepts: None,
        examples: &[|| {
            let (rows, groups, shape, offsets, total) = instances_example::<S>(0);
            let (translate, rotate) = (words(&[Vec3::new(-1.0, 0.5, 2.0)]), words(&[Vec3::new(10.0, 200.0, -30.0)]));
            vec![rows, translate, rotate, words(&[3.0f32]), groups, shape, offsets, total]
        }],
        stages: &[],
    }];
}

impl<P: Placeable> Ports<P> {
    const REALIZE: &'static [PortDecl] = &[PortDecl::of("instances", P::INSTANCES)];
    const REALIZE_OUT: &'static [PortDecl] = &[PortDecl::new::<P>("shapes").grouped()];
    const REALIZE_GPU: &'static [GpuKernel] = &[GpuKernel {
        wgsl: P::REALIZE_WGSL,
        outputs: &[
            // As long as all copies together, which the instances say
            // without anybody looking at their rows.
            GpuOutput {
                entry: "realize_shapes",
                len: |inputs| inputs.total(0).elements as usize,
                words: None,
            },
            GpuOutput {
                entry: "realize_offsets",
                len: |inputs| inputs.len(0) + 1,
                words: None,
            },
        ],
        requires: Caps::NONE,
        // The total of the instances.
        reads: &[3],
        accepts: None,
        examples: &[
            || {
                let (rows, _, shape, offsets, total) = instances_example::<P>(0);
                vec![rows, shape, offsets, total]
            },
            || {
                let (_, _, shape, offsets, _) = instances_example::<P>(0);
                vec![Vec::new(), shape, offsets, InstanceTotal::default().words()]
            },
            // A block of rows for each of three points, as Instance on
            // Points makes them: the row of an element is found by dividing.
            || {
                let examples = P::examples();
                let (shape, offsets) = &examples[0];
                let block = example_rows(shape.len(), offsets);
                let elements: u32 = group_ranges(shape.len(), offsets).iter().map(|range| range.len() as u32).sum();
                let rows: Vec<InstanceRow> = (0..3u32)
                    .flat_map(|point| {
                        block.iter().map(move |row| InstanceRow {
                            first: point * elements + row.first,
                            transform: Similarity {
                                offset: [point as f32 * 2.0, 0.0, 0.0],
                                ..row.transform
                            },
                            ..*row
                        })
                    })
                    .collect();
                let total = InstanceTotal::repeated(3, InstanceTotal::single(block.len() as u32, elements), block.len() as u32);
                vec![words(&rows), words(shape), offsets.clone(), total.words()]
            },
        ],
        stages: &[],
    }];
}

/// Makes `S` something which can be placed: by [`INSTANCE`] and
/// [`TRANSFORM`], given itself or instances of it.
pub fn register_shape<S: Shape>(registry: &mut NodeRegistry) {
    let versions = [
        (&INSTANCE, S::NAME, Ports::<S>::INSTANCE, Ports::<S>::INSTANCE_OUT, instance::<S> as fn(&Inputs<'_>, &mut Outputs), Ports::<S>::INSTANCE_GPU),
        (&INSTANCE, S::INSTANCES.name, Ports::<S>::INSTANCE_INSTANCES, Ports::<S>::INSTANCE_OUT, instance_instances::<S>, Ports::<S>::INSTANCE_INSTANCES_GPU),
        (&TRANSFORM, S::NAME, Ports::<S>::TRANSFORM, Ports::<S>::TRANSFORM_OUT, transform::<S>, Ports::<S>::TRANSFORM_GPU),
        (&TRANSFORM, S::INSTANCES.name, Ports::<S>::TRANSFORM_INSTANCES, Ports::<S>::TRANSFORM_INSTANCES_OUT, transform_instances::<S>, Ports::<S>::TRANSFORM_INSTANCES_GPU),
    ];
    for (generic, ty, inputs, outputs, cpu, gpu) in versions {
        let node = version(generic, ty, |id| NodeDef {
            id,
            inputs,
            outputs,
            version: 1,
            cpu,
            gpu,
        });
        registry.register_instance(generic, node);
    }
    // Instances are drawn like the shape, each where it is.
    if let Some(drawable) = registry.drawable(&S::INFO).copied() {
        registry.register_drawable(Drawable {
            ty: S::INSTANCES,
            ..drawable
        });
    }
}

/// Makes `P` something which can be placed, and its instances something
/// which can be copied back into it.
pub fn register_placeable<P: Placeable>(registry: &mut NodeRegistry) {
    register_shape::<P>(registry);
    let node = version(&REALIZE, P::INSTANCES.name, |id| NodeDef {
        id,
        inputs: Ports::<P>::REALIZE,
        outputs: Ports::<P>::REALIZE_OUT,
        version: 1,
        cpu: realize::<P>,
        gpu: Ports::<P>::REALIZE_GPU,
    });
    registry.register_instance(&REALIZE, node);
}

pub fn register(registry: &mut NodeRegistry) {
    register_placeable::<CubicNurbsSegmentCache>(registry);
    register_placeable::<TubePatch>(registry);
    register_placeable::<SweptPatch>(registry);
    register_shape::<Circle>(registry);
    registry.set_realizer(&REALIZE);
}

/// What people see of the instances of every shape.
#[cfg(feature = "meta")]
pub fn describe(meta: &mut crate::meta::MetaRegistry) {
    use crate::Described;
    fn of<S: Shape + Described>(meta: &mut crate::meta::MetaRegistry) {
        meta.add_type(crate::meta::TypeMeta {
            name: S::INSTANCES.name,
            label: S::INSTANCES_LABEL,
            hue: S::META.hue,
            doc: "Copies of a shape put somewhere, which refer to the shape instead of \
                  copying it: a table of where each one goes. They are drawn as they \
                  are, and copied only where something needs the shapes themselves.",
        });
    }
    of::<CubicNurbsSegmentCache>(meta);
    of::<TubePatch>(meta);
    of::<SweptPatch>(meta);
    of::<Circle>(meta);
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

    fn registry() -> NodeRegistry {
        let mut registry = NodeRegistry::new();
        crate::register(&mut registry);
        registry
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

    /// The rotation of a frame takes x, y and z to its normal, binormal and
    /// tangent, whichever way it is turned.
    #[test]
    fn a_frame_puts_x_along_the_normal_and_z_along_the_tangent() {
        for frame in example_frames(8) {
            let frame = MovingFrame::from(frame);
            let place = Placement::of(&onto_frame(&EvaluatedFrame::from(frame), 1.0));
            assert!(place.point(Vec3::X).distance(frame.position + frame.normal) < 1e-5);
            assert!(place.point(Vec3::Y).distance(frame.position + frame.binormal) < 1e-5);
            assert!(place.point(Vec3::Z).distance(frame.position + frame.tangent) < 1e-5);
        }
        let turned = quat_from_degrees(Vec3::new(0.0, 0.0, 90.0));
        assert!((turned * Vec3::X).distance(Vec3::Y) < 1e-6);
    }

    #[test]
    fn tubes_keep_their_radius_relative_to_their_size() {
        use crate::surface::Surface;
        let (tubes, _) = swept_example(&[6], &[Circle::new(0.25)]);
        let place = Placement::onto(&frame(), 2.0);
        for tube in &tubes {
            let placed = tube.placed(&place);
            for (u, v) in [(0.0, 0.0), (0.5, 0.3), (1.0, 0.75)] {
                let (before, normal) = tube.point(u, v);
                let (after, placed_normal) = placed.point(u, v);
                assert!(after.distance(place.point(before)) < 1e-3, "{u} {v}");
                assert!(placed_normal.distance(place.rotation * normal) < 1e-3, "{u} {v}");
            }
        }
    }

    /// Curves along a curve: an instance of every group of the shape per
    /// frame, which refer to the shape instead of copying it. Realized,
    /// they are the copies.
    #[test]
    fn every_point_gets_every_group_of_the_shape() {
        let registry = registry();
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
        assert_eq!(b.slot(placed).prototype.unwrap().elements, shape_slot);
        let copies = b.generic(&registry, &REALIZE, &[placed]).unwrap()[0];
        b.output("instances", placed);
        b.output("copies", copies);
        b.output("frames", frames);

        let mut engine = Engine::default();
        let id = engine.add_program(b.finish());
        let mut instance = engine.instantiate(id);
        engine.set_param(&mut instance, "shape", &shape).unwrap();
        engine.set_param_words(&mut instance, "shape#groups", &shape_offsets).unwrap();
        engine.evaluate([&mut instance]);

        let rows = engine.read::<InstanceRow>(&instance, "instances").unwrap().to_vec();
        assert_eq!(rows.len(), 4 * 2);
        assert_eq!(engine.groups(&instance, "instances"), Some(&[0, 2, 4, 6, 8][..]));
        let (elements, _) = engine.prototype_keys(&instance, "instances").unwrap();
        assert_eq!(engine.read::<CubicNurbsSegmentCache>(&instance, "copies").unwrap().len(), 4 * shape.len());
        assert_eq!(engine.output_key(&instance, "frames").is_some(), true);
        // The shape is referred to, stored once.
        assert_eq!(engine.fetch_key::<CubicNurbsSegmentCache>(elements).unwrap(), &shape[..]);

        let frames = engine.read::<EvaluatedFrame>(&instance, "frames").unwrap().to_vec();
        let copies = engine.read::<CubicNurbsSegmentCache>(&instance, "copies").unwrap();
        let groups = engine.groups(&instance, "copies").unwrap();
        let per = shape.len() as u32;
        let first = shape_offsets[1];
        assert_eq!(groups, [0, first, per, per + first, 2 * per, 2 * per + first, 3 * per, 3 * per + first, 4 * per]);
        let place = Placement::onto(&MovingFrame::from(frames[2]), 0.5);
        for (segment, copy) in shape.iter().zip(&copies[2 * shape.len()..]) {
            let expected = Placeable::placed(segment, &place);
            for u in [0.0, 0.5, 1.0] {
                assert!(copy.evaluate(u).0.distance(expected.evaluate(u).0) < 1e-4);
            }
        }
    }

    /// Transforms of instances come on top of where they are, and the
    /// shape stays where it was referred to.
    #[test]
    fn transforms_add_up() {
        let registry = registry();
        let mut b = ProgramBuilder::new("transforms");
        let circles = b.constant(&[Circle::new(1.0)]);
        let translate = b.constant(&[Vec3::new(1.0, 0.0, 0.0)]);
        let rotate = b.constant(&[Vec3::new(0.0, 0.0, 90.0)]);
        let scale = b.constant(&[2.0f32]);
        let once = b.generic(&registry, &TRANSFORM, &[circles, translate, rotate, scale]).unwrap()[0];
        let twice = b.generic(&registry, &TRANSFORM, &[once, translate, rotate, scale]).unwrap()[0];
        assert_eq!(b.slot(twice).prototype, b.slot(once).prototype);
        b.output("twice", twice);
        // Circles can not be copied anywhere, they have no place of their own.
        assert!(b.generic(&registry, &REALIZE, &[twice]).is_err());
        assert!(!registry.realizes(&Circle::INSTANCES));
        assert!(registry.realizes(&TubePatch::INSTANCES));

        let mut engine = Engine::default();
        let id = engine.add_program(b.finish());
        let mut instance = engine.instantiate(id);
        engine.evaluate([&mut instance]);
        let row = engine.read::<InstanceRow>(&instance, "twice").unwrap()[0];
        let point = row.transform.point(Vec3::X);
        assert!((row.transform.scale - 4.0).abs() < 1e-6);
        let expected = Similarity::new(quat_from_degrees(Vec3::new(0.0, 0.0, 90.0)), 2.0, Vec3::X);
        let expected = expected.then(&expected);
        assert!(point.distance(expected.point(Vec3::X)) < 1e-5);
    }
}
