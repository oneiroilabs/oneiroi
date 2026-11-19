//! What the editor edits: one graph, and the engine which previews it.
//!
//! Nothing in here knows about the user interface, so all of it can be
//! tested without opening a window.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use glam::Vec3;
use oneiroi_core::{
    exec::{Engine, Instance, Overflow},
    node::{Card, Drawable, NodeRegistry, PortType, Topology},
    store::ContentKey,
    types::{DataType, InstanceRow, Scalar, Trait, TypeInfo},
};
use oneiroi_wgpu::{DeferredExecutor, orbit::OrbitCamera};
use oneiroi_core::program::ProgramId;
use oneiroi_graph::{
    About, Context, Graph, Input, Library, LibraryError, LowerError, LowerErrorKind, NodeId, NodeKind,
    Note, Value,
};
use oneiroi_nodes::{
    curve::nurbs::clamped_uniform_knots,
    meta::{Hue, MetaRegistry, label_of, markdown},
};

/// How many 32-bit words of results the preview keeps around.
const BUDGET_WORDS: u64 = 1 << 26;

/// What a pin carries, as far as it is known.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PinType {
    Exact(TypeInfo),
    Trait(Trait),
    /// The node is not registered, so nothing is known about its pins.
    Unknown,
}

impl PinType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Exact(ty) => ty.name,
            Self::Trait(tr) => tr.name,
            Self::Unknown => "?",
        }
    }
}

impl From<PortType> for PinType {
    fn from(ty: PortType) -> Self {
        match ty {
            PortType::Exact(ty) => Self::Exact(ty),
            PortType::Trait(tr) => Self::Trait(tr),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PortView {
    pub name: String,
    /// What it is called where it is shown.
    pub label: String,
    /// How the node uses it, as markdown.
    pub doc: String,
    pub ty: PinType,
    /// How many elements it carries.
    pub card: Card,
    /// The value of an input nothing is set on.
    pub default: Vec<u32>,
}

/// Which side of a node a port is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Input,
    Output,
}

/// A node as it is shown.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeView {
    pub title: String,
    /// What the node does, as markdown.
    pub doc: String,
    pub inputs: Vec<PortView>,
    pub outputs: Vec<PortView>,
    /// Whether the registry knows the node.
    pub known: bool,
}

/// What the preview shows of the displayed node.
#[derive(Debug, Clone, Copy)]
pub enum Shown {
    Nothing,
    /// A value the GPU draws as it is, without a mesh in between. For
    /// instances, `key` is their rows and `shape` the elements and offsets
    /// of the shape they refer to.
    Drawn {
        key: ContentKey,
        drawable: Drawable,
        shape: Option<(ContentKey, ContentKey)>,
    },
    /// A triangle mesh somebody made on purpose.
    Mesh {
        positions: ContentKey,
        indices: ContentKey,
        normals: Option<ContentKey>,
    },
}

impl PartialEq for Shown {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nothing, Self::Nothing) => true,
            (
                Self::Drawn { key: a, drawable: x, shape: c },
                Self::Drawn { key: b, drawable: y, shape: z },
            ) => a == b && x.ty == y.ty && c == z,
            (
                Self::Mesh { positions: a, indices: b, normals: c },
                Self::Mesh { positions: x, indices: y, normals: z },
            ) => (a, b, c) == (x, y, z),
            _ => false,
        }
    }
}

/// A triangle mesh of the displayed node.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewMesh {
    pub positions: Vec<Vec3>,
    pub normals: Option<Vec<Vec3>>,
    pub indices: Vec<u32>,
}

/// The part of a document which computes: the engine and what it is asked
/// to show.
///
/// It is shared between the application, which plans, and the preview,
/// which has the graphics device and therefore is where things run.
pub struct Session {
    engine: Engine<DeferredExecutor>,
    instance: Option<Instance>,
    shown: Shown,
    /// Counts up every time something else is shown.
    generation: u64,
    /// The camera which was aimed at what is shown, and the request it
    /// was aimed for.
    pub framed: Option<(u64, OrbitCamera)>,
    /// How many nodes ran for what is shown since it was last planned.
    ran: usize,
    /// How many of those ran on the GPU.
    ran_on_gpu: usize,
    /// What the preview draws surfaces with, once it drew one.
    pub drawn_with: Option<&'static str>,
    /// How many triangles the preview drew the last time they were counted.
    pub triangles: Option<u32>,
    /// How many more frames the preview wants, to find the detail of its
    /// budget of triangles.
    pub settling: u32,
    /// The graphs which ran for fewer elements than they were given.
    overflows: Vec<Overflow>,
    /// The shape of the instances which are shown.
    summary: Option<ShapeSummary>,
}

/// What drawing instances needs to know of the shape they refer to, see
/// [`Session::shape_summary`].
#[derive(Debug, Clone)]
pub struct ShapeSummary {
    /// The keys of the shape and of its offsets.
    key: (ContentKey, ContentKey),
    /// Center and radius around each group, a negative radius for an
    /// empty one.
    pub spheres: Vec<[f32; 4]>,
    pub elements: u32,
    /// How many elements the longest group has.
    pub slots: u32,
}

impl Session {
    fn new() -> Self {
        Self {
            engine: Engine::new(DeferredExecutor::default()),
            instance: None,
            shown: Shown::Nothing,
            generation: 0,
            framed: None,
            ran: 0,
            ran_on_gpu: 0,
            drawn_with: None,
            triangles: None,
            settling: 0,
            overflows: Vec::new(),
            summary: None,
        }
    }

    pub fn engine_mut(&mut self) -> &mut Engine<DeferredExecutor> {
        &mut self.engine
    }

    pub fn shown(&self) -> Shown {
        self.shown
    }

    /// Different whenever something else is shown.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    fn show(&mut self, shown: Shown) {
        if self.shown != shown {
            self.shown = shown;
            self.generation += 1;
        }
    }

    /// Runs what was planned. On the GPU once a device is attached to the
    /// executor, on the CPU before that.
    pub fn run(&mut self) -> usize {
        if self.engine.pending() == 0 {
            return 0;
        }
        let on_gpu = |engine: &Engine<DeferredExecutor>| {
            engine.executor().gpu().map_or(0, |gpu| gpu.stats().gpu)
        };
        let before = on_gpu(&self.engine);
        let ran = self.engine.flush();
        self.ran += ran;
        self.ran_on_gpu += on_gpu(&self.engine) - before;
        self.overflows = self.engine.overflows().to_vec();
        if ran > 0 {
            self.engine.end_frame(BUDGET_WORDS);
        }
        ran
    }

    /// The mesh which is shown, brought over from the GPU if need be.
    pub fn mesh(&mut self) -> Option<PreviewMesh> {
        let Shown::Mesh { positions, indices, normals } = self.shown else {
            return None;
        };
        self.run();
        let positions = self.engine.fetch_key::<Vec3>(positions)?.to_vec();
        let indices = self.engine.fetch_key::<u32>(indices)?.to_vec();
        let normals = normals
            .and_then(|key| self.engine.fetch_key::<Vec3>(key))
            .map(<[Vec3]>::to_vec);
        (!positions.is_empty() && !indices.is_empty()).then_some(PreviewMesh {
            positions,
            normals,
            indices,
        })
    }

    /// What drawing instances needs to know of their shape: a sphere
    /// around each of its groups, how many elements it has and how many
    /// its longest group has. The shape is brought over once, and its
    /// summary kept while it stays the same.
    pub fn shape_summary(&mut self, drawable: &Drawable, elements: ContentKey, groups: ContentKey) -> Option<&ShapeSummary> {
        let key = (elements, groups);
        if self.summary.as_ref().is_none_or(|summary| summary.key != key) {
            let shape = drawable.ty.prototype?;
            let words = self.engine.fetch_key::<u32>(elements)?.to_vec();
            let offsets = self.engine.fetch_key::<u32>(groups)?.to_vec();
            let len = words.len() / shape.words as usize;
            let ranges: Vec<std::ops::Range<usize>> = match offsets.is_empty() {
                true => vec![0..len],
                false => offsets
                    .windows(2)
                    .map(|pair| (pair[0] as usize).min(len)..(pair[1] as usize).min(len))
                    .collect(),
            };
            let at = |index: usize| index * shape.words as usize;
            let spheres = ranges
                .iter()
                .map(|range| match (drawable.bounds)(&words[at(range.start)..at(range.end)]) {
                    Some((center, radius)) => [center[0], center[1], center[2], radius],
                    None => [0.0, 0.0, 0.0, -1.0],
                })
                .collect();
            self.summary = Some(ShapeSummary {
                key,
                spheres,
                elements: len as u32,
                slots: ranges.iter().map(|range| range.len() as u32).max().unwrap_or(0),
            });
        }
        self.summary.as_ref()
    }

    /// A sphere around all instances: the one around each group of the
    /// shape, put where every instance is, and a sphere around those.
    fn instances_bounds(
        &mut self,
        rows: ContentKey,
        drawable: &Drawable,
        elements: ContentKey,
        groups: ContentKey,
    ) -> Option<(Vec3, f32)> {
        let spheres = self.shape_summary(drawable, elements, groups)?.spheres.clone();
        let rows = self.engine.fetch_key::<InstanceRow>(rows)?;
        let (mut min, mut max) = (Vec3::MAX, Vec3::MIN);
        for row in rows {
            let Some([x, y, z, radius]) = spheres.get(row.group as usize).copied() else {
                continue;
            };
            if radius < 0.0 {
                continue;
            }
            let center = row.transform.point(Vec3::new(x, y, z));
            let radius = radius * row.transform.scale.abs();
            min = min.min(center - radius);
            max = max.max(center + radius);
        }
        (min.x <= max.x).then(|| ((min + max) * 0.5, (max - min).length() * 0.5))
    }

    /// Center and radius of a sphere around what is shown.
    pub fn bounds(&mut self) -> Option<(Vec3, f32)> {
        self.run();
        match self.shown {
            Shown::Nothing => None,
            Shown::Drawn { key, drawable, shape: None } => {
                let (center, radius) = (drawable.bounds)(self.engine.fetch_key::<u32>(key)?)?;
                Some((center.into(), radius))
            }
            Shown::Drawn { key, drawable, shape: Some((elements, groups)) } => {
                self.instances_bounds(key, &drawable, elements, groups)
            }
            Shown::Mesh { positions, .. } => {
                let positions = self.engine.fetch_key::<Vec3>(positions)?;
                let (min, max) = positions
                    .iter()
                    .fold((Vec3::MAX, Vec3::MIN), |(min, max), p| (min.min(*p), max.max(*p)));
                (min.x <= max.x).then(|| ((min + max) * 0.5, (max - min).length() * 0.5))
            }
        }
    }
}

