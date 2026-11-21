use bytemuck::{Pod, Zeroable};

use super::*;
use crate::{node::{Inputs, Outputs, PortDecl}, types::{DataType, FieldType, Layout, Scalar}};

/// A part of a shape: a whole group of them is one.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Part(f32);

impl DataType for Part {
    type ConfigurationOptions = ();

    const NAME: &'static str = "part";
    const WGSL: &'static str = "f32";
    const SCALAR: Scalar = Scalar::F32;
    const LAYOUT: Layout = Layout::Value(FieldType::F32);
    const PARTS: bool = true;
}

fn nothing(_: &Inputs<'_>, _: &mut Outputs) {}

/// One shape of one number.
static SHAPE: NodeDef = NodeDef {
    id: "test.shape",
    version: 0,
    inputs: &[PortDecl::new::<f32>("size").one()],
    outputs: &[PortDecl::new::<Part>("shape").one()],
    cpu: nothing,
    gpu: &[],
};

/// A shape of each number, a group of parts for each.
static SHAPES: NodeDef = NodeDef {
    id: "test.shapes",
    version: 0,
    inputs: &[PortDecl::new::<f32>("sizes")],
    outputs: &[PortDecl::new::<Part>("shapes").grouped()],
    cpu: nothing,
    gpu: &[],
};

static TAKE_ONE: NodeDef = NodeDef {
    id: "test.take_one",
    version: 0,
    inputs: &[PortDecl::new::<Part>("shape").one()],
    outputs: &[PortDecl::new::<f32>("size").one()],
    cpu: nothing,
    gpu: &[],
};

/// Numbers element by element: one for one.
static DOUBLE: NodeDef = NodeDef {
    id: "test.double",
    version: 0,
    inputs: &[PortDecl::new::<f32>("values")],
    outputs: &[PortDecl::new::<f32>("values").follows(&[0])],
    cpu: nothing,
    gpu: &[],
};

#[test]
fn a_shape_of_parts_is_one_item() {
    assert_eq!(Card::Many.items(&Part::INFO), Items::One);
    assert_eq!(Card::Grouped.items(&Part::INFO), Items::Many);
    assert_eq!(Card::Many.items(&f32::INFO), Items::Many);
    assert_eq!(Card::Grouped.items(&f32::INFO), Items::Nested);

    // However many parts, a constant of them is one shape.
    let mut b = ProgramBuilder::new("parts");
    let parts = b.constant(&[Part(1.0), Part(2.0), Part(3.0)]);
    assert_eq!(b.items(parts), Items::One);
    assert!(b.node(&TAKE_ONE, &[parts]).is_ok());
}

#[test]
fn only_one_item_goes_where_one_is_wanted() {
    let mut b = ProgramBuilder::new("one");
    let sizes = b.constant(&[1.0f32, 2.0]);
    assert!(matches!(b.node(&SHAPE, &[sizes]), Err(BuildError::Card { .. })));
    let size = b.constant(&[1.0f32]);
    let shape = b.node(&SHAPE, &[size]).unwrap()[0];
    assert!(b.node(&TAKE_ONE, &[shape]).is_ok());

    // A list of shapes is no shape.
    let shapes = b.node(&SHAPES, &[sizes]).unwrap()[0];
    assert_eq!(b.items(shapes), Items::Many);
    assert!(matches!(b.node(&TAKE_ONE, &[shapes]), Err(BuildError::Card { .. })));
}

#[test]
fn an_output_follows_the_inputs_it_is_made_of() {
    let mut b = ProgramBuilder::new("follow");
    let one = b.constant(&[1.0f32]);
    let many = b.constant(&[1.0f32, 2.0]);
    let doubled = b.node(&DOUBLE, &[one]).unwrap()[0];
    assert_eq!(b.items(doubled), Items::One);
    assert!(b.node(&SHAPE, &[doubled]).is_ok());
    let doubled = b.node(&DOUBLE, &[many]).unwrap()[0];
    assert_eq!(b.items(doubled), Items::Many);
}
