use std::collections::HashMap;

use glam::Vec3;
use oneiroi_core::{
    exec::Engine,
    node::NodeRegistry,
    program::{BuildError, ProgramBuilder, ProgramId},
};
use oneiroi_graph::{
    Context, EditError, FormatError, Graph, Input, Library, LibraryError, LowerErrorKind, NodeId,
    Value,
};
use oneiroi_nodes::{
    curve::{NURBS, SWEEP, nurbs::clamped_uniform_knots},
    primitive::CIRCLE,
};

const POINTS: [[f32; 4]; 5] = [
    [0., 0., 0., 1.],
    [1., 2., 0., 1.],
    [2., 0., 1., 1.],
    [3., 2., 0., 1.],
    [4., 0., -1., 1.],
];

fn registry() -> NodeRegistry {
    let mut registry = NodeRegistry::new();
    oneiroi_nodes::register(&mut registry);
    registry
}

struct Nodes {
    curve: NodeId,
    circle: NodeId,
    sweep: NodeId,
}

/// A circle swept along a curve, with the radius exposed.
fn tube() -> (Graph, Nodes) {
    let mut graph = Graph::new("tube");
    let curve = graph.add("curve.nurbs");
    graph
        .set(curve, "points", Value::F32(POINTS.as_flattened().to_vec()))
        .unwrap();
    graph
        .set(curve, "knots", Value::F32(clamped_uniform_knots(5)))
        .unwrap();

    let circle = graph.add("curve.circle");
    graph
        .expose(circle, "radius", "radius", Some(0.25.into()))
        .unwrap();

    let sweep = graph.add("curve.sweep");
    graph.connect(curve, "segments", sweep, "spine").unwrap();
    graph.connect(circle, "circle", sweep, "profile").unwrap();
    graph.set(sweep, "sides", 6u32).unwrap();

    graph.add_output("positions", sweep, "positions").unwrap();
    graph.add_output("indices", sweep, "indices").unwrap();
    graph.display = Some(sweep);
    (
        graph,
        Nodes {
            curve,
            circle,
            sweep,
        },
    )
}

fn lower(graph: &Graph, engine: &mut Engine) -> ProgramId {
    let program = graph
        .lower(Context {
            registry: &registry(),
            programs: engine.programs(),
            graphs: &HashMap::new(),
        })
        .unwrap();
    engine.add_program(program)
}

fn error(graph: &Graph) -> (Option<NodeId>, Option<String>, LowerErrorKind) {
    let error = graph
        .lower(Context {
            registry: &registry(),
            programs: Engine::default().programs(),
            graphs: &HashMap::new(),
        })
        .unwrap_err();
    (error.node, error.port, error.kind)
}

#[test]
fn a_graph_runs_like_the_program_written_by_hand() {
    let mut engine = Engine::default();
    let (graph, _) = tube();
    let lowered = lower(&graph, &mut engine);

    let mut b = ProgramBuilder::new("by hand");
    let points = b.constant(&POINTS);
    let knots = b.constant(&clamped_uniform_knots(5));
    let radius = b.param::<f32>("radius", &[0.25]);
    let rings = b.constant::<u32>(&[16]);
    let sides = b.constant::<u32>(&[6]);
    let spine = b.node(&NURBS, &[points, knots]).unwrap()[0];
    let circle = b.node(&CIRCLE, &[radius]).unwrap()[0];
    let mesh = b
        .generic(&registry(), &SWEEP, &[spine, circle, rings, sides])
        .unwrap();
    b.output("positions", mesh[0]);
    let by_hand = engine.add_program(b.finish());

    let mut a = engine.instantiate(lowered);
    let mut b = engine.instantiate(by_hand);
    // The same nodes on the same values are the same work.
    assert_eq!(engine.evaluate([&mut a]), 3);
    assert_eq!(engine.evaluate([&mut b]), 0);
    assert_eq!(
        engine.output_key(&a, "positions"),
        engine.output_key(&b, "positions")
    );

    // 2 segments with the default of 16 rings, 6 sides and the seam.
    let positions = engine.read::<Vec3>(&a, "positions").unwrap();
    assert_eq!(positions.len(), (2 * 16 + 1) * 7);

    engine.set_param(&mut a, "radius", &[0.5f32]).unwrap();
    assert_eq!(engine.evaluate([&mut a]), 2);
}

