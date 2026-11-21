use std::collections::HashMap;

use glam::Vec3;
use oneiroi_core::{
    exec::Engine,
    node::{Card, NodeRegistry},
    program::{BuildError, ProgramBuilder, ProgramId},
};
use oneiroi_graph::{
    About, Context, EditError, FormatError, Graph, Inline, Input, Library, LibraryError, LowerErrorKind, NodeId,
    Note, Value,
};
use oneiroi_nodes::{
    curve::{ControlPoint, NURBS, SWEEP, nurbs::clamped_uniform_knots, swept},
    primitive::CIRCLE,
    surface,
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

    let mesh = graph.add("surface.mesh");
    graph.connect(sweep, "surface", mesh, "surface").unwrap();
    graph.set(mesh, "tolerance", 0.01f32).unwrap();

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
    let points = b.constant(&POINTS.map(ControlPoint::from));
    let knots = b.constant(&clamped_uniform_knots(5));
    let radius = b.param_one_of::<f32>("radius", 0.25);
    let tolerance = b.constant::<f32>(&[0.01]);
    let spine = b.node(&NURBS, &[points, knots]).unwrap()[0];
    let circle = b.node(&CIRCLE, &[radius]).unwrap()[0];
    let surface = swept(&mut b, &registry(), spine, circle).unwrap();
    let mesh = surface::mesh(&mut b, &registry(), surface, tolerance).unwrap();
    b.output("positions", mesh[0].1);
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

    assert!(!engine.read::<Vec3>(&a, "positions").unwrap().is_empty());

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

    let newer = json.replacen("\"version\": 2", "\"version\": 3", 1);
    assert!(matches!(
        Graph::from_json(&newer),
        Err(FormatError::Version(3))
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
    meta.set_input_note("radius", Note {
        label: Some("Thickness".into()),
        doc: Some("How thick the tube is.".into()),
    });
    graph.display = Some(nodes.circle);
    graph.about.doc = Some("A tube along a curve.".into());
    graph.about.set_param_note("radius", Note {
        label: None,
        doc: Some("Half the thickness.".into()),
    });
    let moved = hash(&graph, &mut engine);
    assert_eq!(before, moved);

    graph.set(nodes.mesh, "tolerance", 0.02f32).unwrap();
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
    let mesh = pipe.add("surface.mesh");
    pipe.connect(sweep, "surface", mesh, "surface").unwrap();
    pipe.set(mesh, "tolerance", 0.01f32).unwrap();
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
    // The thicker one bends more around, so it needs more of the same
    // tolerance.
    assert!(thin.len() < thick.len());

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
    // Open, so that every patch is one along the curve.
    graph.set(sweep, "ends", 1u32).unwrap();
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
    broken.set(nodes.mesh, "tolerance", 6u32).unwrap();
    assert_eq!(
        error(&broken).2,
        LowerErrorKind::Value { expected: "f32" }
    );
    let mut broken = graph.clone();
    broken.set(nodes.curve, "points", Value::F32(vec![1.0; 7])).unwrap();
    assert_eq!(
        error(&broken).2,
        LowerErrorKind::Value { expected: "control_point" }
    );

    // A port of a single value takes no list.
    let mut broken = graph.clone();
    broken.set(nodes.mesh, "tolerance", Value::F32(vec![0.1, 0.2])).unwrap();
    assert_eq!(
        error(&broken),
        (Some(nodes.mesh), Some("tolerance".into()), LowerErrorKind::NotOne)
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
    let frames = broken.add("curve.sample");
    broken.connect(nodes.curve, "segments", frames, "segments").unwrap();
    // The parameter has the type of the radius it was linked to first.
    broken.expose(frames, "count", "radius", None).unwrap();
    broken.add_output("frames", frames, "frames").unwrap();
    let (node, port, kind) = error(&broken);
    assert_eq!((node, port.as_deref()), (Some(frames), Some("count")));
    assert!(matches!(kind, LowerErrorKind::Build(BuildError::Type { .. })), "{kind:?}");

    // A parameter nothing tells the type of can not be taken.
    let mut broken = graph.clone();
    broken.add_param(oneiroi_graph::GraphParam::new("anything")).unwrap();
    let input = broken.input_node().unwrap();
    broken.connect(input, "anything", nodes.sweep, "profile").unwrap();
    assert_eq!(error(&broken).2, LowerErrorKind::ParamType("anything".into()));

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
    let id = |node: NodeId| node.raw().to_string();
    broken["nodes"][id(nodes.curve)]["inputs"]["points"] =
        serde_json::json!({ "link": { "node": nodes.mesh.raw(), "output": "positions" } });
    let broken = Graph::from_json(&broken.to_string()).unwrap();
    assert_eq!(error(&broken).2, LowerErrorKind::Cycle);

    let mut broken: serde_json::Value = serde_json::from_str(&json).unwrap();
    broken["nodes"][id(nodes.sweep)]["inputs"]["spine"] =
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
    assert_eq!(graph.exposed_as(a, "radius"), Some("size"));
    assert_eq!(graph.params().len(), 1);
}

#[test]
fn exposed_inputs_keep_what_they_carry() {
    let (mut graph, nodes) = tube();
    graph.expose(nodes.curve, "points", "points", None).unwrap();
    graph.expose(nodes.mesh, "tolerance", "tolerance", None).unwrap();

    let mut engine = Engine::default();
    let id = lower(&graph, &mut engine);
    let program = engine.programs().get(id);
    // The points of the curve are a list, and the tolerance is a single
    // number.
    let names: Vec<_> = program.visible_params().map(|p| p.name.as_str()).collect();
    // In the order they were made.
    assert_eq!(names, ["radius", "points", "tolerance"]);
    assert_eq!(program.params.len(), 3);
    let card = |name: &str| {
        let param = program.params.iter().find(|p| p.name == name).unwrap();
        program.slot(param.slot).card
    };
    assert_eq!(card("points"), Card::Many);
    assert_eq!(card("tolerance"), Card::One);

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

#[test]
fn notes_survive_the_file_and_empty_ones_are_left_out() {
    let (mut graph, nodes) = tube();
    let empty = graph.to_json();
    assert!(!empty.contains("\"about\"") && !empty.contains("\"inputs\": {}"));

    let meta = &mut graph.node_mut(nodes.sweep).unwrap().meta;
    meta.set_output_note("surface", Note {
        label: Some("Tube".into()),
        doc: None,
    });
    meta.set_input_note("spine", Note::default());
    assert!(meta.inputs.is_empty(), "an empty note is no note");
    graph.about.doc = Some("A *tube*.".into());
    graph.about.set_output_note("mesh", Note {
        label: None,
        doc: Some("Triangles of it.".into()),
    });

    let json = graph.to_json();
    assert_eq!(Graph::from_json(&json).unwrap(), graph);
    // Files of before have none of it, and still load.
    assert_eq!(Graph::from_json(&empty).unwrap().about, About::default());

    graph.about.set_output_note("mesh", Note::default());
    graph.about.doc = None;
    assert!(graph.about.is_empty());
}

#[test]
fn a_parameter_is_described_like_the_input_it_comes_from() {
    let mut graph = Graph::new("g");
    let a = graph.add("value.f32");
    let b = graph.add("value.f32");
    graph.expose(b, "values", "size", None).unwrap();
    graph.expose(a, "values", "size", None).unwrap();
    assert_eq!(graph.param_source("size"), Some((a, "values")));
    assert_eq!(graph.param_source("other"), None);
}

/// Instances linked to a port which takes the shape are copied into it in
/// front of the port, and where they are taken as they are nothing is
/// copied.
#[test]
fn instances_are_copied_where_the_shape_is_needed() {
    let mut graph = Graph::new("instances");
    let curve = graph.add("curve.nurbs");
    graph.set(curve, "points", Value::F32(POINTS.as_flattened().to_vec())).unwrap();
    let sample = graph.add("curve.sample");
    graph.set(sample, "count", 3u32).unwrap();
    graph.connect(curve, "segments", sample, "segments").unwrap();
    let placed = graph.add("instance.on_points");
    graph.connect(sample, "frames", placed, "points").unwrap();
    graph.connect(curve, "segments", placed, "shape").unwrap();
    // Instances of instances stay instances.
    let moved = graph.add("instance.transform");
    graph.connect(placed, "instances", moved, "shape").unwrap();
    // Translate takes curves, so these are copied.
    let translate = graph.add("curve.translate");
    graph.connect(moved, "instances", translate, "segments").unwrap();
    graph.add_output("segments", translate, "segments").unwrap();
    graph.add_output("instances", moved, "instances").unwrap();

    let mut engine = Engine::default();
    let id = lower(&graph, &mut engine);
    let program = engine.programs().get(id);
    let ids: Vec<_> = program
        .ops
        .iter()
        .filter_map(|op| match op.kind {
            oneiroi_core::program::OpKind::Node(node) => Some(node.id),
            _ => None,
        })
        .collect();
    assert_eq!(
        ids.iter().filter(|id| id.starts_with("instance.realize")).count(),
        1,
        "{ids:?}"
    );

    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);
    let segments = POINTS.len() - 3;
    let copies = engine.read::<oneiroi_nodes::curve::nurbs::CubicNurbsSegmentCache>(&instance, "segments");
    assert_eq!(copies.map(<[_]>::len), Some(3 * segments));
    let rows = engine.read::<oneiroi_core::types::InstanceRow>(&instance, "instances");
    assert_eq!(rows.map(<[_]>::len), Some(3));
}

#[test]
fn circles_and_rectangles_are_curves_where_curves_are_needed() {
    use oneiroi_nodes::curve::{SweptPatch, nurbs::CubicNurbsSegmentCache};

    // A rectangular bar bent into a ring: the circle becomes the spine,
    // the rectangle the profile, both as curves.
    let mut graph = Graph::new("ring");
    let circle = graph.add("curve.circle");
    graph.set(circle, "radius", 2.0f32).unwrap();
    let rectangle = graph.add("curve.rectangle");
    graph.set(rectangle, "width", 0.2f32).unwrap();
    graph.set(rectangle, "height", 0.1f32).unwrap();
    let sweep = graph.add("curve.sweep");
    graph.connect(circle, "circle", sweep, "spine").unwrap();
    graph.connect(rectangle, "rectangle", sweep, "profile").unwrap();
    graph.add_output("surface", sweep, "surface").unwrap();
    // A line on its own, which needs nothing in between.
    let line = graph.add("curve.line");
    graph.add_output("line", line, "curve").unwrap();

    let mut engine = Engine::default();
    let id = lower(&graph, &mut engine);
    let program = engine.programs().get(id);
    let conversions: Vec<_> = program
        .ops
        .iter()
        .filter_map(|op| match op.kind {
            oneiroi_core::program::OpKind::Node(node) if node.id.starts_with("curve.to_curve") => Some(node.id),
            _ => None,
        })
        .collect();
    assert_eq!(conversions, ["curve.to_curve<circle>", "curve.to_curve<rectangle>"]);

    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);
    // 4 quarters of the ring, each swept by the 4 sides of the bar, and
    // caps which are not there: the ring is closed.
    assert_eq!(engine.read::<SweptPatch>(&instance, "surface").map(<[_]>::len), Some(16 + 8));
    let line = engine.read::<CubicNurbsSegmentCache>(&instance, "line").unwrap();
    assert_eq!(line.len(), 1);
    assert!((line[0].length() - 1.0).abs() < 1e-5);
}

#[test]
fn a_file_of_before_gets_input_and_output_nodes() {
    // Format 1: the radius of two circles is exposed as one parameter.
    let json = r#"{
        "version": 1,
        "name": "old",
        "nodes": {
            "0": { "kind": { "node": "curve.circle" }, "inputs": { "radius": { "exposed": { "name": "size" } } },
                   "meta": { "position": [100.0, 50.0] } },
            "1": { "kind": { "node": "curve.circle" }, "inputs": { "radius": { "exposed": { "name": "size", "value": { "f32": [2.0] } } } } },
            "2": { "kind": { "node": "curve.line" }, "inputs": { "end": { "exposed": { "name": "end" } } } }
        },
        "next_id": 3,
        "outputs": [{ "name": "circles", "node": 0, "output": "circle" }]
    }"#;
    let graph = Graph::from_json(json).unwrap();
    let input = graph.input_node().unwrap();
    let output = graph.output_node().unwrap();
    assert_eq!((input.raw(), output.raw()), (3, 4));
    let names: Vec<_> = graph.params().iter().map(|param| param.name.as_str()).collect();
    assert_eq!(names, ["size", "end"]);
    assert_eq!(graph.param("size").unwrap().default, Some(Value::F32(vec![2.0])));
    assert_eq!(graph.exposed_as(NodeId::from(0), "radius"), Some("size"));
    assert_eq!(graph.exposed_as(NodeId::from(1), "radius"), Some("size"));
    // Left of everything and right of it.
    assert!(graph.node(input).unwrap().meta.position[0] < 0.0);
    assert!(graph.node(output).unwrap().meta.position[0] > 100.0);
    assert_eq!(Graph::from_json(&graph.to_json()).unwrap(), graph);

    let mut engine = Engine::default();
    let id = lower(&graph, &mut engine);
    let names: Vec<_> = engine.programs().get(id).visible_params().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["size", "end"]);
}

