use super::*;
use crate::{
    node::{GpuKernel, NodeDef},
    program::SlotId,
    types::{DataType, InstanceRow, InstanceTotal, Similarity, TypeInfo},
};

const NUMBERS: TypeInfo = TypeInfo::instances("instances<f32>", &f32::INFO);

/// Puts every group of the numbers at every one of `offsets`.
static INSTANCE: NodeDef = NodeDef {
    id: "test.instance",
    version: 0,
    inputs: &[
        PortDecl::new::<f32>("shape").grouped(),
        PortDecl::new::<f32>("offsets"),
    ],
    outputs: &[PortDecl::of("instances", NUMBERS).instances_of(0)],
    cpu: |inputs, outputs| {
        let groups = inputs.groups(0);
        let len = inputs.len(0) as u32;
        let rows: Vec<InstanceRow> = inputs
            .get::<f32>(1)
            .iter()
            .enumerate()
            .flat_map(|(point, offset)| {
                let groups = groups.clone();
                groups.into_iter().enumerate().map(move |(group, range)| InstanceRow {
                    transform: Similarity {
                        offset: Vec3::new(*offset, 0.0, 0.0),
                        ..Similarity::IDENTITY
                    },
                    group: group as u32,
                    first: point as u32 * len + range.start as u32,
                })
            })
            .collect();
        outputs.alloc::<InstanceRow>(0, rows.len()).copy_from_slice(&rows);
        let points = inputs.get::<f32>(1).len() as u32;
        outputs.set_total(0, InstanceTotal::repeated(points, InstanceTotal::single(groups.len() as u32, len), groups.len() as u32));
    },
    gpu: &[],
};

/// Moves instances further by `by`, without looking at their shape.
static MOVE: NodeDef = NodeDef {
    id: "test.move",
    version: 0,
    inputs: &[PortDecl::of("instances", NUMBERS), PortDecl::new::<f32>("by").one()],
    outputs: &[PortDecl::of("instances", NUMBERS).instances_of(0)],
    cpu: |inputs, outputs| {
        let by = inputs.value::<f32>(1);
        let rows: Vec<InstanceRow> = inputs
            .get::<InstanceRow>(0)
            .iter()
            .map(|row| InstanceRow {
                transform: Similarity {
                    offset: Vec3::new(row.transform.offset.x + by, 0.0, 0.0),
                    ..row.transform
                },
                ..*row
            })
            .collect();
        outputs.alloc::<InstanceRow>(0, rows.len()).copy_from_slice(&rows);
        outputs.set_total(0, inputs.total(0));
    },
    gpu: &[],
};

/// Every number of every instance, moved by its offset.
static REALIZE: NodeDef = NodeDef {
    id: "test.realize",
    version: 0,
    inputs: &[PortDecl::of("instances", NUMBERS)],
    outputs: &[PortDecl::new::<f32>("values")],
    cpu: |inputs, outputs| {
        let shape = inputs.prototype::<f32>(0);
        let groups = inputs.prototype_groups(0);
        let values: Vec<f32> = inputs
            .get::<InstanceRow>(0)
            .iter()
            .flat_map(|row| {
                shape[groups[row.group as usize].clone()]
                    .iter()
                    .map(move |value| value + row.transform.offset[0])
            })
            .collect();
        assert_eq!(values.len(), inputs.total(0).elements as usize, "the total is what the copies have");
        outputs.set(0, &values);
    },
    gpu: &[],
};

fn make_shape(b: &mut ProgramBuilder) -> SlotId {
    let values = b.param::<f32>("values", &[1., 2., 3.]);
    let size = b.constant(&[2u32]);
    b.node(&groups_split(), &[values, size]).unwrap()[0]
}

fn groups_split() -> &'static NodeDef {
    &super::groups::SPLIT
}

#[test]
fn a_transform_composes() {
    let turn = Similarity::new(glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2), 2.0, glam::Vec3::X);
    let moved = Similarity::new(glam::Quat::IDENTITY, 1.0, glam::Vec3::Y);
    let both = turn.then(&moved);
    let point = glam::Vec3::new(1.0, 2.0, 3.0);
    assert!(both.point(point).distance(turn.point(moved.point(point))) < 1e-5);
    assert!(Similarity::IDENTITY.point(point).distance(point) < 1e-6);
}

/// The shape of instances is the very value they were made of, and
/// it goes wherever they go.
#[test]
fn instances_refer_to_their_shape() {
    let mut b = ProgramBuilder::new("instances");
    let shape = make_shape(&mut b);
    let offsets = b.constant(&[10f32, 20.]);
    let placed = b.node(&INSTANCE, &[shape, offsets]).unwrap()[0];
    let prototype = b.slot(placed).prototype.unwrap();
    assert_eq!(prototype.elements, shape);
    assert_eq!(Some(prototype.groups), b.slot(shape).groups);

    let by = b.constant(&[100f32]);
    let moved = b.node(&MOVE, &[placed, by]).unwrap()[0];
    assert_eq!(b.slot(moved).prototype, Some(prototype));
    let values = b.node(&REALIZE, &[moved]).unwrap()[0];
    b.output("instances", moved);
    b.output("values", values);

    let mut engine = Engine::default();
    let id = engine.add_program(b.finish());
    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);
    assert_eq!(
        engine.read::<f32>(&instance, "values"),
        Some(&[111., 112., 113., 121., 122., 123.][..])
    );
    let rows = engine.read::<InstanceRow>(&instance, "instances").unwrap();
    assert_eq!(rows.iter().map(|row| row.first).collect::<Vec<_>>(), [0, 2, 3, 5]);

    // Nothing was copied: the shape is stored once, under its key.
    let (elements, groups) = engine.prototype_keys(&instance, "instances").unwrap();
    assert_eq!(engine.store.read::<f32>(elements), Some(&[1f32, 2., 3.][..]));
    assert_eq!(engine.store.read::<u32>(groups), Some(&[0u32, 2, 3][..]));
}