pub fn format_value(value: &Value) -> String {
    let join = |values: Vec<String>| values.join(", ");
    match value {
        Value::F32(values) => join(values.iter().map(|v| v.to_string()).collect()),
        Value::U32(values) => join(values.iter().map(|v| v.to_string()).collect()),
    }
}

/// Reads numbers separated by commas or spaces.
pub fn parse_value(text: &str, scalar: Scalar) -> Option<Value> {
    let numbers = text
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|part| !part.is_empty());
    Some(match scalar {
        Scalar::F32 => Value::F32(numbers.map(|n| n.parse().ok()).collect::<Option<_>>()?),
        Scalar::U32 => Value::U32(numbers.map(|n| n.parse().ok()).collect::<Option<_>>()?),
    })
}

fn words_to_value(words: &[u32], scalar: Scalar) -> Value {
    match scalar {
        Scalar::F32 => Value::F32(bytemuck::cast_slice(words).to_vec()),
        Scalar::U32 => Value::U32(words.to_vec()),
    }
}

/// Something which can be added to a graph as a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteEntry {
    pub label: String,
    /// Where it is found, like `Curve`. Other graphs are in `Graph`.
    pub category: String,
    pub kind: NodeKind,
}

impl std::fmt::Display for PaletteEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} / {}", self.category, self.label)
    }
}

pub struct Document {
    /// Every graph of the document. They can use each other as nodes.
    library: Library,
    /// The name of the graph which is being edited.
    current: String,
    /// What the graphs lowered to the last time, the ones which could be.
    programs: HashMap<String, ProgramId>,
    library_errors: Vec<LibraryError>,
    registry: NodeRegistry,
    /// What people see of the nodes: their names and documentation.
    meta: MetaRegistry,
    session: Arc<Mutex<Session>>,
    error: Option<LowerError>,
}

impl Default for Document {
    fn default() -> Self {
        Self::new(Graph::new("untitled"))
    }
}

impl Document {
    /// What node `id` is called, by its description.
    fn label(&self, id: &str) -> String {
        self.meta.node(id).map_or_else(|| id.to_string(), |meta| meta.label.to_string())
    }

    pub fn new(graph: Graph) -> Self {
        let mut library = Library::new();
        let current = graph.name.clone();
        library.insert(graph);
        Self::with_library(library, &current)
    }

    /// A document of several graphs, with `current` opened. An empty
    /// library gets a graph to start with.
    pub fn with_library(mut library: Library, current: &str) -> Self {
        if library.is_empty() {
            library.insert(Graph::new("untitled"));
        }
        let current = match library.get(current) {
            Some(graph) => graph.name.clone(),
            None => library.iter().next().unwrap().name.clone(),
        };

        let mut registry = NodeRegistry::new();
        oneiroi_nodes::register(&mut registry);
        let mut meta = MetaRegistry::new();
        oneiroi_nodes::describe(&mut meta);
        let mut document = Self {
            library,
            current,
            programs: HashMap::new(),
            library_errors: Vec::new(),
            registry,
            meta,
            session: Arc::new(Mutex::new(Session::new())),
            error: None,
        };
        document.refresh();
        document
    }

    /// Something to look at: a circle swept along a curve, and in a second
    /// graph small rings put onto points along the same curve.
    pub fn example() -> Self {
        let mut library = Library::new();
        library.insert(Self::tube_example());
        library.insert(Self::instances_example());
        Self::with_library(library, "tube")
    }

    /// The curve both examples start from.
    fn example_curve(graph: &mut Graph) -> NodeId {
        let points: Vec<f32> = (0..8)
            .flat_map(|i| {
                let side = if i % 2 == 0 { -1.0 } else { 1.0 };
                [side * 0.8, i as f32, 0.5 * (i as f32 * 1.3).cos(), 1.0]
            })
            .collect();
        let curve = graph.add("curve.nurbs");
        let edits = [
            graph.set(curve, "points", Value::F32(points)),
            graph.set(curve, "knots", Value::F32(clamped_uniform_knots(8))),
        ];
        debug_assert!(edits.iter().all(Result::is_ok));
        curve
    }

    fn place(graph: &mut Graph, positions: &[(NodeId, [f32; 2])]) {
        for (node, position) in positions {
            graph.node_mut(*node).unwrap().meta.position = *position;
        }
    }

    /// A circle swept along a curve.
    fn tube_example() -> Graph {
        let mut graph = Graph::new("tube");
        let curve = Self::example_curve(&mut graph);
        let circle = graph.add("curve.circle");
        let sweep = graph.add("curve.sweep");
        let edits = [
            graph.set(circle, "radius", 0.25f32),
            graph.connect(curve, "segments", sweep, "spine"),
            graph.connect(circle, "circle", sweep, "profile"),
        ];
        debug_assert!(edits.iter().all(Result::is_ok));
        Self::place(&mut graph, &[(curve, [40., 40.]), (circle, [40., 220.]), (sweep, [300., 110.])]);
        // The surface is drawn as it is, it never has to become a mesh.
        graph.display = Some(sweep);
        graph
    }