#[test]
fn parameters_have_a_type_and_an_order_of_their_own() {
    use oneiroi_core::node::Card;
    use oneiroi_graph::{GraphParam, NodeKind};

    let mut graph = Graph::new("params");
    let input = graph.add_node(NodeKind::Input);
    let circle = graph.add("curve.circle");
    // Linked to nothing, but with a type it is a parameter all the same.
    graph
        .add_param(GraphParam {
            ty: Some("u32".into()),
            card: Some(Card::One),
            default: Some(Value::U32(vec![7])),
            ..GraphParam::new("count")
        })
        .unwrap();
    graph.add_param(GraphParam::new("radius")).unwrap();
    assert!(graph.add_param(GraphParam::new("radius")).is_err());
    graph.connect(input, "radius", circle, "radius").unwrap();
    graph.add_output("circle", circle, "circle").unwrap();

    let mut engine = Engine::default();
    let id = lower(&graph, &mut engine);
    let program = engine.programs().get(id);
    let names: Vec<_> = program.visible_params().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["count", "radius"]);
    assert_eq!(program.params[0].default.as_ref(), &[7]);

    graph.move_param("radius", 0).unwrap();
    graph.rename_param("radius", "size").unwrap();
    assert!(graph.rename_param("size", "count").is_err());
    assert_eq!(graph.exposed_as(circle, "radius"), Some("size"));
    let id = lower(&graph, &mut engine);
    let names: Vec<_> = engine.programs().get(id).visible_params().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["size", "count"]);

    // Gone with its links: the circle has its own radius again.
    graph.remove_param("size").unwrap();
    assert_eq!(graph.exposed_as(circle, "radius"), None);
    assert!(graph.node(circle).unwrap().inputs.is_empty());
}

