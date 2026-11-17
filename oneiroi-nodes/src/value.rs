//! Values as nodes, so a list typed in once can be linked to many places.

use glam::Vec3;
use oneiroi_core::{
    node::{Inputs, NodeDef, NodeRegistry, Outputs, PortDecl, Shape},
    types::{DataType, PortDefault},
};

fn pass<T: DataType>(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    outputs.set(0, inputs.get::<T>(0));
}

/// A list of numbers.
pub static F32: NodeDef = NodeDef {
    id: "value.f32",
    display_name: "Numbers",
    inputs: &[PortDecl::new::<f32>("values").with_default(PortDefault::F32(&[0.0]))],
    outputs: &[PortDecl::new::<f32>("values")],
    shape: Shape::Map,
    cpu: pass::<f32>,
    gpu: &[],
};

/// A list of points or directions.
pub static VEC3: NodeDef = NodeDef {
    id: "value.vec3",
    display_name: "Vectors",
    inputs: &[PortDecl::new::<Vec3>("values").with_default(PortDefault::F32(&[0.0; 3]))],
    outputs: &[PortDecl::new::<Vec3>("values")],
    shape: Shape::Map,
    cpu: pass::<Vec3>,
    gpu: &[],
};

pub fn register(registry: &mut NodeRegistry) {
    registry.register(&F32);
    registry.register(&VEC3);
}