#[test]
fn a_graph_survives_its_file() {
    let (mut graph, nodes) = tube();
    graph.node_mut(nodes.sweep).unwrap().meta.position = [320.0, 40.0];
    graph.node_mut(nodes.sweep).unwrap().meta.comment = Some("the mesh".into());
    // This build has never heard of this node.
    let unknown = graph.add("someone_elses.node");
    graph.connect(nodes.curve, "segments", unknown, "curve").unwrap();

    let json = graph.to_json();
    assert_eq!(Graph::from_json(&json).unwrap(), graph);
    assert!(json.contains("\"someone_elses.node\""));

    // Nothing reads the unknown node, so the graph still works.
    lower(&Graph::from_json(&json).unwrap(), &mut Engine::default());

    let newer = json.replacen("\"version\": 1", "\"version\": 2", 1);
    assert!(matches!(
        Graph::from_json(&newer),
        Err(FormatError::Version(2))
    ));
    assert!(matches!(
        Graph::from_json("{"),
        Err(FormatError::Syntax(_))
    ));
}

fn hash(graph: &Graph, engine: &mut Engine) -> u64 {
    let id = lower(graph, engine);
    engine.programs().get(id).hash
}

#[test]
fn metadata_does_not_change_the_program() {
    let mut engine = Engine::default();
    let (mut graph, nodes) = tube();
    let before = hash(&graph, &mut engine);

    let meta = &mut graph.node_mut(nodes.circle).unwrap().meta;
    meta.position = [10.0, 20.0];
    meta.name = Some("Profile".into());
    graph.display = Some(nodes.circle);
    let moved = hash(&graph, &mut engine);
    assert_eq!(before, moved);

    graph.set(nodes.sweep, "sides", 8u32).unwrap();
    let changed = hash(&graph, &mut engine);
    assert_ne!(before, changed);
}

#[test]
fn a_node_can_be_previewed_on_its_own() {
    let (graph, nodes) = tube();
    let mut engine = Engine::default();
    let program = graph
        .lower_node(
            Context {
                registry: &registry(),
                programs: engine.programs(),
                graphs: &HashMap::new(),
            },
            nodes.sweep,
        )
        .unwrap();

    let names: Vec<_> = program.outputs.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(names, ["positions", "normals", "uvs", "indices"]);

    let id = engine.add_program(program);
    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);
    assert!(!engine.read::<Vec3>(&instance, "normals").unwrap().is_empty());
}

