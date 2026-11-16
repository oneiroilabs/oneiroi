use std::f32::consts::TAU;

use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use oneiroi_core::{
    node::{Inputs, NodeDef, Outputs, PortDecl, Shape},
    types::{DataType, PortDefault, Scalar},
};

use crate::curve::{Curve, Profile};

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
    fn point(profile: &[Self], v: f32) -> (Vec2, Vec2) {
        let direction = Vec2::from_angle(v * TAU);
        (direction * profile[0].radius, direction)
    }
}

pub static CIRCLE: NodeDef = NodeDef {
    id: "curve.circle",
    display_name: "Circle",
    inputs: &[PortDecl::new::<f32>("radius").with_default(PortDefault::F32(&[1.0]))],
    outputs: &[PortDecl::new::<Circle>("circle")],
    shape: Shape::Map,
    cpu: circle,
    gpu: &[],
};

fn circle(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let circles: Vec<_> = inputs.get(0).iter().map(|r| Circle::new(*r)).collect();
    outputs.set(0, &circles);
}
