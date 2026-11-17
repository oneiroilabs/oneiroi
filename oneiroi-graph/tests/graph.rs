use std::collections::HashMap;

use glam::Vec3;
use oneiroi_core::{
    exec::Engine,
    node::{Card, NodeRegistry},
    program::{BuildError, ProgramBuilder, ProgramId},
};
use oneiroi_graph::{
    Context, EditError, FormatError, Graph, Input, Library, LibraryError, LowerErrorKind, NodeId,
    Value,
};
use oneiroi_nodes::{
    curve::{NURBS, SWEEP, nurbs::clamped_uniform_knots},
    primitive::CIRCLE,
    surface::TESSELLATE,
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
    mesh: NodeId,
}

/// A circle swept along a curve and turned into a mesh, with the radius
/// exposed.
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

    let mesh = graph.add("surface.tessellate");
    graph.connect(sweep, "surface", mesh, "surface").unwrap();
    graph.set(mesh, "sides", 6u32).unwrap();

    graph.add_output("positions", mesh, "positions").unwrap();
    graph.add_output("indices", mesh, "indices").unwrap();
    graph.display = Some(mesh);
    (
        graph,
        Nodes {
            curve,
            circle,
            sweep,
            mesh,
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
    let surface = b.generic(&registry(), &SWEEP, &[spine, circle]).unwrap()[0];
    let mesh = b
        .generic(&registry(), &TESSELLATE, &[surface, rings, sides])
        .unwrap();
    b.output("positions", mesh[0]);
    let by_hand = engine.add_program(b.finish());

    let mut a = engine.instantiate(lowered);
    let mut b = engine.instantiate(by_hand);
    // The same nodes on the same values are the same work.
    assert_eq!(engine.evaluate([&mut a]), 4);
    assert_eq!(engine.evaluate([&mut b]), 0);
    assert_eq!(
        engine.output_key(&a, "positions"),
        engine.output_key(&b, "positions")
    );

    // 2 segments, each a grid with the default of 16 rings and 6 sides.
    let positions = engine.read::<Vec3>(&a, "positions").unwrap();
    assert_eq!(positions.len(), 2 * 17 * 7);

    engine.set_param(&mut a, "radius", &[0.5f32]).unwrap();
    assert_eq!(engine.evaluate([&mut a]), 3);
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

    graph.set(nodes.mesh, "sides", 8u32).unwrap();
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
            nodes.mesh,
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
    let mesh = pipe.add("surface.tessellate");
    pipe.connect(sweep, "surface", mesh, "surface").unwrap();
    pipe.set(mesh, "sides", 6u32).unwrap();
    pipe.add_output("positions", mesh, "positions").unwrap();

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
    // One curve, and a circle, a sweep and a mesh for each of the pipes.
    assert_eq!(engine.evaluate([&mut instance]), 7);
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

    // The scene of before the edit is of no use anymore.
    assert_eq!(engine.programs_mut().retain(edited.values().copied()), 1);
    assert!(!engine.programs().contains(programs["scene"]));
    assert!(engine.programs().contains(edited["pipe"]));
}

#[test]
fn a_broken_graph_does_not_stop_the_others() {
    let (tube, _) = tube();
    let mut broken = Graph::new("broken");
    let node = broken.add("someone_elses.node");
    broken.add_output("out", node, "out").unwrap();
    let mut user = Graph::new("user of broken");
    let used = user.add_graph("broken");
    user.add_output("out", used, "out").unwrap();

    let mut library = Library::new();
    library.insert(tube);
    library.insert(broken);
    library.insert(user);
    assert!(library.uses("user of broken", "broken") && !library.uses("broken", "tube"));
    assert!(library.uses("tube", "tube"));

    let mut engine = Engine::default();
    let (programs, errors) = library.lower_all(&registry(), engine.programs_mut());
    assert_eq!(programs.keys().collect::<Vec<_>>(), ["tube"]);
    // What is wrong is said once, for the graph it is wrong in.
    assert!(matches!(
        &errors[..],
        [LibraryError::Lower { graph, .. }] if graph == "broken"
    ));

    // A library survives its file, and so does a file of a single graph.
    assert_eq!(Library::from_json(&library.to_json()).unwrap(), library);
    let single = Library::from_json(&library.get("tube").unwrap().to_json()).unwrap();
    assert_eq!(single.len(), 1);
}

/// A tube along a curve, moved by its parameter `offset`.
fn moved_tube() -> Graph {
    let mut graph = Graph::new("moved tube");
    let curve = graph.add("curve.nurbs");
    graph
        .set(curve, "points", Value::F32(POINTS.as_flattened().to_vec()))
        .unwrap();
    graph
        .set(curve, "knots", Value::F32(clamped_uniform_knots(5)))
        .unwrap();
    let moved = graph.add("curve.translate");
    graph.connect(curve, "segments", moved, "segments").unwrap();
    graph.expose(moved, "offset", "offset", None).unwrap();
    let circle = graph.add("curve.circle");
    graph.expose(circle, "radius", "radius", Some(0.2.into())).unwrap();
    let sweep = graph.add("curve.sweep");
    graph.connect(moved, "segments", sweep, "spine").unwrap();
    graph.connect(circle, "circle", sweep, "profile").unwrap();
    graph.add_output("surface", sweep, "surface").unwrap();
    graph
}

/// The moved tube once for each of three offsets.
fn row_of_tubes() -> (Graph, NodeId, NodeId) {
    let mut graph = Graph::new("row");
    let offsets = graph.add("value.vec3");
    graph
        .set(offsets, "values", Value::F32(vec![0., 0., 0., 3., 0., 0., 6., 0., 0.]))
        .unwrap();
    let tubes = graph.add_graph("moved tube");
    graph.connect(offsets, "values", tubes, "offset").unwrap();
    assert!(graph.set_each(tubes, "offset", true));
    graph.add_output("surface", tubes, "surface").unwrap();
    (graph, offsets, tubes)
}

#[test]
fn a_graph_can_run_for_each_element() {
    let (row, offsets, tubes) = row_of_tubes();
    let mut library = Library::new();
    library.insert(moved_tube());
    library.insert(row.clone());

    let mut engine = Engine::default();
    let registry = registry();
    let programs = library.lower_into(&registry, engine.programs_mut()).unwrap();
    let mut instance = engine.instantiate(programs["row"]);
    engine.evaluate([&mut instance]);

    // Three tubes of two segments each, one after the other, and each
    // moved by its own offset.
    let patches = engine
        .read::<oneiroi_nodes::curve::TubePatch>(&instance, "surface")
        .unwrap();
    assert_eq!(patches.len(), 3 * 2);
    use oneiroi_nodes::surface::Surface;
    let start = |patch: usize| patches[patch].point(0.0, 0.0).0;
    assert!((start(2) - start(0) - Vec3::new(3.0, 0.0, 0.0)).length() < 1e-4);
    assert!((start(4) - start(0) - Vec3::new(6.0, 0.0, 0.0)).length() < 1e-4);
    assert!(engine.overflows().is_empty());

    // It survives its file.
    assert_eq!(Graph::from_json(&row.to_json()).unwrap(), row);
    assert!(row.to_json().contains("\"each\": true"));

    // Only a graph can run for each element, and of one input only.
    let mut broken = row.clone();
    let circle = broken.add("curve.circle");
    let numbers = broken.add("value.f32");
    broken.connect(numbers, "values", circle, "radius").unwrap();
    assert!(broken.set_each(circle, "radius", true));
    broken.add_output("circle", circle, "circle").unwrap();
    let error = |graph: Graph| {
        let mut library = Library::new();
        library.insert(moved_tube());
        library.insert(graph);
        match library.lower_into(&registry, Engine::default().programs_mut()) {
            Err(LibraryError::Lower { error, .. }) => error.kind,
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(error(broken), LowerErrorKind::EachOnNode);

    let mut broken = row.clone();
    let numbers = broken.add("value.f32");
    broken.connect(numbers, "values", tubes, "radius").unwrap();
    assert!(broken.set_each(tubes, "radius", true));
    assert_eq!(error(broken), LowerErrorKind::EachTwice);

    // Only a link can be for each element.
    let mut graph = row.clone();
    assert!(!graph.set_each(offsets, "values", true));
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
        graph.connect(nodes.mesh, "positions", nodes.curve, "points"),
        Err(EditError::Cycle)
    );
    assert_eq!(
        graph.connect(nodes.sweep, "surface", nodes.sweep, "spine"),
        Err(EditError::Cycle)
    );
    assert_eq!(
        graph.add_output("positions", nodes.mesh, "normals"),
        Err(EditError::DuplicateOutput("positions".into()))
    );

    // Removing a node takes its links and outputs with it.
    graph.remove_node(nodes.circle).unwrap();
    assert!(!graph.node(nodes.sweep).unwrap().inputs.contains_key("profile"));
    graph.remove_node(nodes.sweep).unwrap();
    assert!(!graph.node(nodes.mesh).unwrap().inputs.contains_key("surface"));
    graph.remove_node(nodes.mesh).unwrap();
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
    broken.connect(nodes.sweep, "surface", nodes.sweep, "profile").unwrap_err();
    let frames = broken.add("curve.sample");
    broken.connect(nodes.curve, "segments", frames, "segments").unwrap();
    broken.connect(frames, "frames", nodes.sweep, "profile").unwrap();
    let (node, port, kind) = error(&broken);
    assert_eq!((node, port.as_deref()), (Some(nodes.sweep), Some("profile")));
    assert!(matches!(kind, LowerErrorKind::Build(BuildError::Trait { .. })));

    // Values have to be whole elements of the right kind of number.
    let mut broken = graph.clone();
    broken.set(nodes.mesh, "sides", 6.0f32).unwrap();
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

    // A port of a single value takes no list.
    let mut broken = graph.clone();
    broken.set(nodes.mesh, "sides", Value::U32(vec![6, 8])).unwrap();
    assert_eq!(
        error(&broken),
        (Some(nodes.mesh), Some("sides".into()), LowerErrorKind::NotOne)
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
    broken.expose(nodes.mesh, "sides", "radius", None).unwrap();
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

#[test]
fn exposed_inputs_keep_what_they_carry() {
    let (mut graph, nodes) = tube();
    graph.expose(nodes.curve, "points", "points", None).unwrap();
    graph.expose(nodes.mesh, "sides", "sides", None).unwrap();

    let mut engine = Engine::default();
    let id = lower(&graph, &mut engine);
    let program = engine.programs().get(id);
    // The points of several curves keep their groups in a parameter which
    // is not shown, and the sides are a single number.
    let names: Vec<_> = program.visible_params().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["points", "radius", "sides"]);
    assert_eq!(program.params.len(), 4);
    let card = |name: &str| {
        let param = program.params.iter().find(|p| p.name == name).unwrap();
        program.slot(param.slot).card
    };
    assert_eq!(card("points"), Card::Grouped);
    assert_eq!(card("sides"), Card::One);

    // A graph used as a node shows only those three.
    let mut outer = Graph::new("outer");
    let tube = outer.add_graph("tube");
    outer.set(tube, "points#groups", Value::U32(vec![0, 5])).unwrap();
    outer.add_output("positions", tube, "positions").unwrap();
    let graphs = HashMap::from([("tube".to_string(), id)]);
    let error = outer
        .lower(Context {
            registry: &registry(),
            programs: engine.programs(),
            graphs: &graphs,
        })
        .unwrap_err();
    assert_eq!(error.kind, LowerErrorKind::UnknownInput);
}