#[test]
fn graphs_use_each_other() {
    // The profile and the level of detail live in a graph of their own.
    let mut pipe = Graph::new("pipe");
    let circle = pipe.add("curve.circle");
    pipe.expose(circle, "radius", "radius", Some(0.25.into()))
        .unwrap();
    let sweep = pipe.add("curve.sweep");
    pipe.expose(sweep, "spine", "spine", None).unwrap();
    pipe.connect(circle, "circle", sweep, "profile").unwrap();
    pipe.set(sweep, "sides", 6u32).unwrap();
    pipe.add_output("positions", sweep, "positions").unwrap();

    let mut scene = Graph::new("scene");
    let curve = scene.add("curve.nurbs");
    scene
        .set(curve, "points", Value::F32(POINTS.as_flattened().to_vec()))
        .unwrap();
    scene
        .set(curve, "knots", Value::F32(clamped_uniform_knots(5)))
        .unwrap();
    let thin = scene.add_graph("pipe");
    scene.connect(curve, "segments", thin, "spine").unwrap();
    let thick = scene.add_graph("pipe");
    scene.connect(curve, "segments", thick, "spine").unwrap();
    scene.set(thick, "radius", 0.5f32).unwrap();
    scene.add_output("thin", thin, "positions").unwrap();
    scene.add_output("thick", thick, "positions").unwrap();
    let thick_node = thick;

    let mut library = Library::new();
    library.insert(scene);
    library.insert(pipe);

    let mut engine = Engine::default();
    let registry = registry();
    let programs = library
        .lower_into(&registry, engine.programs_mut())
        .unwrap();
    assert_eq!(engine.programs().len(), 2);

    let mut instance = engine.instantiate(programs["scene"]);
    // One curve, and a circle and a sweep for each of the two pipes.
    assert_eq!(engine.evaluate([&mut instance]), 5);
    let thin = engine.read::<Vec3>(&instance, "thin").unwrap();
    let thick = engine.read::<Vec3>(&instance, "thick").unwrap();
    assert_eq!(thin.len(), thick.len());
    assert_ne!(thin, thick);

    // Lowering again without a change adds nothing, an edit adds only the
    // graphs which do something else now.
    let again = library
        .lower_into(&registry, engine.programs_mut())
        .unwrap();
    assert_eq!(again, programs);
    assert_eq!(engine.programs().len(), 2);

    let scene = library.get_mut("scene").unwrap();
    scene.set(thick_node, "radius", 0.75f32).unwrap();
    let edited = library
        .lower_into(&registry, engine.programs_mut())
        .unwrap();
    assert_eq!(edited["pipe"], programs["pipe"]);
    assert_ne!(edited["scene"], programs["scene"]);
    assert_eq!(engine.programs().len(), 3);
}

#[test]
fn graphs_can_not_use_themselves() {
    let mut a = Graph::new("a");
    a.add_graph("b");
    let mut b = Graph::new("b");
    b.add_graph("a");
    let mut library = Library::new();
    library.insert(a);
    library.insert(b);

    let error = library
        .lower_into(&registry(), Engine::default().programs_mut())
        .unwrap_err();
    assert!(matches!(error, LibraryError::Cycle(_)));
}

#[test]
fn editing_keeps_the_graph_consistent() {
    let (mut graph, nodes) = tube();
    assert_eq!(
        graph.connect(nodes.sweep, "positions", nodes.curve, "points"),
        Err(EditError::Cycle)
    );
    assert_eq!(
        graph.connect(nodes.sweep, "positions", nodes.sweep, "spine"),
        Err(EditError::Cycle)
    );
    assert_eq!(
        graph.add_output("positions", nodes.sweep, "normals"),
        Err(EditError::DuplicateOutput("positions".into()))
    );

    // Removing a node takes its links and outputs with it.
    graph.remove_node(nodes.circle).unwrap();
    assert!(!graph.node(nodes.sweep).unwrap().inputs.contains_key("profile"));
    graph.remove_node(nodes.sweep).unwrap();
    assert!(graph.outputs().is_empty());
    assert_eq!(graph.display, None);

    // Ids are never handed out twice.
    let new = graph.add("curve.circle");
    assert!(new != nodes.circle && new != nodes.sweep);
}

