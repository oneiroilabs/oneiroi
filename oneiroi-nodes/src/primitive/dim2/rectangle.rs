use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use oneiroi_core::{
    node::{Inputs, Outputs, PortDecl},
    types::{DataType, Field, FieldType, Layout, Scalar},
};

/// A rectangle around the origin, flat in the plane of `x` and `y`: its
/// size, and how round its corners are. Every point on it is computed when
/// it is asked for.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Rectangle {
    size: Vec2,
    corner_radius: f32,
    _padding: f32,
}

impl Rectangle {
    /// Sizes below `0` are `0`, and the corners are at most as round as
    /// the shorter side allows.
    pub fn new(width: f32, height: f32, corner_radius: f32) -> Self {
        let size = Vec2::new(width, height).max(Vec2::ZERO);
        Self {
            size,
            corner_radius: corner_radius.clamp(0.0, size.min_element() * 0.5),
            _padding: 0.0,
        }
    }

    /// Its width along `x` and its height along `y`.
    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn corner_radius(&self) -> f32 {
        self.corner_radius
    }
}

impl DataType for Rectangle {
    type ConfigurationOptions = ();

    const NAME: &'static str = "rectangle";
    const WGSL: &'static str = "Rectangle";
    const SCALAR: Scalar = Scalar::F32;
    const LAYOUT: Layout = Layout::Struct(&[
        Field::new("size", FieldType::Vec2),
        Field::new("corner_radius", FieldType::F32),
        Field::pad(1),
    ]);
}

describe_type! {
    /// A rectangle around the origin, given by its width, its height and how
    /// round its corners are.
    Rectangle { label: "Rectangle", hue: Orange }
}

node! {
    /// A rectangle around the origin, flat in the plane of `x` and `y`. It is a
    /// curve wherever one is needed, and as a profile it sweeps out a bar.
    pub static RECTANGLE = "curve.rectangle" {
        label: "Rectangle",
        category: "Curve",
        version: 2,
        inputs {
            /// How wide it is along `x`.
            port PortDecl::new::<f32>("width").with_default(&[1.0f32]).one();
            /// How high it is along `y`.
            port PortDecl::new::<f32>("height").with_default(&[1.0f32]).one();
            /// The radius of its corners, `0` for sharp ones. At most half its
            /// shorter side.
            port PortDecl::new::<f32>("corner_radius").with_default(&[0.0f32]).one();
        }
        outputs {
            /// The rectangle.
            port PortDecl::new::<Rectangle>("rectangle").one();
        }
        cpu: rectangle,
        gpu: &[],
    }
}

fn rectangle(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let [width, height, corner] = [0, 1, 2].map(|index| inputs.value::<f32>(index));
    outputs.set(0, &[Rectangle::new(width, height, corner)]);
}