    /// Small rings, a thin tube swept around a circle of a curve, put onto
    /// frames along the curve of the tube.
    fn instances_example() -> Graph {
        let mut graph = Graph::new("instances");
        let curve = Self::example_curve(&mut graph);
        let around: Vec<f32> = (0..=12)
            .flat_map(|i| {
                let angle = i as f32 / 12.0 * std::f32::consts::TAU;
                [0.45 * angle.cos(), 0.45 * angle.sin(), 0.0, 1.0]
            })
            .collect();
        let ring = graph.add("curve.nurbs");
        let thin = graph.add("curve.circle");
        let torus = graph.add("curve.sweep");
        let sample = graph.add("curve.sample");
        let instances = graph.add("instance.on_points");
        let edits = [
            graph.set(ring, "points", Value::F32(around)),
            graph.set(thin, "radius", 0.06f32),
            graph.connect(ring, "segments", torus, "spine"),
            graph.connect(thin, "circle", torus, "profile"),
            graph.set(sample, "count", 12u32),
            graph.connect(curve, "segments", sample, "segments"),
            graph.connect(sample, "frames", instances, "points"),
            graph.connect(torus, "surface", instances, "shape"),
        ];
        debug_assert!(edits.iter().all(Result::is_ok));
        Self::place(
            &mut graph,
            &[
                (curve, [40., 40.]),
                (sample, [300., 40.]),
                (ring, [40., 220.]),
                (thin, [40., 400.]),
                (torus, [300., 260.]),
                (instances, [560., 140.]),
            ],
        );
        graph.display = Some(instances);
        graph
    }

    /// The graph which is being edited.
    pub fn graph(&self) -> &Graph {
        self.library.get(&self.current).expect("the current graph is in the library")
    }

    fn graph_mut(&mut self) -> &mut Graph {
        self.library
            .get_mut(&self.current)
            .expect("the current graph is in the library")
    }

    /// The names of all graphs of the document.
    pub fn graphs(&self) -> Vec<String> {
        self.library.iter().map(|graph| graph.name.clone()).collect()
    }

    pub fn current(&self) -> &str {
        &self.current
    }

    /// Switches to editing another graph. Returns whether there is one
    /// with that name.
    pub fn open(&mut self, name: &str) -> bool {
        let known = self.library.get(name).is_some();
        if known {
            self.current = name.into();
            self.refresh();
        }
        known
    }

    /// Adds an empty graph, opens it and returns its name.
    pub fn new_graph(&mut self) -> String {
        let name = (1..)
            .map(|number| match number {
                1 => "graph".to_string(),
                _ => format!("graph {number}"),
            })
            .find(|name| self.library.get(name).is_none())
            .unwrap();
        self.library.insert(Graph::new(name.clone()));
        self.open(&name);
        name
    }

    /// Everything which can be added to the current graph as a node:
    /// the nodes of the registry, and the other graphs of the document
    /// as long as they do not use this one themselves.
    /// Sorted by category and label, so that what belongs together is
    /// next to each other.
    pub fn palette(&self) -> Vec<PaletteEntry> {
        let ids = self.registry.palette();
        let mut nodes: Vec<_> = ids
            .into_iter()
            .map(|id| {
                let meta = self.meta.node(id);
                PaletteEntry {
                    label: meta.map_or_else(|| id.to_string(), |meta| meta.label.to_string()),
                    category: meta.map_or("Other", |meta| meta.category).to_string(),
                    kind: NodeKind::Node(id.into()),
                }
            })
            .collect();
        nodes.sort_by(|a, b| (&a.category, &a.label).cmp(&(&b.category, &b.label)));
        let graphs = self
            .library
            .iter()
            .filter(|graph| !self.library.uses(&graph.name, &self.current))
            .map(|graph| PaletteEntry {
                label: graph.name.clone(),
                category: "Graph".into(),
                kind: NodeKind::Graph(graph.name.clone()),
            });
        nodes.into_iter().chain(graphs).collect()
    }

    /// The hue of what a pin carries: of its type, or of the types which
    /// implement its trait.
    pub fn hue(&self, ty: &PinType) -> Hue {
        let hue = match ty {
            PinType::Exact(ty) => self.meta.ty(ty.name).map(|meta| meta.hue),
            PinType::Trait(tr) => self.meta.tr(tr.name).map(|meta| meta.hue),
            PinType::Unknown => None,
        };
        hue.unwrap_or(Hue::Zinc)
    }

    /// What a pin carries in words, like `Curve` or `any Profile`, and what
    /// that is, as markdown.
    pub fn type_text(&self, ty: &PinType) -> (String, String) {
        match ty {
            PinType::Exact(ty) => self.meta.ty(ty.name).map_or_else(
                || (ty.name.to_string(), String::new()),
                |meta| (meta.label.to_string(), markdown(meta.doc)),
            ),
            PinType::Trait(tr) => self.meta.tr(tr.name).map_or_else(
                || (format!("any {}", tr.name), String::new()),
                |meta| (format!("any {}", meta.label), markdown(meta.doc)),
            ),
            PinType::Unknown => ("unknown".into(), String::new()),
        }
    }

    /// Node `id` of the open graph as it is shown: its pins, and what they
    /// are called and do.
    pub fn view(&self, id: NodeId) -> Option<NodeView> {
        let mut view = self.pins(id)?;
        let graph = self.graph();
        let kind = &graph.node(id)?.kind;
        view.doc = self.node_doc(kind);
        for (side, ports) in [(Side::Input, &mut view.inputs), (Side::Output, &mut view.outputs)] {
            for port in ports {
                (port.label, port.doc) = self.port_text(graph, id, side, &port.name, 0);
            }
        }
        Some(view)
    }

    /// What a node of `kind` does, as markdown.
    fn node_doc(&self, kind: &NodeKind) -> String {
        match kind {
            NodeKind::Node(id) => self.meta.node(id).map_or_else(String::new, |meta| markdown(meta.doc)),
            NodeKind::Graph(name) => {
                self.library.get(name).and_then(|graph| graph.about.doc.clone()).unwrap_or_default()
            }
        }
    }

    /// What `port` of node `id` of `graph` is called and how it is used:
    /// the note on the node if there is one, else what describes the node.
    /// A graph used as a node says so for its parameters and outputs, or
    /// they are described like the ports they come from.
    fn port_text(&self, graph: &Graph, id: NodeId, side: Side, port: &str, depth: u32) -> (String, String) {
        let Some(node) = graph.node(id) else {
            return (label_of(port), String::new());
        };
        let notes = match side {
            Side::Input => &node.meta.inputs,
            Side::Output => &node.meta.outputs,
        };
        let note = notes.get(port);
        let (label, doc) = match &node.kind {
            NodeKind::Node(node) => self
                .meta
                .node(node)
                .and_then(|meta| match side {
                    Side::Input => meta.inputs.iter().find(|p| p.name == port),
                    Side::Output => meta.outputs.iter().find(|p| p.name == port),
                })
                .map_or_else(|| (label_of(port), String::new()), |meta| (meta.label(), markdown(meta.doc))),
            // Graphs can not use themselves, the depth only guards against
            // a library which was edited by hand.
            NodeKind::Graph(name) if depth < 32 => match self.library.get(name) {
                Some(inner) => {
                    let (note, source) = match side {
                        Side::Input => (
                            inner.about.params.get(port),
                            inner.param_source(port).map(|(node, port)| (node, Side::Input, port)),
                        ),
                        Side::Output => (
                            inner.about.outputs.get(port),
                            inner
                                .outputs()
                                .iter()
                                .find(|output| output.name == port)
                                .map(|output| (output.node, Side::Output, output.output.as_str())),
                        ),
                    };
                    let doc = note.and_then(|note| note.doc.clone()).unwrap_or_else(|| {
                        source.map_or_else(String::new, |(node, side, port)| {
                            self.port_text(inner, node, side, port, depth + 1).1
                        })
                    });
                    // A parameter is called what the graph calls it.
                    let label = note.and_then(|note| note.label.clone()).unwrap_or_else(|| label_of(port));
                    (label, doc)
                }
                None => (label_of(port), String::new()),
            },
            NodeKind::Graph(_) => (label_of(port), String::new()),
        };
        (
            note.and_then(|note| note.label.clone()).unwrap_or(label),
            note.and_then(|note| note.doc.clone()).unwrap_or(doc),
        )
    }

