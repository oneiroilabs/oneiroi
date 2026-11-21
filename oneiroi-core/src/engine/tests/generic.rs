use super::*;
use crate::{node::{GenericNode, GenericPort, NodeDef, NodeRegistry}, types::Trait};

const NUMBER: Trait = Trait { name: "number" };

static DOUBLE: GenericNode = GenericNode {
    id: "test.double",
    inputs: &[GenericPort::implementing("values", NUMBER)],
    outputs: &[GenericPort::implementing("values", NUMBER)],
};
static DOUBLE_F32: NodeDef = NodeDef {
    id: "test.double<f32>",
    inputs: &[PortDecl::new::<f32>("values")],
    outputs: &[PortDecl::new::<f32>("values")],
    version: 0,
    cpu: |i, o| o.set(0, &i.get::<f32>(0).iter().map(|v| v * 2.).collect::<Vec<_>>()),
    gpu: &[],
};
static DOUBLE_U32: NodeDef = NodeDef {
    id: "test.double<u32>",
    inputs: &[PortDecl::new::<u32>("values")],
    outputs: &[PortDecl::new::<u32>("values")],
    version: 0,
    cpu: |i, o| o.set(0, &i.get::<u32>(0).iter().map(|v| v * 2).collect::<Vec<_>>()),
    gpu: &[],
};

fn registry() -> NodeRegistry {
    let mut registry = NodeRegistry::new();
    registry.register_instance(&DOUBLE, &DOUBLE_F32);
    registry.register_instance(&DOUBLE, &DOUBLE_U32);
    registry
}

#[test]
fn the_connected_type_picks_the_version() {
    let registry = registry();
    let mut b = ProgramBuilder::new("generic");
    let floats = b.constant::<f32>(&[1.5]);
    let ints = b.constant::<u32>(&[4]);
    let floats = b.generic(&registry, &DOUBLE, &[floats]).unwrap()[0];
    let ints = b.generic(&registry, &DOUBLE, &[ints]).unwrap()[0];
    b.output("floats", floats);
    b.output("ints", ints);

    let mut engine = Engine::default();
    let id = engine.add_program(b.finish());
    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);
    assert_eq!(engine.read::<f32>(&instance, "floats"), Some(&[3.0][..]));
    assert_eq!(engine.read::<u32>(&instance, "ints"), Some(&[8][..]));
}

#[test]
fn a_type_has_to_implement_the_trait() {
    let registry = registry();
    let mut b = ProgramBuilder::new("generic");
    let vector = b.constant::<glam::Vec3>(&[glam::Vec3::ONE]);
    assert_eq!(
        b.generic(&registry, &DOUBLE, &[vector]),
        Err(BuildError::Trait {
            target: "test.double".into(),
            port: "values".into(),
            expected: "number",
            found: "vec3",
        })
    );
}

#[test]
fn the_registry_knows_who_implements_what() {
    let registry = registry();
    let mut names: Vec<_> = registry
        .implementors(NUMBER)
        .iter()
        .map(|ty| ty.name)
        .collect();
    names.sort();
    assert_eq!(names, ["f32", "u32"]);
    assert_eq!(registry.instances(&DOUBLE).len(), 2);
    assert!(registry.generic("test.double").is_some());
    assert!(registry.get("test.double<u32>").is_some());
}

#[test]
#[should_panic(expected = "is not a version of")]
fn versions_have_to_line_up_with_their_generic_node() {
    NodeRegistry::new().register_instance(&DOUBLE, &SCALE);
}
