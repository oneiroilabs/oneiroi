use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use oneiroi_core::{
    node::{Inputs, Outputs, PortDecl},
    types::{DataType, Field, FieldType, Layout, PortDefault, Scalar},
};

/// A rectangle around the origin, flat in the plane of `x` and `y`: its
/// size, and how round its corners are. Every point on it is computed when
/// it is asked for.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Rectangle {
    size: [f32; 2],
    corner_radius: f32,
    _padding: f32,
}

impl Rectangle {
    /// Sizes below `0` are `0`, and the corners are at most as round as
    /// the shorter side allows.
    pub fn new(width: f32, height: f32, corner_radius: f32) -> Self {
        let size = Vec2::new(width, height).max(Vec2::ZERO);
        Self {
            size: size.to_array(),
            corner_radius: corner_radius.clamp(0.0, size.min_element() * 0.5),
            _padding: 0.0,
        }
    }

    /// Its width along `x` and its height along `y`.
    pub fn size(&self) -> Vec2 {
        Vec2::from_array(self.size)
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
        version: 1,
        inputs {
            /// How wide it is along `x`, a rectangle for each. The shorter of the
            /// widths and heights repeats its last value.
            port PortDecl::new::<f32>("width").with_default(PortDefault::F32(&[1.0]));
            /// How high it is along `y`.
            port PortDecl::new::<f32>("height").with_default(PortDefault::F32(&[1.0]));
            /// The radius of its corners, `0` for sharp ones. At most half its
            /// shorter side.
            port PortDecl::new::<f32>("corner_radius").with_default(PortDefault::F32(&[0.0])).one();
        }
        outputs {
            /// The rectangles.
            port PortDecl::new::<Rectangle>("rectangle"), "Rectangles";
        }
        cpu: rectangle,
        gpu: &[],
    }
}

fn rectangle(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let (widths, heights) = (inputs.get::<f32>(0), inputs.get::<f32>(1));
    let corner = inputs.value::<f32>(2);
    let count = match widths.is_empty() || heights.is_empty() {
        true => 0,
        false => widths.len().max(heights.len()),
    };
    let at = |values: &[f32], index: usize| values[index.min(values.len() - 1)];
    let rectangles: Vec<_> = (0..count)
        .map(|index| Rectangle::new(at(widths, index), at(heights, index), corner))
        .collect();
    outputs.set(0, &rectangles);
}