/// A program can take instances and give them back, their shape
/// passed along in hidden parameters and outputs.
#[test]
fn instances_go_into_and_out_of_programs() {
    let mut engine = Engine::default();
    let mut b = ProgramBuilder::new("inner");
    let given = b.param_words("instances", NUMBERS, &[]);
    assert_eq!(b.slot(given).prototype.map(|p| p.elements).is_some(), true);
    let by = b.constant(&[1000f32]);
    let moved = b.node(&MOVE, &[given, by]).unwrap()[0];
    b.output("instances", moved);
    let inner = engine.add_program(b.finish());
    assert_eq!(engine.programs().get(inner).visible_params().count(), 1);

    let mut b = ProgramBuilder::new("outer");
    let shape = make_shape(&mut b);
    let offsets = b.constant(&[1f32]);
    let placed = b.node(&INSTANCE, &[shape, offsets]).unwrap()[0];
    let called = b.call(engine.programs(), inner, &[Some(placed)]).unwrap()[0];
    assert!(b.slot(called).prototype.is_some());
    let values = b.node(&REALIZE, &[called]).unwrap()[0];
    b.output("values", values);
    b.output("instances", called);
    let outer = engine.add_program(b.finish());

    let mut instance = engine.instantiate(outer);
    engine.evaluate([&mut instance]);
    assert_eq!(
        engine.read::<f32>(&instance, "values"),
        Some(&[1002., 1003., 1004.][..])
    );
    let (elements, _) = engine.prototype_keys(&instance, "instances").unwrap();
    assert_eq!(engine.store.read::<f32>(elements), Some(&[1f32, 2., 3.][..]));

    // Each run of a call for each element could have a shape of its
    // own, so they can not give instances.
    let mut b = ProgramBuilder::new("each");
    let values = b.constant(&[1f32, 2.]);
    let shape = make_shape(&mut b);
    let placed = b.node(&INSTANCE, &[shape, values]).unwrap()[0];
    assert!(matches!(
        b.for_each(engine.programs(), inner, &[Some(placed)], 0, 4),
        Err(BuildError::Instances { .. })
    ));
}

/// What instances refer to is kept with them: planning them again
/// brings their shape back if it was dropped.
#[test]
fn the_shape_of_an_output_is_planned_with_it() {
    let mut b = ProgramBuilder::new("kept");
    let shape = make_shape(&mut b);
    let offsets = b.constant(&[1f32]);
    let placed = b.node(&INSTANCE, &[shape, offsets]).unwrap()[0];
    b.output("instances", placed);
    let mut engine = Engine::default();
    let id = engine.add_program(b.finish());
    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);
    let (elements, _) = engine.prototype_keys(&instance, "instances").unwrap();
    assert!(engine.store.contains(elements));
    // Everything is dropped, then planned again.
    engine.end_frame(0);
    engine.end_frame(0);
    engine.evaluate([&mut instance]);
    assert!(engine.store.contains(elements));
}

/// The shape of instances can be read in a kernel on the GPU.
#[test]
fn kernels_read_the_shape() {
    static GPU_REALIZE: NodeDef = NodeDef {
        gpu: &[GpuKernel {
            wgsl: "
fn realize(index: u32) -> f32 {
let row = in_instances(in_instances_row_of(index));
let start = in_instances_prototype_start(row.group);
let end = in_instances_prototype_end(row.group);
let count = in_instances_prototype_groups() + in_instances_prototype_len() + in_instances_total();
return in_instances_prototype(start + index % max(end - start, 1u)) + row.transform.offset.x + f32(count);
}",
            outputs: &[crate::node::GpuOutput {
                entry: "realize",
                len: |inputs| inputs.prototype_len(0),
                words: None,
            }],
            requires: crate::node::Caps::NONE,
            reads: &[],
            accepts: None,
            examples: &[],
            stages: &[],
        }],
        id: "test.gpu_realize",
        version: 0,
        inputs: &[PortDecl::of("instances", NUMBERS)],
        outputs: &[PortDecl::new::<f32>("values")],
        cpu: |_, _| {},
    };
    crate::shader::compute(&GPU_REALIZE, &GPU_REALIZE.gpu[0]).unwrap_or_else(|e| panic!("{e}"));
    // And so does the culling of instances before they are drawn.
    crate::shader::cull().unwrap_or_else(|e| panic!("{e}"));
    // The rows, and the shape with its offsets and the total.
    assert_eq!(GPU_REALIZE.all_inputs(), 4);
}
