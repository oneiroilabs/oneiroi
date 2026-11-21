use std::f32::consts::TAU;

use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use oneiroi_core::{
    node::{Caps, Drawable, GpuKernel, GpuOutput, Inputs, Outputs, PortDecl, Topology},
    types::{DataType, Field, FieldType, Layout, Scalar},
};

use crate::curve::{Curve, Piece, Profile, TubePatch, sweep_example};

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

describe_type! {
    /// A circle around the origin, given by its radius.
    Circle { label: "Circle", hue: Orange }
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

    fn pieces(profile: usize) -> usize {
        profile.min(1)
    }

    fn patch(piece: &Piece, profile: &[Self], _index: usize) -> TubePatch {
        TubePatch::of(piece, profile[0].radius)
    }

    const SWEEP_WGSL: &'static str = concat!(include_str!("../../curve/curve.wgsl"), include_str!("../../curve/fractions.wgsl"), include_str!("../../curve/sweep.wgsl"), "
alias Patch = TubePatch;

fn sweep_pieces() -> u32 {
    return min(in_profile_len(), 1u);
}

fn sweep_piece(
    segment: CubicNurbsSegmentCache,
    curve: u32,
    piece: u32,
    along: vec2<f32>,
    kind: u32,
    miter_start: vec4<f32>,
    miter_end: vec4<f32>,
    span: vec2<f32>,
    scale: vec4<f32>,
) -> TubePatch {
    let circle = in_profile(min(curve, in_profile_len() - 1u));
    return TubePatch(segment, circle.radius, along, kind, miter_start, miter_end, span, scale);
}
");

    const SWEEP_EXAMPLES: &'static [fn() -> Vec<Vec<u32>>] = &[
        || sweep_example(&[6, 5, 7], &[Circle::new(0.25), Circle::new(0.5)], 0),
        || sweep_example(&[9], &[Circle::new(1.0)], 1),
        || sweep_example::<Circle>(&[6], &[], 0),
        || sweep_example(&[], &[Circle::new(1.0)], 0),
        || {
            let (spine, offsets) = crate::curve::cornered_spine();
            crate::curve::sweep_example_along(&spine, &offsets, &[Circle::new(0.1)], 0, &[])
        },
        || {
            let (spine, offsets) = crate::curve::example_curves(&[9, 6, 5]);
            let scales: &[&[(f32, f32)]] = &[crate::curve::SPIKE, crate::curve::LATE];
            crate::curve::sweep_example_along(&spine, &offsets, &[Circle::new(0.3)], 0, scales)
        },
        || {
            let (spine, offsets) = crate::curve::cornered_spine();
            crate::curve::sweep_example_along(&spine, &offsets, &[Circle::new(0.1)], 1, &[crate::curve::LATE])
        },
    ];
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
        Some((glam::Vec3::ZERO, radius))
    },
};

node! {
    /// A circle around the origin, flat in the plane of `x` and `y`. As a
    /// profile it sweeps out a tube.
    pub static CIRCLE = "curve.circle" {
        label: "Circle",
        category: "Curve",
        version: 1,
        inputs {
            /// The radius, a circle for each.
            port PortDecl::new::<f32>("radius").with_default(&[1.0f32]);
        }
        outputs {
            /// The circles.
            port PortDecl::new::<Circle>("circle"), "Circles";
        }
        cpu: circle,
        gpu: &[GpuKernel {
            wgsl: "fn circle(index: u32) -> Circle { return Circle(in_radius(index)); }",
            outputs: &[GpuOutput {
                entry: "circle",
                len: |inputs| inputs.len(0),
                words: None,
            }],
            requires: Caps::NONE,
            reads: &[],
            accepts: None,
            examples: &[
                || vec![bytemuck::cast_slice(&[0.25f32, 1.0, -2.0, 0.0]).to_vec()],
                || vec![Vec::new()],
            ],
            stages: &[],
        }],
    }
}

fn circle(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let circles: Vec<_> = inputs.get(0).iter().map(|r| Circle::new(*r)).collect();
    outputs.set(0, &circles);
}