    /// The pins of node `id` of the open graph, without their text.
    fn pins(&self, id: NodeId) -> Option<NodeView> {
        let node = self.graph().node(id)?;
        let name = match &node.kind {
            NodeKind::Node(id) | NodeKind::Graph(id) => id.as_str(),
        };
        let custom = node.meta.name.clone();

        // Another graph shows up with its parameters and outputs as pins.
        if let NodeKind::Graph(graph) = &node.kind
            && let Some(program) = self.programs.get(graph)
        {
            let session = self.session.lock().unwrap();
            let program = session.engine.programs().get(*program);
            return Some(NodeView {
                title: custom.unwrap_or_else(|| graph.clone()),
                doc: String::new(),
                inputs: program
                    .visible_params()
                    .map(|param| PortView {
                        name: param.name.clone(),
                        label: String::new(),
                        doc: String::new(),
                        ty: PinType::Exact(program.slot(param.slot).ty),
                        card: program.slot(param.slot).card,
                        default: param.default.to_vec(),
                    })
                    .collect(),
                outputs: program
                    .outputs
                    .iter()
                    .map(|output| PortView {
                        name: output.name.clone(),
                        label: String::new(),
                        doc: String::new(),
                        ty: PinType::Exact(program.slot(output.slot).ty),
                        card: program.slot(output.slot).card,
                        default: Vec::new(),
                    })
                    .collect(),
                known: true,
            });
        }
        let is_graph = matches!(node.kind, NodeKind::Graph(_));

        if let Some(generic) = self.registry.generic(name).filter(|_| !is_graph) {
            let port = |port: &oneiroi_core::node::GenericPort| PortView {
                name: port.name.into(),
                label: String::new(),
                doc: String::new(),
                ty: port.ty.into(),
                card: port.card,
                default: port.default.words().into(),
            };
            return Some(NodeView {
                title: custom.unwrap_or_else(|| self.label(generic.id)),
                doc: String::new(),
                inputs: generic.inputs.iter().map(port).collect(),
                outputs: generic.outputs.iter().map(port).collect(),
                known: true,
            });
        }
        if let Some(def) = self.registry.get(name).filter(|_| !is_graph) {
            let port = |port: &oneiroi_core::node::PortDecl| PortView {
                name: port.name.into(),
                label: String::new(),
                doc: String::new(),
                ty: PinType::Exact(port.ty),
                card: port.card,
                default: port.default.words().into(),
            };
            return Some(NodeView {
                title: custom.unwrap_or_else(|| self.label(def.id)),
                doc: String::new(),
                inputs: def.inputs.iter().map(port).collect(),
                outputs: def.outputs.iter().map(port).collect(),
                known: true,
            });
        }

        // Keep showing what the file says about a node nobody knows, so
        // its links stay visible and nothing is lost.
        let unknown = |name: &str| PortView {
            name: name.into(),
            label: String::new(),
            doc: String::new(),
            ty: PinType::Unknown,
            card: Card::Many,
            default: Vec::new(),
        };
        let mut outputs: Vec<String> = self
            .graph()
            .nodes()
            .flat_map(|(_, other)| other.inputs.values())
            .filter_map(|input| match input {
                Input::Link { node, output, .. } if *node == id => Some(output.clone()),
                _ => None,
            })
            .collect();
        outputs.sort();
        outputs.dedup();
        Some(NodeView {
            title: custom.unwrap_or_else(|| name.into()),
            doc: String::new(),
            inputs: node.inputs.keys().map(|name| unknown(name)).collect(),
            outputs: outputs.iter().map(|name| unknown(name)).collect(),
            known: false,
        })
    }

    /// Whether `output` of `from` may be plugged into `input` of `to`.
    pub fn accepts(&self, from: NodeId, output: &str, to: NodeId, input: &str) -> bool {
        self.link(from, output, to, input).is_some()
    }

    /// Whether a link from `output` of `from` to `input` of `to` copies
    /// instances into the shape they refer to: the input takes the shape,
    /// and is given instances of it.
    pub fn realizes(&self, from: NodeId, output: &str, to: NodeId, input: &str) -> bool {
        self.link(from, output, to, input) == Some(true)
    }

    /// Whether the link can be made, and if so whether instances are copied
    /// on the way.
    fn link(&self, from: NodeId, output: &str, to: NodeId, input: &str) -> Option<bool> {
        let port = |id, name: &str, outputs: bool| {
            let view = self.view(id)?;
            let ports = if outputs { view.outputs } else { view.inputs };
            ports.into_iter().find(|port| port.name == name)
        };
        let (source, target) = (port(from, output, true)?, port(to, input, false)?);
        let takes = |found: &TypeInfo| match target.ty {
            PinType::Exact(expected) => *found == expected,
            PinType::Trait(tr) => self.registry.implements(found, tr),
            PinType::Unknown => true,
        };
        match source.ty {
            PinType::Exact(found) if takes(&found) => Some(false),
            PinType::Exact(found) => {
                let shape = found.prototype.filter(|_| self.registry.realizes(&found))?;
                takes(shape).then_some(true)
            }
            // Only lowering can tell once a trait or an unknown node is involved.
            _ => Some(false),
        }
    }

    pub fn add_node(&mut self, id: &str, position: [f32; 2]) -> NodeId {
        self.add(NodeKind::Node(id.into()), position)
    }

    /// Adds a node from the [palette](Document::palette).
    pub fn add(&mut self, kind: NodeKind, position: [f32; 2]) -> NodeId {
        let graph = self.graph_mut();
        let node = graph.add_node(kind);
        graph.node_mut(node).unwrap().meta.position = position;
        if graph.display.is_none() {
            graph.display = Some(node);
        }
        self.refresh();
        node
    }

    pub fn remove_nodes(&mut self, nodes: &[NodeId]) {
        for node in nodes {
            self.graph_mut().remove_node(*node);
        }
        self.refresh();
    }

    pub fn move_nodes(&mut self, nodes: &[NodeId], delta: [f32; 2]) {
        for node in nodes {
            if let Some(node) = self.graph_mut().node_mut(*node) {
                node.meta.position[0] += delta[0];
                node.meta.position[1] += delta[1];
            }
        }
    }

    /// The note on `port` of `node`, what it shows over what describes the
    /// node.
    pub fn note(&self, node: NodeId, side: Side, port: &str) -> Note {
        let notes = self.graph().node(node).map(|node| match side {
            Side::Input => &node.meta.inputs,
            Side::Output => &node.meta.outputs,
        });
        notes.and_then(|notes| notes.get(port)).cloned().unwrap_or_default()
    }

    /// Gives `port` of `node` another name or documentation, or with an
    /// empty note takes them away again. Nothing has to run again for it.
    pub fn set_note(&mut self, node: NodeId, side: Side, port: &str, note: Note) {
        let note = Note {
            label: note.label.filter(|label| !label.trim().is_empty()),
            doc: note.doc.filter(|doc| !doc.trim().is_empty()),
        };
        if let Some(node) = self.graph_mut().node_mut(node) {
            match side {
                Side::Input => node.meta.set_input_note(port, note),
                Side::Output => node.meta.set_output_note(port, note),
            }
        }
    }

    /// What the open graph says about itself where it is used as a node.
    pub fn about(&self) -> &About {
        &self.graph().about
    }

    /// Changes what the open graph says about itself. Its parameters and
    /// outputs are by their names.
    pub fn set_about(&mut self, about: About) {
        self.graph_mut().about = about;
    }

    /// Returns whether the link was made.
    pub fn connect(&mut self, from: NodeId, output: &str, to: NodeId, input: &str) -> bool {
        let connected = self.accepts(from, output, to, input)
            && self.graph_mut().connect(from, output, to, input).is_ok();
        if connected {
            self.refresh();
        }
        connected
    }

    /// Removes the link into `input` of `to` if it comes from `from`.
    pub fn disconnect(&mut self, from: NodeId, output: &str, to: NodeId, input: &str) {
        let linked = self.graph().node(to).and_then(|node| node.inputs.get(input));
        if matches!(linked, Some(Input::Link { node, output: o, .. }) if *node == from && o == output) {
            self.reset(to, input);
        }
    }

    /// Puts an input back to its default.
    pub fn reset(&mut self, node: NodeId, input: &str) {
        self.graph_mut().reset(node, input);
        self.refresh();
    }

    pub fn set_display(&mut self, node: Option<NodeId>) {
        self.graph_mut().display = node;
        self.refresh();
    }

