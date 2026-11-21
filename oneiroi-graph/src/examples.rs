//! Graphs which show what can be made, and which benchmarks measure.

use oneiroi_core::node::Card;

use crate::{Graph, GraphParam, Inline, NodeId, NodeKind, Value};

/// What an input of a node in [`fence`] gets.
enum Arg {
    Link(NodeId, &'static str),
    F32(Vec<f32>),
    U32(u32),
    /// A node right at the input.
    Inline(Inline),
}

fn link(node: NodeId, output: &'static str) -> Arg {
    Arg::Link(node, output)
}

fn f32s(values: &[f32]) -> Arg {
    Arg::F32(values.to_vec())
}

/// Adds node `id` at `position` with `inputs`.
fn add(graph: &mut Graph, id: &str, position: [f32; 2], inputs: Vec<(&str, Arg)>) -> NodeId {
    let node = graph.add(id);
    graph.node_mut(node).unwrap().meta.position = position;
    for (port, arg) in inputs {
        let done = match arg {
            Arg::Link(from, output) => graph.connect(from, output, node, port),
            Arg::F32(values) => graph.set(node, port, Value::F32(values)),
            Arg::U32(value) => graph.set(node, port, value),
            Arg::Inline(inline) => graph.set_inline(node, port, inline),
        };
        debug_assert!(done.is_ok(), "{id}.{port}: {done:?}");
    }
    node
}

/// What `value.math` does with two lists, by the index of its operation.
const ADD: u32 = 0;
const SUBTRACT: u32 = 1;
const MULTIPLY: u32 = 2;

fn math(graph: &mut Graph, position: [f32; 2], a: Arg, b: Arg, operation: u32) -> NodeId {
    add(graph, "value.math", position, vec![("a", a), ("b", b), ("operation", Arg::U32(operation))])
}

fn vectors(graph: &mut Graph, position: [f32; 2], x: Arg, y: Arg) -> NodeId {
    add(graph, "value.combine_xyz", position, vec![("x", x), ("y", y), ("z", f32s(&[0.0]))])
}

/// How big a profile is along a sweep, drawn right at the sweep: a straight
/// Curve 2D from `base` times the profile at the start down to a point.
fn spike(base: f32) -> Arg {
    let linear = 1u32;
    Arg::Inline(
        Inline::new("curve.curve_2d")
            .with("points", Value::F32(vec![0.0, base, 1.0, 0.0]))
            .with("interpolation", linear),
    )
}

/// A picket fence: rails, a band of rings, and pickets of a frame with a
/// ring in it and a rod ending in a spear, between two posts. It runs along
/// `x` and stands up along `y`, as long as its parameter `length`, as high
/// as `height`, with a picket every `spacing`.
///
/// Every part is made once, as a single shape, and put onto the points where
/// it goes by reference: a picket onto every point along the fence, a rail
/// onto three heights, a post onto both ends.
pub fn fence() -> Graph {
    let mut graph = Graph::new("fence");
    let input = graph.add_node(NodeKind::Input);
    graph.node_mut(input).unwrap().meta.position = [-500.0, 300.0];
    for (name, default) in [("length", 4.0), ("height", 1.6), ("spacing", 0.4)] {
        let param = GraphParam {
            ty: Some("f32".into()),
            card: Some(Card::One),
            default: Some(Value::F32(vec![default])),
            ..GraphParam::new(name)
        };
        let added = graph.add_param(param);
        debug_assert!(added.is_ok());
    }
    let (length, height, spacing) = (link(input, "length"), link(input, "height"), link(input, "spacing"));
    let g = &mut graph;

    // A rail along the whole fence, put at the bottom and at both edges of
    // the band.
    let ground = math(g, [-250., 0.], height.clone_link(), f32s(&[0.0, 1.0, 1.0]), MULTIPLY);
    let rails_y = math(g, [0., 0.], link(ground, "values"), f32s(&[0.12, -0.45, -0.15]), ADD);
    let rails_at = vectors(g, [250., 0.], f32s(&[0.0]), link(rails_y, "values"));
    let rail_to = vectors(g, [250., 120.], length.clone_link(), f32s(&[0.0]));
    let rail = add(g, "curve.line", [500., 120.], vec![("end", link(rail_to, "vectors"))]);
    let flat_bar = add(g, "curve.rectangle", [500., 240.], vec![("width", f32s(&[0.05])), ("height", f32s(&[0.03]))]);
    let rail = add(g, "curve.sweep", [750., 120.], vec![("spine", link(rail, "curve")), ("profile", link(flat_bar, "rectangle"))]);
    let rails = add(
        g,
        "instance.on_points",
        [1000., 0.],
        vec![("points", link(rails_at, "vectors")), ("shape", link(rail, "surface")), ("align", Arg::U32(2))],
    );

    // Where the rods of the pickets stand: every `spacing` along the
    // ground, from one `spacing` in to one before the end. The line goes
    // on for half of one, so that the last is on it however it rounds.
    let half = math(g, [-500., 420.], spacing.clone_link(), f32s(&[0.5]), MULTIPLY);
    let last = math(g, [-250., 300.], length.clone_link(), link(half, "values"), SUBTRACT);
    let ground_from = vectors(g, [0., 300.], spacing.clone_link(), f32s(&[0.0]));
    let ground_to = vectors(g, [0., 420.], link(last, "values"), f32s(&[0.0]));
    let ground = add(g, "curve.line", [250., 300.], vec![("start", link(ground_from, "vectors")), ("end", link(ground_to, "vectors"))]);
    let points = add(
        g,
        "curve.sample",
        [500., 300.],
        vec![("segments", link(ground, "curve")), ("spacing", spacing.clone_link())],
    );

    // And the same in the middle of the frames, from the bottom rail up to
    // the band, which a frame and its ring are put around.
    let middle = math(g, [-250., 720.], height.clone_link(), f32s(&[0.33]), SUBTRACT);
    let middle = math(g, [0., 720.], link(middle, "values"), f32s(&[0.5]), MULTIPLY);
    let middle_from = vectors(g, [250., 720.], spacing.clone_link(), link(middle, "values"));
    let middle_to = vectors(g, [250., 840.], link(last, "values"), link(middle, "values"));
    let middles = add(g, "curve.line", [500., 720.], vec![("start", link(middle_from, "vectors")), ("end", link(middle_to, "vectors"))]);
    let middles = add(
        g,
        "curve.sample",
        [750., 720.],
        vec![("segments", link(middles, "curve")), ("spacing", spacing.clone_link())],
    );

    // A picket: a frame with a ring in its middle, around the origin.
    let frame_height = math(g, [-250., 600.], height.clone_link(), f32s(&[0.57]), SUBTRACT);
    let outline = add(g, "curve.rectangle", [0., 600.], vec![("width", f32s(&[0.2])), ("height", link(frame_height, "values"))]);
    let square_bar = add(g, "curve.rectangle", [250., 960.], vec![("width", f32s(&[0.022])), ("height", f32s(&[0.022]))]);
    let frame = add(g, "curve.sweep", [500., 600.], vec![("spine", link(outline, "rectangle")), ("profile", link(square_bar, "rectangle"))]);
    let ring = add(g, "curve.circle", [0., 960.], vec![("radius", f32s(&[0.055]))]);
    let wire = add(g, "curve.circle", [250., 1080.], vec![("radius", f32s(&[0.01]))]);
    let ring = add(g, "curve.sweep", [500., 960.], vec![("spine", link(ring, "circle")), ("profile", link(wire, "circle"))]);

    // And a rod from the band up, and a spear on it: as thick as the rod
    // is two and a half times, down to a point.
    let rod_from = math(g, [-250., 1200.], height.clone_link(), f32s(&[-0.45]), ADD);
    let rod_to = math(g, [-250., 1320.], height.clone_link(), f32s(&[0.02]), ADD);
    let tip = math(g, [-250., 1440.], height.clone_link(), f32s(&[0.22]), ADD);
    let rod_from = vectors(g, [0., 1200.], f32s(&[0.0]), link(rod_from, "values"));
    let rod_to = vectors(g, [0., 1320.], f32s(&[0.0]), link(rod_to, "values"));
    let tip = vectors(g, [0., 1440.], f32s(&[0.0]), link(tip, "values"));
    let rod = add(g, "curve.line", [250., 1200.], vec![("start", link(rod_from, "vectors")), ("end", link(rod_to, "vectors"))]);
    let rod = add(g, "curve.sweep", [500., 1200.], vec![("spine", link(rod, "curve")), ("profile", link(square_bar, "rectangle"))]);
    let spear = add(g, "curve.line", [250., 1320.], vec![("start", link(rod_to, "vectors")), ("end", link(tip, "vectors"))]);
    let spear = add(
        g,
        "curve.sweep",
        [500., 1320.],
        vec![("spine", link(spear, "curve")), ("profile", link(square_bar, "rectangle")), ("scale", spike(2.5))],
    );

    // Every part of a picket on every point.
    let place = |g: &mut Graph, y: f32, points: NodeId, shape: NodeId| {
        add(
            g,
            "instance.on_points",
            [1000., y],
            vec![("points", link(points, "frames")), ("shape", link(shape, "surface")), ("align", Arg::U32(2))],
        )
    };
    let frames = place(g, 600., middles, frame);
    let rings = place(g, 960., middles, ring);
    let rods = place(g, 1200., points, rod);
    let spears = place(g, 1320., points, spear);

    // A ring in the band between every two rods, on a line which again goes
    // on for half of one.
    let half = math(g, [-250., 1500.], spacing.clone_link(), f32s(&[1.5]), MULTIPLY);
    let band_last = math(g, [0., 1620.], length.clone_link(), spacing.clone_link(), SUBTRACT);
    let band_y = math(g, [0., 1500.], height.clone_link(), f32s(&[0.3]), SUBTRACT);
    let band_from = vectors(g, [250., 1500.], link(half, "values"), link(band_y, "values"));
    let band_to = vectors(g, [250., 1620.], link(band_last, "values"), link(band_y, "values"));
    let band = add(g, "curve.line", [500., 1500.], vec![("start", link(band_from, "vectors")), ("end", link(band_to, "vectors"))]);
    let band = add(g, "curve.sample", [750., 1500.], vec![("segments", link(band, "curve")), ("spacing", spacing.clone_link())]);
    let loop_ = add(g, "curve.circle", [500., 1740.], vec![("radius", f32s(&[0.12]))]);
    let thin = add(g, "curve.circle", [500., 1860.], vec![("radius", f32s(&[0.012]))]);
    let loop_ = add(g, "curve.sweep", [750., 1740.], vec![("spine", link(loop_, "circle")), ("profile", link(thin, "circle"))]);
    let band = add(
        g,
        "instance.on_points",
        [1000., 1500.],
        vec![("points", link(band, "frames")), ("shape", link(loop_, "surface")), ("align", Arg::U32(2))],
    );

    // A post with a pointed cap, put at either end.
    let posts_x = math(g, [-250., 2100.], length.clone_link(), f32s(&[0.0, 1.0]), MULTIPLY);
    let posts_at = vectors(g, [0., 2100.], link(posts_x, "values"), f32s(&[0.0]));
    let post_top = math(g, [-250., 2220.], height.clone_link(), f32s(&[0.1]), ADD);
    let cap_top = math(g, [-250., 2340.], height.clone_link(), f32s(&[0.28]), ADD);
    let post_top = vectors(g, [0., 2220.], f32s(&[0.0]), link(post_top, "values"));
    let cap_top = vectors(g, [0., 2340.], f32s(&[0.0]), link(cap_top, "values"));
    let post = add(g, "curve.line", [250., 2220.], vec![("end", link(post_top, "vectors"))]);
    let cap = add(g, "curve.line", [250., 2340.], vec![("start", link(post_top, "vectors")), ("end", link(cap_top, "vectors"))]);
    let post_bar = add(g, "curve.rectangle", [250., 2460.], vec![("width", f32s(&[0.09])), ("height", f32s(&[0.09]))]);
    let post = add(g, "curve.sweep", [500., 2220.], vec![("spine", link(post, "curve")), ("profile", link(post_bar, "rectangle"))]);
    let cap = add(
        g,
        "curve.sweep",
        [500., 2340.],
        vec![("spine", link(cap, "curve")), ("profile", link(post_bar, "rectangle")), ("scale", spike(1.0))],
    );
    let at_ends = |g: &mut Graph, y: f32, shape: NodeId| {
        add(
            g,
            "instance.on_points",
            [1000., y],
            vec![("points", link(posts_at, "vectors")), ("shape", link(shape, "surface")), ("align", Arg::U32(2))],
        )
    };
    let posts = at_ends(g, 2220., post);
    let caps = at_ends(g, 2340., cap);

    let output = g.add_node(NodeKind::Output);
    g.node_mut(output).unwrap().meta.position = [1300., 900.];
    for (name, node, port) in [
        ("rails", rails, "instances"),
        ("posts", posts, "instances"),
        ("caps", caps, "instances"),
        ("frames", frames, "instances"),
        ("rings", rings, "instances"),
        ("rods", rods, "instances"),
        ("spears", spears, "instances"),
        ("band", band, "instances"),
    ] {
        let done = g.connect(node, port, output, name);
        debug_assert!(done.is_ok());
    }
    graph.about.doc = Some(
        "A picket fence between two posts: a frame with a ring and a rod with a spear every `spacing`, \
         a band of rings under the spears."
            .into(),
    );
    graph
}

impl Arg {
    /// The same link again, for another input.
    fn clone_link(&self) -> Arg {
        match self {
            Arg::Link(node, output) => Arg::Link(*node, output),
            Arg::F32(values) => Arg::F32(values.clone()),
            Arg::U32(value) => Arg::U32(*value),
            Arg::Inline(inline) => Arg::Inline(inline.clone()),
        }
    }
}