#[test]
fn the_output_node_holds_the_outputs() {
    use oneiroi_graph::{EditError, NodeKind};

    let (mut graph, nodes) = tube();
    let output = graph.add_node(NodeKind::Output);
    assert_eq!(graph.add_node(NodeKind::Output), output, "there is only one");
    // Linking into it adds an output, linking again replaces it.
    graph.connect(nodes.sweep, "surface", output, "surface").unwrap();
    graph.connect(nodes.circle, "circle", output, "surface").unwrap();
    let names: Vec<_> = graph.outputs().iter().map(|output| output.name.as_str()).collect();
    assert_eq!(names, ["positions", "indices", "surface"]);
    assert_eq!(graph.outputs()[2].node, nodes.circle);
    assert_eq!(graph.connect(output, "surface", nodes.sweep, "spine"), Err(EditError::FromOutputNode));

    graph.rename_output("surface", "profile").unwrap();
    graph.move_output("profile", 0);
    let names: Vec<_> = graph.outputs().iter().map(|output| output.name.as_str()).collect();
    assert_eq!(names, ["profile", "positions", "indices"]);
    graph.reset(output, "indices");

    // Previewed, it shows every output of the graph.
    let program = graph
        .lower_node(
            Context {
                registry: &registry(),
                programs: Engine::default().programs(),
                graphs: &HashMap::new(),
            },
            output,
        )
        .unwrap();
    let names: Vec<_> = program.outputs.iter().map(|output| output.name.as_str()).collect();
    assert_eq!(names, ["profile", "positions"]);

    graph.remove_node(output);
    assert!(graph.outputs().is_empty());
}

