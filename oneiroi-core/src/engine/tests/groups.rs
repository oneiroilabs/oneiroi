use super::*;
use crate::{node::{Card, NodeDef}, program::SlotId, types::DataType};

/// The sum of every group, and how many elements each had.
static SUMS: NodeDef = NodeDef {
    id: "test.sums",
    version: 0,
    inputs: &[PortDecl::new::<f32>("values").grouped()],
    outputs: &[
        PortDecl::new::<f32>("sums"),
        PortDecl::new::<u32>("lengths"),
    ],
    cpu: |inputs, outputs| {
        let values = inputs.get::<f32>(0);
        let groups = inputs.groups(0);
        let sums: Vec<f32> = groups.iter().map(|g| values[g.clone()].iter().sum()).collect();
        let lengths: Vec<u32> = groups.iter().map(|g| g.len() as u32).collect();
        outputs.set(0, &sums);
        outputs.set(1, &lengths);
    },
    gpu: &[],
};

/// Splits a list into groups of `size` elements.
pub(super) static SPLIT: NodeDef = NodeDef {
    id: "test.split",
    version: 0,
    inputs: &[
        PortDecl::new::<f32>("values"),
        PortDecl::new::<u32>("size").one(),
    ],
    outputs: &[PortDecl::new::<f32>("values").grouped()],
    cpu: |inputs, outputs| {
        let values = inputs.get::<f32>(0);
        let size = inputs.value::<u32>(1).max(1) as usize;
        outputs.set(0, values);
        let offsets: Vec<u32> = (0..values.len().div_ceil(size))
            .map(|group| (group * size) as u32)
            .chain([values.len() as u32])
            .collect();
        outputs.set_groups(0, &offsets);
    },
    gpu: &[],
};

/// Doubles every element and keeps the groups as they are.
static DOUBLE: NodeDef = NodeDef {
    id: "test.double_grouped",
    version: 0,
    inputs: &[PortDecl::new::<f32>("values").grouped()],
    outputs: &[PortDecl::new::<f32>("values").grouped_like(0)],
    cpu: |inputs, outputs| {
        let doubled: Vec<f32> = inputs.get::<f32>(0).iter().map(|v| v * 2.0).collect();
        outputs.set(0, &doubled);
    },
    gpu: &[],
};

fn split(b: &mut ProgramBuilder, values: &[f32], size: u32) -> SlotId {
    let values = b.constant(values);
    let size = b.constant(&[size]);
    b.node(&SPLIT, &[values, size]).unwrap()[0]
}

#[test]
fn groups_go_from_node_to_node() {
    let mut b = ProgramBuilder::new("groups");
    let split = split(&mut b, &[1., 2., 3., 4., 5.], 2);
    assert_eq!(b.slot(split).card, Card::Grouped);
    let doubled = b.node(&DOUBLE, &[split]).unwrap()[0];
    // Groups which are passed along are the very same slot.
    assert_eq!(b.slot(doubled).groups, b.slot(split).groups);
    let sums = b.node(&SUMS, &[doubled]).unwrap();
    b.output("doubled", doubled);
    b.output("sums", sums[0]);
    b.output("lengths", sums[1]);

    let mut engine = Engine::default();
    let id = engine.add_program(b.finish());
    let mut instance = engine.instantiate(id);
    assert_eq!(engine.evaluate([&mut instance]), 3);
    assert_eq!(engine.read::<f32>(&instance, "sums"), Some(&[6., 14., 10.][..]));
    assert_eq!(engine.read::<u32>(&instance, "lengths"), Some(&[2, 2, 1][..]));
    assert_eq!(engine.groups(&instance, "doubled"), Some(&[0, 2, 4, 5][..]));
    assert_eq!(engine.groups(&instance, "sums"), None);
}

#[test]
fn a_list_without_groups_is_a_single_group() {
    let mut b = ProgramBuilder::new("one group");
    let values = b.constant::<f32>(&[1., 2., 3.]);
    let sums = b.node(&SUMS, &[values]).unwrap();
    b.output("sums", sums[0]);
    let mut engine = Engine::default();
    let id = engine.add_program(b.finish());
    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);
    assert_eq!(engine.read::<f32>(&instance, "sums"), Some(&[6.][..]));
}

#[test]
fn a_single_value_is_not_a_list() {
    let mut b = ProgramBuilder::new("cards");
    let values = b.constant::<f32>(&[1., 2.]);
    let sizes = b.constant::<u32>(&[1, 2]);
    assert_eq!(
        b.node(&SPLIT, &[values, sizes]),
        Err(BuildError::Card {
            target: "test.split".into(),
            port: "size".into()
        })
    );
    // A parameter can be single too.
    let size = b.param_one("size", u32::INFO, &[2]);
    assert!(b.node(&SPLIT, &[values, size]).is_ok());
    let many = b.param::<u32>("many", &[2]);
    assert!(b.node(&SPLIT, &[values, many]).is_err());
}

#[test]
fn a_program_runs_for_each_group() {
    let mut engine = Engine::default();
    let mut b = ProgramBuilder::new("sum of one group");
    let values = b.param::<f32>("values", &[]);
    let sums = b.node(&SUMS, &[values]).unwrap();
    b.output("sum", sums[0]);
    let inner = engine.add_program(b.finish());

    let mut b = ProgramBuilder::new("each group");
    let split = split(&mut b, &[1., 2., 3., 4., 5.], 2);
    let each = b
        .for_each(engine.programs(), inner, &[Some(split)], 0, 100)
        .unwrap();
    assert_eq!(b.slot(each[0]).card, Card::Grouped);
    b.output("sums", each[0]);
    let id = engine.add_program(b.finish());

    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);
    // One run per group, and every run is a group of its own.
    assert_eq!(engine.read::<f32>(&instance, "sums"), Some(&[3., 7., 5.][..]));
    assert_eq!(engine.groups(&instance, "sums"), Some(&[0, 1, 2, 3][..]));
}

#[test]
fn groups_go_into_and_out_of_programs() {
    let mut engine = Engine::default();
    // Doubles what it is given and keeps its groups.
    let mut b = ProgramBuilder::new("double");
    let values = b.param_grouped("values", f32::INFO, &[]);
    let doubled = b.node(&DOUBLE, &[values]).unwrap()[0];
    b.output("doubled", doubled);
    let inner = engine.add_program(b.finish());
    assert_eq!(engine.programs().get(inner).visible_params().count(), 1);

    let mut b = ProgramBuilder::new("outer");
    let split = split(&mut b, &[1., 2., 3.], 1);
    let doubled = b.call(engine.programs(), inner, &[Some(split)]).unwrap()[0];
    let sums = b.node(&SUMS, &[doubled]).unwrap();
    b.output("sums", sums[0]);
    let id = engine.add_program(b.finish());

    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);
    assert_eq!(engine.read::<f32>(&instance, "sums"), Some(&[2., 4., 6.][..]));
}