    /// What an input holds as text, and whether that text can be edited.
    /// A linked input and one of a type nobody can type in can not.
    pub fn input_text(&self, node: NodeId, input: &str) -> Option<(String, bool)> {
        let view = self.view(node)?;
        let port = view.inputs.iter().find(|port| port.name == input)?;
        let stored = self.graph().node(node)?.inputs.get(input);
        match stored {
            Some(Input::Link { each: true, .. }) => return Some(("each element".into(), false)),
            Some(Input::Link { .. }) => return Some(("linked".into(), false)),
            _ => {}
        }
        let PinType::Exact(ty) = port.ty else {
            return Some(("needs a link".into(), false));
        };
        Some(match stored {
            Some(Input::Value(value)) | Some(Input::Exposed { value: Some(value), .. }) => {
                (format_value(value), true)
            }
            _ => (format_value(&words_to_value(&port.default, ty.scalar)), true),
        })
    }

    /// Sets an input from text. Returns whether the text was a value of
    /// the type of the input.
    pub fn set_input(&mut self, node: NodeId, input: &str, text: &str) -> bool {
        let port = self.view(node).and_then(|view| {
            let port = view.inputs.into_iter().find(|port| port.name == input)?;
            match port.ty {
                PinType::Exact(ty) => Some((ty, port.card)),
                _ => None,
            }
        });
        let Some((ty, card)) = port else {
            return false;
        };
        let Some(value) = parse_value(text, ty.scalar) else {
            return false;
        };
        let len = match &value {
            Value::F32(values) => values.len(),
            Value::U32(values) => values.len(),
        };
        if len % ty.words as usize != 0 || (card == Card::One && len != ty.words as usize) {
            return false;
        }
        // A parameter stays one, the value is what it starts with.
        let exposed = match self.graph().input_of(node, input) {
            Some(Input::Exposed { name, .. }) => Some(name.clone()),
            _ => None,
        };
        let graph = self.graph_mut();
        let set = match exposed {
            Some(name) => graph.expose(node, input, &name, Some(value)),
            None => graph.set(node, input, value),
        };
        if set.is_err() {
            return false;
        }
        self.refresh();
        true
    }

    /// The engine and what it shows, for the preview to run and draw.
    pub fn session(&self) -> Arc<Mutex<Session>> {
        self.session.clone()
    }

    /// Lowers the displayed node again and plans what has to run for it.
    /// Everything which did not change is found in the store and is not
    /// computed twice. Running happens in the [`Session`].
    fn refresh(&mut self) {
        self.error = None;
        let mut session = self.session.lock().unwrap();
        let session = &mut *session;
        session.instance = None;

        // Every graph first, so the ones used as nodes are there.
        let (programs, errors) = self
            .library
            .lower_all(&self.registry, session.engine.programs_mut());
        self.programs = programs;
        self.library_errors = errors;
        let keep: Vec<_> = self.programs.values().copied().collect();

        let graph = self.library.get(&self.current).unwrap();
        let lowered = graph.display.map(|display| {
            graph.lower_node(
                Context {
                    registry: &self.registry,
                    programs: session.engine.programs(),
                    graphs: &self.programs,
                },
                display,
            )
        });
        let program = match lowered {
            Some(Ok(program)) => program,
            Some(Err(error)) => {
                self.error = Some(error);
                session.engine.programs_mut().retain(keep);
                return session.show(Shown::Nothing);
            }
            None => {
                session.engine.programs_mut().retain(keep);
                return session.show(Shown::Nothing);
            }
        };

        let id = match session.engine.programs().find(program.hash) {
            Some(id) => id,
            None => session.engine.add_program(program),
        };
        // Whatever the graphs lowered to before the edit is of no use now.
        session.engine.programs_mut().retain(keep.into_iter().chain([id]));

        let mut instance = session.engine.instantiate(id);
        // What was planned for before the edit is of no use anymore.
        session.engine.discard();
        session.engine.plan([&mut instance]);
        session.ran = 0;
        session.ran_on_gpu = 0;
        session.overflows.clear();

        // What can be shown of the outputs: a value which can be drawn as
        // it is if there is one, a mesh otherwise.
        let program = session.engine.programs().get(id);
        let output = |name: &str, ty: TypeInfo| {
            let output = &program.outputs[program.output_index(name)?];
            (program.slot(output.slot).ty == ty)
                .then(|| session.engine.output_key(&instance, name))
                .flatten()
        };
        let drawn = program.outputs.iter().find_map(|output| {
            let drawable = *self.registry.drawable(&program.slot(output.slot).ty)?;
            let key = session.engine.output_key(&instance, &output.name)?;
            let shape = session.engine.prototype_keys(&instance, &output.name);
            Some(Shown::Drawn { key, drawable, shape })
        });
        let mesh = || {
            Some(Shown::Mesh {
                positions: output("positions", Vec3::INFO)?,
                indices: output("indices", u32::INFO)?,
                normals: output("normals", Vec3::INFO),
            })
        };
        let shown = drawn.or_else(mesh).unwrap_or(Shown::Nothing);

        session.instance = Some(instance);
        session.show(shown);
    }

    /// The mesh of the displayed node, if it has one. This runs whatever
    /// is still planned.
    pub fn mesh(&self) -> Option<PreviewMesh> {
        self.session.lock().unwrap().mesh()
    }

    pub fn shown(&self) -> Shown {
        self.session.lock().unwrap().shown()
    }

    pub fn error(&self) -> Option<&LowerError> {
        self.error.as_ref()
    }

    /// One line about the state of the preview.
    pub fn status(&self) -> String {
        // What is planned runs here if the preview did not get to it yet,
        // so what is said is what happened.
        let (ran, on_gpu, overflows, drawn_with) = {
            let mut session = self.session.lock().unwrap();
            session.run();
            let on_gpu = session.ran_on_gpu;
            (session.ran, on_gpu, session.overflows.clone(), session.drawn_with)
        };
        // Without binding arrays the device can not run the graph at all.
        let cpu_only = {
            let session = self.session.lock().unwrap();
            let gpu = session.engine.executor().gpu();
            gpu.is_some_and(|gpu| !gpu.is_supported())
        };
        let ran = match (cpu_only, oneiroi_wgpu::compute_disabled()) {
            (true, true) => format!("{ran} (all on the CPU, as ONEIROI_COMPUTE asks)"),
            (true, false) => format!("{ran} (all on the CPU, the device can not run kernels)"),
            (false, _) => ran.to_string(),
        };
        let ran = match on_gpu {
            0 => ran,
            on_gpu => format!("{ran} ({on_gpu} on the GPU)"),
        };
        if let Some(overflow) = overflows.first() {
            return format!(
                "`{}` ran for {} of {} elements, the rest is left out",
                overflow.program, overflow.max, overflow.elements
            );
        }
        if let Some(error) = &self.error {
            // A graph which is used here and broken itself says more
            // than that it could not be found.
            if let LowerErrorKind::UnknownGraph(name) = &error.kind
                && let Some(inner) = self.library_errors.iter().find(
                    |inner| matches!(inner, LibraryError::Lower { graph, .. } if graph == name),
                )
            {
                return inner.to_string();
            }
            return error.to_string();
        }
        match (self.shown(), self.graph().display) {
            (Shown::Drawn { drawable, .. }, _) => {
                let with = match (drawable.topology, drawn_with) {
                    (Topology::Surface, Some(with)) => format!(" with {with}"),
                    _ => String::new(),
                };
                format!("`{}` drawn as it is{with}, {ran} nodes ran", drawable.ty.name)
            }
            (Shown::Mesh { .. }, _) => format!("a mesh, {ran} nodes ran"),
            (Shown::Nothing, Some(_)) => "the displayed node has nothing to show".into(),
            (Shown::Nothing, None) => "no node is displayed".into(),
        }
    }

    /// Whether a node runs for each element of what is linked to `input`.
    pub fn is_each(&self, node: NodeId, input: &str) -> bool {
        matches!(
            self.graph().input_of(node, input),
            Some(Input::Link { each: true, .. })
        )
    }

    /// Whether a node can run for each element of an input: it is another
    /// graph and something is linked there.
    pub fn can_be_each(&self, node: NodeId, input: &str) -> bool {
        let graph = matches!(self.graph().node(node).map(|n| &n.kind), Some(NodeKind::Graph(_)));
        graph && matches!(self.graph().input_of(node, input), Some(Input::Link { .. }))
    }