#[test]
fn the_fence_stands() {
    use oneiroi_core::types::InstanceRow;
    use oneiroi_nodes::curve::SweptPatch;

    let fence = oneiroi_graph::examples::fence();
    assert_eq!(Graph::from_json(&fence.to_json()).unwrap(), fence);
    let mut engine = Engine::default();
    let id = lower(&fence, &mut engine);
    let names: Vec<_> = engine.programs().get(id).visible_params().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["length", "height", "spacing"]);

    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);
    fn rows(engine: &Engine, instance: &oneiroi_core::exec::Instance, name: &str) -> usize {
        engine.read::<InstanceRow>(instance, name).map_or(0, <[_]>::len)
    }
    // A rail of 4 sides and 2 caps of 4 each, put at 3 heights; a post the
    // same, and its pointed cap, the tip of which is not there, put at
    // both ends.
    let shape = |name| {
        let (elements, _) = engine.prototype_keys(&instance, name).unwrap();
        engine.store().read::<SweptPatch>(elements).unwrap().to_vec()
    };
    assert_eq!((rows(&engine, &instance, "rails"), shape("rails").len()), (3, 4 + 8));
    assert_eq!((rows(&engine, &instance, "posts"), shape("posts").len()), (2, 4 + 8));
    let tips: Vec<_> = shape("caps").iter().map(SweptPatch::kind).collect();
    assert_eq!((rows(&engine, &instance, "caps"), tips.len()), (2, 4 + 8));
    assert_eq!(tips.iter().filter(|kind| **kind == oneiroi_nodes::curve::kind::NONE).count(), 4);
    // From one spacing in to one before the end: 0.4, 0.8 up to 3.6.
    let pickets = rows(&engine, &instance, "frames");
    assert_eq!(pickets, 9);
    assert_eq!(rows(&engine, &instance, "rings"), pickets);
    assert_eq!(rows(&engine, &instance, "rods"), pickets);
    assert_eq!(rows(&engine, &instance, "spears"), pickets);
    // A ring between every two rods.
    assert_eq!(rows(&engine, &instance, "band"), pickets - 1);

    // Longer, it has more pickets of the same shape.
    engine.set_param(&mut instance, "length", &[8.0f32]).unwrap();
    engine.evaluate([&mut instance]);
    assert_eq!(rows(&engine, &instance, "frames"), 19);
}

