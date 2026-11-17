use std::f32::consts::TAU;

use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use oneiroi_core::{
    node::{Caps, Drawable, GpuKernel, GpuOutput, Inputs, NodeDef, Outputs, PortDecl, Topology},
    types::{DataType, Field, FieldType, Layout, PortDefault, Scalar},
};

use crate::curve::{Curve, Profile, TubePatch, fractions, nurbs::CubicNurbsSegmentCache, sweep_example};

/// A circle around the origin. It is its radius and nothing else, every
/// point on it is computed when it is asked for.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Circle {
    radius: f32,
}

impl Circle {
    pub fn new(radius: f32) -> Self {
        Self { radius }
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }
}

impl DataType for Circle {
    type ConfigurationOptions = ();

    const NAME: &'static str = "circle";
    const WGSL: &'static str = "Circle";
    const SCALAR: Scalar = Scalar::F32;
    const LAYOUT: Layout = Layout::Struct(&[Field::new("radius", FieldType::F32)]);
}

/// Runs counter-clockwise, starting on the x axis.
impl Curve<Vec2> for Circle {
    fn sample_unchecked(&self, t: f32) -> Vec2 {
        Vec2::from_angle(t * TAU) * self.radius
    }

    fn sample(&self, t: f32) -> Vec2 {
        self.sample_unchecked(t)
    }

    fn domain(&self) -> std::ops::Range<f32> {
        0.0..1.0
    }

    fn length(&self) -> f32 {
        TAU * self.radius.abs()
    }

    fn t_at_distance(&self, distance: f32) -> f32 {
        let length = self.length();
        if length > 0.0 { distance / length } else { 0.0 }
    }
}

impl Profile for Circle {
    type Patch = TubePatch;

    /// A circle per curve: the circle of the curve's group, or the last
    /// one if there are fewer circles than curves.
    fn select(profile: &[Self], group: usize) -> &[Self] {
        match profile.len() {
            0 => profile,
            len => std::slice::from_ref(&profile[group.min(len - 1)]),
        }
    }

    fn patches(segments: usize, profile: usize) -> usize {
        match profile {
            0 => 0,
            _ => segments,
        }
    }

    const SWEEP_WGSL: &'static str = concat!(include_str!("../../curve/fractions.wgsl"), "
fn sweep_patch(index: u32) -> TubePatch {
    let curve = in_spine_group_of(index);
    let circle = in_profile(min(curve, in_profile_len() - 1u));
    let along = curve_fractions(in_spine(index), in_spine(in_spine_end(curve) - 1u));
    return TubePatch(in_spine(index), circle.radius, along);
}

fn sweep_offset(group: u32) -> u32 {
    if (in_profile_len() == 0u) {
        return 0u;
    }
    return spine_offset(group);
}
");

    const SWEEP_EXAMPLES: &'static [fn() -> Vec<Vec<u32>>] = &[
        || sweep_example(&[6, 5, 7], &[Circle::new(0.25), Circle::new(0.5)]),
        || sweep_example(&[9], &[Circle::new(1.0)]),
        || sweep_example::<Circle>(&[6], &[]),
        || sweep_example(&[], &[Circle::new(1.0)]),
    ];

    fn sweep(spine: &[CubicNurbsSegmentCache], profile: &[Self]) -> Vec<TubePatch> {
        let Some(circle) = profile.first() else {
            return Vec::new();
        };
        spine
            .iter()
            .zip(fractions(spine))
            .map(|(segment, along)| TubePatch::new(*segment, circle.radius, along))
            .collect()
    }
}

/// A circle can be looked at as it is: a line around the origin of the
/// plane it lies in.
pub const CIRCLE_LINE: Drawable = Drawable {
    ty: Circle::INFO,
    topology: Topology::Line,
    wgsl: include_str!("circle.wgsl"),
    bounds: |words| {
        let circles: &[Circle] = bytemuck::try_cast_slice(words).ok()?;
        let radius = circles.iter().map(|circle| circle.radius.abs()).reduce(f32::max)?;
        Some(([0.0; 3], radius))
    },
};

pub static CIRCLE: NodeDef = NodeDef {
    id: "curve.circle",
    display_name: "Circle",
    inputs: &[PortDecl::new::<f32>("radius").with_default(PortDefault::F32(&[1.0]))],
    outputs: &[PortDecl::new::<Circle>("circle")],
    version: 1,
    cpu: circle,
    gpu: &[GpuKernel {
        wgsl: "fn circle(index: u32) -> Circle { return Circle(in_radius(index)); }",
        outputs: &[GpuOutput {
            entry: "circle",
            len: |inputs| inputs.len(0),
        }],
        requires: Caps::NONE,
        reads: &[],
        accepts: None,
        examples: &[
            || vec![bytemuck::cast_slice(&[0.25f32, 1.0, -2.0, 0.0]).to_vec()],
            || vec![Vec::new()],
        ],
    }],
};

fn circle(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let circles: Vec<_> = inputs.get(0).iter().map(|r| Circle::new(*r)).collect();
    outputs.set(0, &circles);
}