    /// Lets a node which is another graph run for each element of what is
    /// linked to `input`, or once for all of it again.
    pub fn toggle_each(&mut self, node: NodeId, input: &str) {
        if self.can_be_each(node, input) {
            let each = !self.is_each(node, input);
            self.graph_mut().set_each(node, input, each);
            self.refresh();
        }
    }

    pub fn is_exposed(&self, node: NodeId, input: &str) -> bool {
        matches!(self.graph().input_of(node, input), Some(Input::Exposed { .. }))
    }

    /// Makes an input a parameter of the graph, named after the input,
    /// or a plain value again. A linked input stays what it is. Returns
    /// whether anything changed.
    pub fn toggle_exposed(&mut self, node: NodeId, input: &str) -> bool {
        let graph = self.graph_mut();
        let changed = match graph.input_of(node, input).cloned() {
            Some(Input::Link { .. }) => false,
            Some(Input::Exposed { value: Some(value), .. }) => graph.set(node, input, value).is_ok(),
            Some(Input::Exposed { value: None, .. }) => graph.reset(node, input).is_some(),
            Some(Input::Value(value)) => graph.expose(node, input, input, Some(value)).is_ok(),
            None => graph.expose(node, input, input, None).is_ok(),
        };
        if changed {
            self.refresh();
        }
        changed
    }

    /// Whether an output of a node is an output of the whole graph.
    pub fn is_output(&self, node: NodeId, output: &str) -> bool {
        self.graph()
            .outputs()
            .iter()
            .any(|declared| declared.node == node && declared.output == output)
    }

    /// Makes an output of a node an output of the whole graph, which is
    /// what other graphs see of it, or takes it away again.
    pub fn toggle_output(&mut self, node: NodeId, output: &str) {
        let declared = self
            .graph()
            .outputs()
            .iter()
            .find(|declared| declared.node == node && declared.output == output)
            .map(|declared| declared.name.clone());
        let graph = self.graph_mut();
        match declared {
            Some(name) => graph.remove_output(&name),
            None => {
                // Named after the port, with a number if that is taken.
                let name = (1..)
                    .map(|number| match number {
                        1 => output.to_string(),
                        _ => format!("{output} {number}"),
                    })
                    .find(|name| graph.outputs().iter().all(|declared| declared.name != *name))
                    .unwrap();
                let _ = graph.add_output(&name, node, output);
            }
        }
        self.refresh();
    }

    /// All graphs of the document as one file.
    pub fn to_json(&self) -> String {
        self.library.to_json()
    }