#[test]
fn broken_graphs_say_where_they_are_broken() {
    let (graph, nodes) = tube();

    // Nothing connected where only a link can tell the type.
    let mut broken = graph.clone();
    broken.reset(nodes.sweep, "profile");
    assert_eq!(
        error(&broken),
        (
            Some(nodes.sweep),
            Some("profile".into()),
            LowerErrorKind::NeedsLink
        )
    );

    // A curve is a profile, a list of numbers is not.
    let mut broken = graph.clone();
    let knots = broken.add("curve.circle");
    broken.connect(knots, "circle", nodes.curve, "knots").unwrap();
    let (node, port, kind) = error(&broken);
    assert_eq!((node, port.as_deref()), (Some(nodes.curve), Some("knots")));
    assert!(matches!(kind, LowerErrorKind::Build(BuildError::Type { .. })));

    let mut broken = graph.clone();
    broken.connect(nodes.sweep, "positions", nodes.sweep, "profile").unwrap_err();
    let positions = broken.add("curve.sample");
    broken.connect(nodes.curve, "segments", positions, "segments").unwrap();
    broken.connect(positions, "positions", nodes.sweep, "profile").unwrap();
    let (node, port, kind) = error(&broken);
    assert_eq!((node, port.as_deref()), (Some(nodes.sweep), Some("profile")));
    assert!(matches!(kind, LowerErrorKind::Build(BuildError::Trait { .. })));

    // Values have to be whole elements of the right kind of number.
    let mut broken = graph.clone();
    broken.set(nodes.sweep, "sides", 6.0f32).unwrap();
    assert_eq!(
        error(&broken).2,
        LowerErrorKind::Value { expected: "u32" }
    );
    let mut broken = graph.clone();
    broken.set(nodes.curve, "points", Value::F32(vec![1.0; 7])).unwrap();
    assert_eq!(
        error(&broken).2,
        LowerErrorKind::Value { expected: "vec4" }
    );

    let mut broken = graph.clone();
    broken.set(nodes.sweep, "nope", 1u32).unwrap();
    assert_eq!(
        error(&broken),
        (
            Some(nodes.sweep),
            Some("nope".into()),
            LowerErrorKind::UnknownInput
        )
    );

    let mut broken = graph.clone();
    broken.connect(nodes.curve, "nope", nodes.sweep, "spine").unwrap();
    assert!(matches!(
        error(&broken).2,
        LowerErrorKind::UnknownOutput { .. }
    ));

    let mut broken = graph.clone();
    broken.expose(nodes.sweep, "sides", "radius", None).unwrap();
    assert_eq!(
        error(&broken).2,
        LowerErrorKind::ExposedTwice("radius".into())
    );

    let mut broken = graph.clone();
    let unknown = broken.add("someone_elses.node");
    broken.connect(unknown, "curve", nodes.sweep, "spine").unwrap();
    assert_eq!(
        error(&broken),
        (
            Some(unknown),
            None,
            LowerErrorKind::UnknownNode("someone_elses.node".into())
        )
    );

    let mut broken = graph.clone();
    let pipe = broken.add_graph("pipe");
    broken.connect(pipe, "positions", nodes.sweep, "spine").unwrap();
    assert_eq!(
        error(&broken).2,
        LowerErrorKind::UnknownGraph("pipe".into())
    );

    // A file can hold what editing would not allow.
    let json = graph.to_json();
    let mut broken: serde_json::Value = serde_json::from_str(&json).unwrap();
    broken["nodes"]["0"]["inputs"]["points"] =
        serde_json::json!({ "link": { "node": 2, "output": "positions" } });
    let broken = Graph::from_json(&broken.to_string()).unwrap();
    assert_eq!(error(&broken).2, LowerErrorKind::Cycle);

    let mut broken: serde_json::Value = serde_json::from_str(&json).unwrap();
    broken["nodes"]["2"]["inputs"]["spine"] =
        serde_json::json!({ "link": { "node": 99, "output": "segments" } });
    let broken = Graph::from_json(&broken.to_string()).unwrap();
    assert!(matches!(error(&broken).2, LowerErrorKind::MissingNode(_)));
}

#[test]
fn inputs_exposed_under_one_name_are_one_parameter() {
    let mut graph = Graph::new("rings");
    let a = graph.add("curve.circle");
    let b = graph.add("curve.circle");
    graph.expose(a, "radius", "size", Some(2.0.into())).unwrap();
    graph.expose(b, "radius", "size", None).unwrap();
    graph.add_output("a", a, "circle").unwrap();
    graph.add_output("b", b, "circle").unwrap();

    let mut engine = Engine::default();
    let id = lower(&graph, &mut engine);
    assert_eq!(engine.programs().get(id).params.len(), 1);

    let mut instance = engine.instantiate(id);
    // Both circles are the same one.
    assert_eq!(engine.evaluate([&mut instance]), 1);
    assert_eq!(
        engine.output_key(&instance, "a"),
        engine.output_key(&instance, "b")
    );
    assert!(matches!(
        graph.node(a).unwrap().inputs["radius"],
        Input::Exposed { .. }
    ));
}