#[test]
fn a_curve_drawn_at_an_input_is_the_node_it_would_be_linked_to() {
    let spike = Inline::new("curve.curve_2d").with("points", Value::F32(vec![0.0, 1.0, 1.0, 0.2]));
    let (mut graph, nodes) = tube();
    graph.set_inline(nodes.sweep, "scale", spike.clone()).unwrap();
    graph.add_output("surface", nodes.sweep, "surface").unwrap();
    assert_eq!(Graph::from_json(&graph.to_json()).unwrap(), graph);

    // The same curve as a node of the graph, linked.
    let (mut linked, nodes) = tube();
    let curve = linked.add("curve.curve_2d");
    linked.set(curve, "points", spike.inputs["points"].clone()).unwrap();
    linked.connect(curve, "curve", nodes.sweep, "scale").unwrap();
    linked.add_output("surface", nodes.sweep, "surface").unwrap();

    let mut engine = Engine::default();
    let (a, b) = (lower(&graph, &mut engine), lower(&linked, &mut engine));
    let (mut a, mut b) = (engine.instantiate(a), engine.instantiate(b));
    engine.evaluate([&mut a, &mut b]);
    assert_eq!(engine.output_key(&a, "surface"), engine.output_key(&b, "surface"));
    // It is a spike now, not a tube: the end of it is thinner.
    let (mut plain, _) = tube();
    plain.add_output("surface", nodes.sweep, "surface").unwrap();
    let plain = lower(&plain, &mut engine);
    let mut plain = engine.instantiate(plain);
    engine.evaluate([&mut plain]);
    assert_ne!(engine.output_key(&a, "surface"), engine.output_key(&plain, "surface"));

    // A node nobody knows is said to be one where it is used.
    let (mut broken, nodes) = tube();
    broken.set_inline(nodes.sweep, "scale", Inline::new("curve.nope")).unwrap();
    assert_eq!(error(&broken).2, LowerErrorKind::UnknownNode("curve.nope".into()));
}