    /// Reads a document back, also one which was saved as a single graph.
    pub fn from_json(json: &str) -> Result<Self, oneiroi_graph::FormatError> {
        Library::from_json(json).map(|library| Self::with_library(library, ""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodes(document: &Document) -> [NodeId; 3] {
        let find = |id: &str| {
            document
                .graph()
                .nodes()
                .find(|(_, node)| node.kind == NodeKind::Node(id.into()))
                .unwrap()
                .0
        };
        [find("curve.nurbs"), find("curve.circle"), find("curve.sweep")]
    }

    /// A mesh of the sweep of the example. It is in no palette, but a
    /// file can still name it.
    fn add_mesh(document: &mut Document) -> NodeId {
        let [_, _, sweep] = nodes(document);
        let mesh = document.add_node("surface.mesh", [540.0, 110.0]);
        assert!(document.connect(sweep, "surface", mesh, "surface"));
        mesh
    }

    /// The example with a mesh node shown.
    fn meshed() -> (Document, [NodeId; 4]) {
        let mut document = Document::example();
        let [curve, circle, sweep] = nodes(&document);
        // The surface was looked at before, so it is there already.
        document.status();
        let mesh = add_mesh(&mut document);
        document.set_display(Some(mesh));
        (document, [curve, circle, sweep, mesh])
    }

    #[test]
    fn the_example_shows_its_surface() {
        let document = Document::example();
        let [_, _, sweep] = nodes(&document);
        assert_eq!(document.graph().display, Some(sweep));

        // The sweep is drawn as it is: only the curve, the circle and the
        // sweep run, and nobody makes a mesh.
        let Shown::Drawn { key, drawable, .. } = document.shown() else {
            panic!("a surface can be drawn");
        };
        assert_eq!(drawable.ty.name, "tube_patch");
        assert_eq!(document.status(), "`tube_patch` drawn as it is, 3 nodes ran");
        assert!(document.mesh().is_none() && document.error().is_none());

        // Saying how many nodes ran runs them, if the preview did not.
        let session = document.session();
        let mut session = session.lock().unwrap();
        assert_eq!(session.run(), 0);
        // One patch for each of the 5 segments of the curve.
        let patches = session.engine_mut().fetch_key::<u32>(key).unwrap().len() / 28;
        assert_eq!(patches, 5);
        let (center, radius) = session.bounds().unwrap();
        assert!(radius > 3.0 && (center.y - 3.5).abs() < 0.5);
    }

    #[test]
    fn a_mesh_node_shows_its_mesh() {
        let (document, _) = meshed();
        let mesh = document.mesh().unwrap();
        // 5 segments, each a grid of the same size.
        assert!(!mesh.positions.is_empty() && mesh.positions.len() % 5 == 0);
        assert!(mesh.normals.is_some());
        // Only the mesh was new, the surface was there from before.
        assert_eq!(document.status(), "a mesh, 1 nodes ran");
        assert!(document.session().lock().unwrap().bounds().unwrap().1 > 3.0);
    }

    #[test]
    fn edits_show_up_in_the_preview() {
        let (mut document, [curve, circle, sweep, mesh]) = meshed();
        let before = document.session().lock().unwrap().generation();

        let fine = document.mesh().unwrap().positions.len();
        assert!(document.set_input(mesh, "tolerance", "0.05"));
        assert_ne!(before, document.session().lock().unwrap().generation());
        assert!(document.mesh().unwrap().positions.len() < fine);
        // The surface is what it was, only its mesh had to be made again.
        assert!(document.status().contains("1 nodes ran"));

        // Text which is not a value of the input changes nothing.
        let after = document.session().lock().unwrap().generation();
        assert!(!document.set_input(mesh, "tolerance", "tiny"));
        // A single value, not a list.
        assert!(!document.set_input(mesh, "tolerance", "0.1, 0.2"));
        assert!(!document.set_input(curve, "points", "1, 2, 3"));
        assert!(!document.set_input(sweep, "profile", "1"));
        assert_eq!(document.session().lock().unwrap().generation(), after);

        assert_eq!(document.input_text(circle, "radius").unwrap(), ("0.25".into(), true));
        assert_eq!(document.input_text(mesh, "tolerance").unwrap(), ("0.05".into(), true));
        assert_eq!(document.input_text(sweep, "spine").unwrap(), ("linked".into(), false));
        assert_eq!(document.input_text(sweep, "profile").unwrap(), ("linked".into(), false));
        document.reset(sweep, "profile");
        assert_eq!(
            document.input_text(sweep, "profile").unwrap(),
            ("needs a link".into(), false)
        );
    }

    #[test]
    fn edits_do_not_pile_up_programs() {
        let (mut document, [_, _, _, mesh]) = meshed();
        for step in 1..20 {
            assert!(document.set_input(mesh, "tolerance", &(step as f32 * 0.01).to_string()));
        }
        // The one of the graph itself and the one of its preview.
        let session = document.session();
        assert_eq!(session.lock().unwrap().engine_mut().programs().len(), 2);
    }

    #[test]
    fn links_are_checked_and_can_be_undone() {
        let mut document = Document::example();
        let [curve, circle, sweep] = nodes(&document);
        let mesh = add_mesh(&mut document);

        // A radius is not a profile and a curve is not a number.
        assert!(!document.connect(curve, "segments", circle, "radius"));
        assert!(!document.connect(mesh, "positions", sweep, "profile"));
        // A curve is a profile too, and only a surface can become a mesh.
        assert!(document.accepts(curve, "segments", sweep, "profile"));
        assert!(document.accepts(sweep, "surface", mesh, "surface"));
        assert!(!document.accepts(curve, "segments", mesh, "surface"));
        // A link back would be a cycle.
        assert!(!document.connect(mesh, "positions", mesh, "surface"));

        document.disconnect(circle, "circle", sweep, "profile");
        assert_eq!(document.shown(), Shown::Nothing);
        assert!(document.status().contains("something has to be connected"));
        assert_eq!(document.error().unwrap().node, Some(sweep));

        assert!(document.connect(circle, "circle", sweep, "profile"));
        assert!(matches!(document.shown(), Shown::Drawn { .. }) && document.error().is_none());
    }

    #[test]
    fn any_node_can_be_displayed() {
        let mut document = Document::example();
        let [curve, circle, sweep] = nodes(&document);
        let mesh = add_mesh(&mut document);

        // A curve and a circle are shown as the lines they are. The sweep
        // was never looked at, so only the curve had to run.
        document.set_display(Some(curve));
        assert_eq!(
            document.status(),
            "`cubic_nurbs_segment` drawn as it is, 1 nodes ran"
        );
        document.set_display(Some(circle));
        assert!(matches!(document.shown(), Shown::Drawn { .. }));
        assert!(document.session().lock().unwrap().bounds().unwrap().1 > 0.2);

        // Frames are neither a line nor a surface nor a mesh.
        let frames = document.add_node("curve.sample", [0.0, 0.0]);
        document.connect(curve, "segments", frames, "segments");
        document.set_display(Some(frames));
        assert_eq!(document.shown(), Shown::Nothing);
        assert_eq!(document.status(), "the displayed node has nothing to show");

        document.set_display(Some(mesh));
        assert!(matches!(document.shown(), Shown::Mesh { .. }));
        document.set_display(Some(sweep));
        assert!(matches!(document.shown(), Shown::Drawn { .. }));

        document.remove_nodes(&[sweep]);
        assert_eq!(document.graph().display, None);
        assert_eq!(document.status(), "no node is displayed");
    }

    #[test]
    fn nodes_are_shown_as_the_registry_describes_them() {
        let mut document = Document::example();
        let [_, _, sweep] = nodes(&document);
        let mesh = add_mesh(&mut document);

        let view = document.view(sweep).unwrap();
        assert_eq!(view.title, "Sweep");
        let ports = |ports: &[PortView]| -> Vec<_> {
            ports.iter().map(|p| (p.name.clone(), p.ty.name())).collect()
        };
        assert_eq!(
            ports(&view.inputs),
            [
                ("spine".to_string(), "cubic_nurbs_segment"),
                ("profile".to_string(), "profile")
            ]
        );
        assert_eq!(ports(&view.outputs), [("surface".to_string(), "surface")]);
        assert_eq!(document.view(mesh).unwrap().outputs.len(), 4);

        let labels: Vec<_> = document.palette().into_iter().map(|e| e.to_string()).collect();
        assert_eq!(
            labels,
            [
                "Curve / Circle",
                "Curve / NURBS Curve",
                "Curve / Sample Curve",
                "Curve / Sweep",
                "Curve / Translate Curve",
                "Instance / Instance on Points",
                "Instance / Realize Instances",
                "Instance / Transform",
                "Value / Numbers",
                "Value / Vectors",
                "Graph / instances"
            ]
        );

        let added = document.add_node("curve.sample", [10.0, 20.0]);
        assert_eq!(document.graph().node(added).unwrap().meta.position, [10.0, 20.0]);
        document.move_nodes(&[added], [5.0, -5.0]);
        assert_eq!(document.graph().node(added).unwrap().meta.position, [15.0, 15.0]);
    }

    fn radius(document: &Document) -> f32 {
        document.session().lock().unwrap().bounds().unwrap().1
    }

    #[test]
    fn pins_are_coloured_by_what_they_carry() {
        let document = Document::example();
        let [curve, circle, sweep] = nodes(&document);
        let hue = |node, output: bool| {
            let view = document.view(node).unwrap();
            let port = if output { &view.outputs[0] } else { &view.inputs[1] };
            document.hue(&port.ty)
        };
        assert_eq!(hue(curve, true), Hue::Amber);
        assert_eq!(hue(circle, true), Hue::Orange);
        // A trait has its own hue, near the ones of the types which have it.
        assert_eq!(hue(sweep, false), Hue::Orange);
        assert_eq!(document.type_text(&document.view(sweep).unwrap().inputs[1].ty).0, "any Profile");
        assert_eq!(document.hue(&PinType::Unknown), Hue::Zinc);
    }

    #[test]
    fn ports_are_described_by_notes_over_their_nodes() {
        let mut document = Document::example();
        let [curve, _, sweep] = nodes(&document);

        let view = document.view(curve).unwrap();
        assert_eq!(view.title, "NURBS Curve");
        assert!(view.doc.starts_with("A smooth curve"), "{}", view.doc);
        let points = &view.inputs[0];
        assert_eq!((points.name.as_str(), points.label.as_str()), ("points", "Control points"));
        assert!(points.doc.contains("weight"), "{}", points.doc);

        // A note changes what is shown.
        document.set_note(sweep, Side::Input, "spine", Note {
            label: Some("Path".into()),
            doc: Some("Where the tube *goes*.".into()),
        });
        let spine = document.view(sweep).unwrap().inputs[0].clone();
        assert_eq!((spine.label.as_str(), spine.doc.as_str()), ("Path", "Where the tube *goes*."));

        // An empty one takes it away again.
        document.set_note(sweep, Side::Input, "spine", Note {
            label: Some(" ".into()),
            doc: None,
        });
        assert_eq!(document.note(sweep, Side::Input, "spine"), Note::default());
        assert_eq!(document.view(sweep).unwrap().inputs[0].label, "Spine");
    }

    #[test]
    fn a_graph_as_a_node_is_described_by_what_is_inside() {
        let mut document = Document::example();
        document.new_graph();
        let circle = document.add_node("curve.circle", [0.0, 0.0]);
        assert!(document.toggle_exposed(circle, "radius"));
        document.toggle_output(circle, "circle");

        document.open("tube");
        let used = document.add(NodeKind::Graph("graph".into()), [0.0, 0.0]);
        let view = document.view(used).unwrap();
        // Without notes, like the ports the parameters and outputs come from.
        assert_eq!(view.doc, "");
        assert_eq!(view.inputs[0].label, "Radius");
        assert_eq!(view.inputs[0].doc, "The radius, a circle for each.");

        // A note inside wins over the node, and one of the graph over both.
        document.open("graph");
        document.set_note(circle, Side::Input, "radius", Note {
            label: None,
            doc: Some("How round.".into()),
        });
        document.open("tube");
        assert_eq!(document.view(used).unwrap().inputs[0].doc, "How round.");
        document.open("graph");
        let mut about = document.about().clone();
        about.doc = Some("A circle of its own.".into());
        about.set_param_note("radius", Note {
            label: Some("Size".into()),
            doc: Some("How large.".into()),
        });
        document.set_about(about);
        document.open("tube");
        let view = document.view(used).unwrap();
        assert_eq!(view.doc, "A circle of its own.");
        assert_eq!((view.inputs[0].label.as_str(), view.inputs[0].doc.as_str()), ("Size", "How large."));
        assert_eq!(view.outputs[0].doc, "The circles.");

        // And a note on the node in this graph wins over all of them.
        document.set_note(used, Side::Input, "radius", Note {
            label: Some("Thickness".into()),
            doc: None,
        });
        let view = document.view(used).unwrap();
        assert_eq!((view.inputs[0].label.as_str(), view.inputs[0].doc.as_str()), ("Thickness", "How large."));

        let mut loaded = Document::from_json(&document.to_json()).unwrap();
        loaded.open("tube");
        assert_eq!(loaded.view(used), document.view(used));
    }

    #[test]
    fn graphs_are_nodes_in_other_graphs() {
        let mut document = Document::example();
        let [_, _, sweep] = nodes(&document);
        let thin = radius(&document);

        // A graph of its own for the profile: a circle whose radius is a
        // parameter and which is what the graph gives.
        let profile = document.new_graph();
        assert_eq!((profile.as_str(), document.current()), ("graph", "graph"));
        let circle = document.add_node("curve.circle", [0.0, 0.0]);
        assert!(document.set_input(circle, "radius", "0.5"));
        assert!(document.toggle_exposed(circle, "radius"));
        document.toggle_output(circle, "circle");
        assert!(document.is_exposed(circle, "radius") && document.is_output(circle, "circle"));

        // Back in the first graph it can be picked like any node.
        assert!(document.open("tube"));
        let entry = document
            .palette()
            .into_iter()
            .find(|entry| entry.to_string() == "Graph / graph")
            .unwrap();
        let used = document.add(entry.kind, [0.0, 0.0]);
        let view = document.view(used).unwrap();
        assert_eq!((view.title.as_str(), view.known), ("graph", true));
        assert_eq!(view.inputs[0].name, "radius");
        assert_eq!(view.outputs[0].ty.name(), "circle");
        assert_eq!(document.input_text(used, "radius").unwrap().0, "0.5");

        // Its circle is a profile, and a thicker one than before.
        assert!(document.connect(used, "circle", sweep, "profile"));
        assert!(matches!(document.shown(), Shown::Drawn { .. }));
        let thick = radius(&document);
        assert!(thick > thin + 0.2, "{thick} {thin}");
        assert!(document.set_input(used, "radius", "0.25"));
        assert!((radius(&document) - thin).abs() < 1e-3);

        // An edit inside the other graph is seen here.
        document.reset(used, "radius");
        document.open("graph");
        assert!(document.set_input(circle, "radius", "1"));
        assert!(document.is_exposed(circle, "radius"));
        document.open("tube");
        assert!(radius(&document) > thick);

        // A graph can not be put into one which it is used by.
        document.open("graph");
        assert!(document.palette().iter().all(|entry| entry.kind != NodeKind::Graph("tube".into())));

        // One program per graph and the one of the preview, however many
        // edits there were.
        let session = document.session();
        assert!(session.lock().unwrap().engine_mut().programs().len() <= 3);
        drop(session);

        // All of it survives its file.
        let loaded = Document::from_json(&document.to_json()).unwrap();
        assert_eq!(loaded.graphs(), ["graph", "instances", "tube"]);
        let mut loaded = loaded;
        loaded.open("tube");
        assert!(radius(&loaded) > thick);
    }

    #[test]
    fn a_graph_can_run_for_each_element() {
        let mut document = Document::example();
        let [curve, circle, sweep] = nodes(&document);

        // A graph which sweeps a circle of any radius along the curve.
        let name = document.new_graph();
        let inner_curve = document.add_node("curve.nurbs", [0.0, 0.0]);
        let points: Vec<String> = (0..8)
            .flat_map(|i| [((i % 2) as f32).to_string(), i.to_string(), "0".into(), "1".into()])
            .collect();
        assert!(document.set_input(inner_curve, "points", &points.join(",")));
        let knots: Vec<_> = clamped_uniform_knots(8).iter().map(f32::to_string).collect();
        assert!(document.set_input(inner_curve, "knots", &knots.join(",")));
        let inner_circle = document.add_node("curve.circle", [0.0, 0.0]);
        document.toggle_exposed(inner_circle, "radius");
        let inner_sweep = document.add_node("curve.sweep", [0.0, 0.0]);
        assert!(document.connect(inner_curve, "segments", inner_sweep, "spine"));
        assert!(document.connect(inner_circle, "circle", inner_sweep, "profile"));
        document.toggle_output(inner_sweep, "surface");

        // Back in the first graph, three tubes of three radii.
        document.open("tube");
        let radii = document.add_node("value.f32", [0.0, 0.0]);
        assert!(document.set_input(radii, "values", "0.1, 0.2, 0.3"));
        let tubes = document.add(NodeKind::Graph(name), [0.0, 0.0]);
        assert!(document.connect(radii, "values", tubes, "radius"));
        assert!(document.can_be_each(tubes, "radius") && !document.can_be_each(sweep, "spine"));
        document.toggle_each(tubes, "radius");
        assert!(document.is_each(tubes, "radius"));
        assert_eq!(document.input_text(tubes, "radius").unwrap().0, "each element");

        document.set_display(Some(tubes));
        let Shown::Drawn { key, drawable, .. } = document.shown() else {
            panic!("the tubes are a surface");
        };
        assert_eq!(drawable.ty.name, "tube_patch");
        // The numbers, the curve once, and a circle and a sweep for each.
        assert_eq!(document.status(), "`tube_patch` drawn as it is, 8 nodes ran");
        let session = document.session();
        let patches = session.lock().unwrap().engine_mut().fetch_key::<u32>(key).unwrap().len() / 28;
        // Three tubes, each of the five segments of the curve.
        assert_eq!(patches, 3 * 5);
        drop(session);

        // Once for all of it again, which is one tube of the first radius.
        document.toggle_each(tubes, "radius");
        assert!(!document.is_each(tubes, "radius"));
        let _ = (curve, circle);
    }

    #[test]
    fn a_broken_graph_says_what_is_wrong_where_it_is_used() {
        let mut document = Document::example();
        let [_, _, sweep] = nodes(&document);

        let name = document.new_graph();
        let inner = document.add_node("curve.sweep", [0.0, 0.0]);
        document.toggle_output(inner, "surface");
        document.open("tube");
        let used = document.add(NodeKind::Graph(name), [0.0, 0.0]);
        document.set_display(Some(used));

        // The sweep in there has no profile, which is the real problem.
        assert!(document.status().contains("something has to be connected"), "{}", document.status());
        assert!(!document.view(used).unwrap().known);

        // Everything else keeps working.
        document.set_display(Some(sweep));
        assert!(matches!(document.shown(), Shown::Drawn { .. }));
    }

    #[test]
    fn outputs_and_parameters_can_be_taken_back() {
        let mut document = Document::example();
        let [_, circle, sweep] = nodes(&document);

        document.toggle_output(sweep, "surface");
        document.toggle_output(circle, "circle");
        let names: Vec<_> = document.graph().outputs().iter().map(|o| o.name.clone()).collect();
        assert_eq!(names, ["surface", "circle"]);
        document.toggle_output(sweep, "surface");
        assert!(!document.is_output(sweep, "surface") && document.is_output(circle, "circle"));

        // The value an input had is what the parameter starts with, and
        // what it goes back to.
        assert!(document.toggle_exposed(circle, "radius"));
        assert_eq!(document.input_text(circle, "radius").unwrap(), ("0.25".into(), true));
        assert!(document.toggle_exposed(circle, "radius"));
        assert!(!document.is_exposed(circle, "radius"));
        assert_eq!(document.input_text(circle, "radius").unwrap().0, "0.25");
        // A link is not a value to expose.
        assert!(!document.toggle_exposed(sweep, "spine"));
    }

    /// The second example puts rings onto points along its curve.
    #[test]
    fn the_examples_run() {
        let mut document = Document::example();
        assert_eq!(document.graphs(), ["instances", "tube"]);
        assert!(matches!(document.shown(), Shown::Drawn { .. }));
        assert!(document.open("instances"));
        assert!(document.error().is_none(), "{:?}", document.error());
        assert!(matches!(document.shown(), Shown::Drawn { .. }));
    }

    #[test]
    fn a_document_survives_its_file() {
        let document = Document::example();
        let mut loaded = Document::from_json(&document.to_json()).unwrap();
        assert_eq!(loaded.graphs(), ["instances", "tube"]);
        assert!(loaded.open("tube"));
        assert_eq!(loaded.graph(), document.graph());
        // The same graph is the same work, so it shows the very same value.
        assert_eq!(loaded.shown(), document.shown());

        // A node nobody knows is still shown with its links.
        let mut graph = document.graph().clone();
        let [curve, _, _] = nodes(&document);
        let unknown = graph.add("someone_elses.node");
        graph.connect(curve, "segments", unknown, "curve").unwrap();
        let loaded = Document::new(graph);
        let view = loaded.view(unknown).unwrap();
        assert!(!view.known);
        assert_eq!(view.inputs[0].name, "curve");
    }

    #[test]
    fn values_are_read_and_written_as_text() {
        assert_eq!(
            parse_value("1, 2.5  3", Scalar::F32),
            Some(Value::F32(vec![1.0, 2.5, 3.0]))
        );
        assert_eq!(parse_value("4,5", Scalar::U32), Some(Value::U32(vec![4, 5])));
        assert_eq!(parse_value("-1", Scalar::U32), None);
        assert_eq!(parse_value("", Scalar::F32), Some(Value::F32(vec![])));
        assert_eq!(format_value(&Value::F32(vec![0.5, 2.0])), "0.5, 2");
    }
}
