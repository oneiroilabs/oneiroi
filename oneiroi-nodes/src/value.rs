//! Values as nodes, so a list typed in once can be linked to many places.

use glam::Vec3;
use oneiroi_core::{
    node::{Inputs, NodeRegistry, Outputs, PortDecl},
    types::{DataType, PortDefault},
};

fn pass<T: DataType>(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    outputs.set(0, inputs.get::<T>(0));
}

node! {
    /// A list of numbers, typed in once and linked to as many places as need it.
    pub static F32 = "value.f32" {
        label: "Numbers",
        category: "Value",
        version: 1,
        inputs {
            /// The numbers.
            port PortDecl::new::<f32>("values").with_default(PortDefault::F32(&[0.0]));
        }
        outputs {
            /// The same numbers.
            port PortDecl::new::<f32>("values");
        }
        cpu: pass::<f32>,
        gpu: &[],
    }
}

node! {
    /// A list of points or directions, typed in once and linked to as many
    /// places as need them.
    pub static VEC3 = "value.vec3" {
        label: "Vectors",
        category: "Value",
        version: 1,
        inputs {
            /// The vectors.
            port PortDecl::new::<Vec3>("values").with_default(PortDefault::F32(&[0.0; 3]));
        }
        outputs {
            /// The same vectors.
            port PortDecl::new::<Vec3>("values");
        }
        cpu: pass::<Vec3>,
        gpu: &[],
    }
}

describe_type! {
    /// A number with a fraction, like `0.5` or `-3`.
    f32 { label: "Number", hue: Sky }
}

describe_type! {
    /// A whole number, never below `0`, like a count or an index.
    u32 { label: "Whole number", hue: Cyan }
}

describe_type! {
    /// Two numbers `x` and `y`: a place on a surface or in a plane.
    glam::Vec2 { label: "2D vector", hue: Indigo }
}

describe_type! {
    /// Three numbers `x`, `y` and `z`: a point or a direction in space.
    Vec3 { label: "Vector", hue: Violet }
}

describe_type! {
    /// A point in space with a weight `w`, like the control points of a curve.
    [f32; 4] { label: "Weighted point", hue: Purple }
}

pub fn register(registry: &mut NodeRegistry) {
    registry.register(&F32);
    registry.register(&VEC3);
}
