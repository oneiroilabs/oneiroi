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
    exec::{Engine, Instance},
    node::{Drawable, NodeRegistry, PortType},
    store::ContentKey,
    types::{DataType, Scalar, Trait, TypeInfo},
};
use oneiroi_wgpu::{DeferredExecutor, orbit::OrbitCamera};
use oneiroi_core::program::ProgramId;
use oneiroi_graph::{
    Context, Graph, Input, Library, LibraryError, LowerError, LowerErrorKind, NodeId, NodeKind,
    Value,
};
use oneiroi_nodes::curve::nurbs::clamped_uniform_knots;

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
    pub ty: PinType,
    /// The value of an input nothing is set on.
    pub default: Vec<u32>,
}

/// A node as it is shown.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeView {
    pub title: String,
    pub inputs: Vec<PortView>,
    pub outputs: Vec<PortView>,
    /// Whether the registry knows the node.
    pub known: bool,
}

/// What the preview shows of the displayed node.
#[derive(Debug, Clone, Copy)]
pub enum Shown {
    Nothing,
    /// A value the GPU draws as it is, without a mesh in between.
    Drawn { key: ContentKey, drawable: Drawable },
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
            (Self::Drawn { key: a, drawable: x }, Self::Drawn { key: b, drawable: y }) => {
                a == b && x.ty == y.ty
            }
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
}

impl Session {
    fn new() -> Self {
        Self {
            engine: Engine::new(DeferredExecutor::default()),
            instance: None,
            shown: Shown::Nothing,
            generation: 0,
            framed: None,
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
        let ran = self.engine.flush();
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

    /// Center and radius of a sphere around what is shown.
    pub fn bounds(&mut self) -> Option<(Vec3, f32)> {
        self.run();
        match self.shown {
            Shown::Nothing => None,
            Shown::Drawn { key, drawable } => {
                let (center, radius) = (drawable.bounds)(self.engine.fetch_key::<u32>(key)?)?;
                Some((center.into(), radius))
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
    pub kind: NodeKind,
}

impl std::fmt::Display for PaletteEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
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
    session: Arc<Mutex<Session>>,
    error: Option<LowerError>,
    nodes_run: usize,
}

impl Default for Document {
    fn default() -> Self {
        Self::new(Graph::new("untitled"))
    }
}

impl Document {
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
        let mut document = Self {
            library,
            current,
            programs: HashMap::new(),
            library_errors: Vec::new(),
            registry,
            session: Arc::new(Mutex::new(Session::new())),
            error: None,
            nodes_run: 0,
        };
        document.refresh();
        document
    }

    /// A circle swept along a curve and turned into a mesh, to have
    /// something to look at.
    pub fn example() -> Self {
        let mut graph = Graph::new("tube");
        let points: Vec<f32> = (0..8)
            .flat_map(|i| {
                let side = if i % 2 == 0 { -1.0 } else { 1.0 };
                [side * 0.8, i as f32, 0.5 * (i as f32 * 1.3).cos(), 1.0]
            })
            .collect();

        let curve = graph.add("curve.nurbs");
        let circle = graph.add("curve.circle");
        let sweep = graph.add("curve.sweep");
        let mesh = graph.add("surface.tessellate");
        let edits = [
            graph.set(curve, "points", Value::F32(points)),
            graph.set(curve, "knots", Value::F32(clamped_uniform_knots(8))),
            graph.set(circle, "radius", 0.25f32),
            graph.connect(curve, "segments", sweep, "spine"),
            graph.connect(circle, "circle", sweep, "profile"),
            graph.connect(sweep, "surface", mesh, "surface"),
        ];
        debug_assert!(edits.iter().all(Result::is_ok));

        let positions = [
            (curve, [40., 40.]),
            (circle, [40., 220.]),
            (sweep, [300., 110.]),
            (mesh, [540., 110.]),
        ];
        for (node, position) in positions {
            graph.node_mut(node).unwrap().meta.position = position;
        }
        // The surface itself is shown. The mesh node is there for whoever
        // wants triangles, the preview does not need it.
        graph.display = Some(sweep);
        Self::new(graph)
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
    pub fn palette(&self) -> Vec<PaletteEntry> {
        let generic = self.registry.generics().map(|node| node.id);
        // The versions of a generic node are picked by lowering, never by hand.
        let plain = self
            .registry
            .iter()
            .filter(|node| !self.registry.generics().any(|g| self.registry.instances(g).iter().any(|i| std::ptr::eq(*i, *node))))
            .map(|node| node.id);
        let mut ids: Vec<_> = generic.chain(plain).collect();
        ids.sort_unstable();

        let nodes = ids.into_iter().map(|id| PaletteEntry {
            label: id.into(),
            kind: NodeKind::Node(id.into()),
        });
        let graphs = self
            .library
            .iter()
            .filter(|graph| !self.library.uses(&graph.name, &self.current))
            .map(|graph| PaletteEntry {
                label: format!("graph: {}", graph.name),
                kind: NodeKind::Graph(graph.name.clone()),
            });
        nodes.chain(graphs).collect()
    }

    pub fn view(&self, id: NodeId) -> Option<NodeView> {
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
                inputs: program
                    .params
                    .iter()
                    .map(|param| PortView {
                        name: param.name.clone(),
                        ty: PinType::Exact(program.slot(param.slot).ty),
                        default: param.default.to_vec(),
                    })
                    .collect(),
                outputs: program
                    .outputs
                    .iter()
                    .map(|output| PortView {
                        name: output.name.clone(),
                        ty: PinType::Exact(program.slot(output.slot).ty),
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
                ty: port.ty.into(),
                default: port.default.words().into(),
            };
            return Some(NodeView {
                title: custom.unwrap_or_else(|| generic.display_name.into()),
                inputs: generic.inputs.iter().map(port).collect(),
                outputs: generic.outputs.iter().map(port).collect(),
                known: true,
            });
        }
        if let Some(def) = self.registry.get(name).filter(|_| !is_graph) {
            let port = |port: &oneiroi_core::node::PortDecl| PortView {
                name: port.name.into(),
                ty: PinType::Exact(port.ty),
                default: port.default.words().into(),
            };
            return Some(NodeView {
                title: custom.unwrap_or_else(|| def.display_name.into()),
                inputs: def.inputs.iter().map(port).collect(),
                outputs: def.outputs.iter().map(port).collect(),
                known: true,
            });
        }

        // Keep showing what the file says about a node nobody knows, so
        // its links stay visible and nothing is lost.
        let unknown = |name: &str| PortView {
            name: name.into(),
            ty: PinType::Unknown,
            default: Vec::new(),
        };
        let mut outputs: Vec<String> = self
            .graph()
            .nodes()
            .flat_map(|(_, other)| other.inputs.values())
            .filter_map(|input| match input {
                Input::Link { node, output } if *node == id => Some(output.clone()),
                _ => None,
            })
            .collect();
        outputs.sort();
        outputs.dedup();
        Some(NodeView {
            title: custom.unwrap_or_else(|| name.into()),
            inputs: node.inputs.keys().map(|name| unknown(name)).collect(),
            outputs: outputs.iter().map(|name| unknown(name)).collect(),
            known: false,
        })
    }

    /// Whether `output` of `from` may be plugged into `input` of `to`.
    pub fn accepts(&self, from: NodeId, output: &str, to: NodeId, input: &str) -> bool {
        let port = |id, name: &str, outputs: bool| {
            let view = self.view(id)?;
            let ports = if outputs { view.outputs } else { view.inputs };
            ports.into_iter().find(|port| port.name == name)
        };
        let (Some(source), Some(target)) = (port(from, output, true), port(to, input, false))
        else {
            return false;
        };
        match (source.ty, target.ty) {
            (PinType::Exact(found), PinType::Exact(expected)) => found == expected,
            (PinType::Exact(found), PinType::Trait(tr)) => self.registry.implements(&found, tr),
            // Only lowering can tell once a trait or an unknown node is involved.
            _ => true,
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
        if matches!(linked, Some(Input::Link { node, output: o }) if *node == from && o == output) {
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
        if matches!(stored, Some(Input::Link { .. })) {
            return Some(("linked".into(), false));
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
        let ty = self.view(node).and_then(|view| {
            let port = view.inputs.into_iter().find(|port| port.name == input)?;
            match port.ty {
                PinType::Exact(ty) => Some(ty),
                _ => None,
            }
        });
        let Some(ty) = ty else {
            return false;
        };
        let Some(value) = parse_value(text, ty.scalar) else {
            return false;
        };
        let len = match &value {
            Value::F32(values) => values.len(),
            Value::U32(values) => values.len(),
        };
        if len % ty.words as usize != 0 {
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
        self.nodes_run = 0;
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
        self.nodes_run = session.engine.plan([&mut instance]);

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
            Some(Shown::Drawn { key, drawable })
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
            (Shown::Drawn { drawable, .. }, _) => format!(
                "`{}` drawn as it is, {} nodes ran",
                drawable.ty.name, self.nodes_run
            ),
            (Shown::Mesh { .. }, _) => format!("a mesh, {} nodes ran", self.nodes_run),
            (Shown::Nothing, Some(_)) => "the displayed node has nothing to show".into(),
            (Shown::Nothing, None) => "no node is displayed".into(),
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

    fn nodes(document: &Document) -> [NodeId; 4] {
        let find = |id: &str| {
            document
                .graph()
                .nodes()
                .find(|(_, node)| node.kind == NodeKind::Node(id.into()))
                .unwrap()
                .0
        };
        [
            find("curve.nurbs"),
            find("curve.circle"),
            find("curve.sweep"),
            find("surface.tessellate"),
        ]
    }

    /// The example with its mesh node shown.
    fn meshed() -> (Document, [NodeId; 4]) {
        let mut document = Document::example();
        let nodes = nodes(&document);
        document.set_display(Some(nodes[3]));
        (document, nodes)
    }

    #[test]
    fn the_example_shows_its_surface() {
        let document = Document::example();
        let [_, _, sweep, _] = nodes(&document);
        assert_eq!(document.graph().display, Some(sweep));

        // The sweep is drawn as it is: only the curve, the circle and the
        // sweep run, and nobody makes a mesh.
        let Shown::Drawn { key, drawable } = document.shown() else {
            panic!("a surface can be drawn");
        };
        assert_eq!(drawable.ty.name, "tube_patch");
        assert_eq!(document.status(), "`tube_patch` drawn as it is, 3 nodes ran");
        assert!(document.mesh().is_none() && document.error().is_none());

        // Running happens in the session, wherever that is called from.
        let session = document.session();
        let mut session = session.lock().unwrap();
        assert_eq!(session.run(), 3);
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
        // 5 segments, each a grid of 16 rings and 24 sides.
        assert_eq!(mesh.positions.len(), 5 * 17 * 25);
        assert!(mesh.normals.is_some());
        // Only the mesh was new, the surface was there from before.
        assert_eq!(document.status(), "a mesh, 1 nodes ran");
        assert!(document.session().lock().unwrap().bounds().unwrap().1 > 3.0);
    }

    #[test]
    fn edits_show_up_in_the_preview() {
        let (mut document, [curve, circle, sweep, mesh]) = meshed();
        let before = document.session().lock().unwrap().generation();

        assert!(document.set_input(mesh, "sides", "6"));
        assert_ne!(before, document.session().lock().unwrap().generation());
        assert_eq!(document.mesh().unwrap().positions.len(), 5 * 17 * 7);
        // The surface is what it was, only its mesh had to be made again.
        assert!(document.status().contains("1 nodes ran"));

        // Text which is not a value of the input changes nothing.
        let after = document.session().lock().unwrap().generation();
        assert!(!document.set_input(mesh, "sides", "six"));
        assert!(!document.set_input(mesh, "sides", "1.5"));
        assert!(!document.set_input(curve, "points", "1, 2, 3"));
        assert!(!document.set_input(sweep, "profile", "1"));
        assert_eq!(document.session().lock().unwrap().generation(), after);

        assert_eq!(document.input_text(circle, "radius").unwrap(), ("0.25".into(), true));
        assert_eq!(document.input_text(mesh, "rings").unwrap(), ("16".into(), true));
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
        for sides in 3..20 {
            assert!(document.set_input(mesh, "sides", &sides.to_string()));
        }
        // The one of the graph itself and the one of its preview.
        let session = document.session();
        assert_eq!(session.lock().unwrap().engine_mut().programs().len(), 2);
    }

    #[test]
    fn links_are_checked_and_can_be_undone() {
        let mut document = Document::example();
        let [curve, circle, sweep, mesh] = nodes(&document);

        // A radius is not a profile and a curve is not a number.
        assert!(!document.connect(curve, "segments", circle, "radius"));
        assert!(!document.connect(mesh, "positions", sweep, "profile"));
        // A curve is a profile too, and only a surface can become a mesh.
        assert!(document.accepts(curve, "segments", sweep, "profile"));
        assert!(document.accepts(sweep, "surface", mesh, "surface"));
        assert!(!document.accepts(curve, "segments", mesh, "surface"));
        // A link back would be a cycle.
        assert!(!document.connect(mesh, "indices", mesh, "rings"));

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
        let [curve, circle, sweep, mesh] = nodes(&document);

        // A curve and a circle are shown as the lines they are.
        document.set_display(Some(curve));
        assert_eq!(
            document.status(),
            "`cubic_nurbs_segment` drawn as it is, 0 nodes ran"
        );
        document.set_display(Some(circle));
        assert!(matches!(document.shown(), Shown::Drawn { .. }));
        assert!(document.session().lock().unwrap().bounds().unwrap().1 > 0.2);

        // Frames are neither a line nor a surface nor a mesh.
        let frames = document.add_node("curve.frames", [0.0, 0.0]);
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
        let [_, _, sweep, mesh] = nodes(&document);

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

        let labels: Vec<_> = document.palette().into_iter().map(|e| e.label).collect();
        assert_eq!(
            labels,
            [
                "curve.circle",
                "curve.frames",
                "curve.nurbs",
                "curve.sample",
                "curve.sweep",
                "surface.tessellate"
            ]
        );

        let added = document.add_node("curve.frames", [10.0, 20.0]);
        assert_eq!(document.graph().node(added).unwrap().meta.position, [10.0, 20.0]);
        document.move_nodes(&[added], [5.0, -5.0]);
        assert_eq!(document.graph().node(added).unwrap().meta.position, [15.0, 15.0]);
    }

    fn radius(document: &Document) -> f32 {
        document.session().lock().unwrap().bounds().unwrap().1
    }

    #[test]
    fn graphs_are_nodes_in_other_graphs() {
        let mut document = Document::example();
        let [_, _, sweep, _] = nodes(&document);
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
            .find(|entry| entry.label == "graph: graph")
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
        assert!(document.palette().iter().all(|entry| entry.label != "graph: tube"));

        // One program per graph and the one of the preview, however many
        // edits there were.
        let session = document.session();
        assert!(session.lock().unwrap().engine_mut().programs().len() <= 3);
        drop(session);

        // All of it survives its file.
        let loaded = Document::from_json(&document.to_json()).unwrap();
        assert_eq!(loaded.graphs(), ["graph", "tube"]);
        let mut loaded = loaded;
        loaded.open("tube");
        assert!(radius(&loaded) > thick);
    }

    #[test]
    fn a_broken_graph_says_what_is_wrong_where_it_is_used() {
        let mut document = Document::example();
        let [_, _, sweep, _] = nodes(&document);

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
        let [_, circle, sweep, _] = nodes(&document);

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

    #[test]
    fn a_document_survives_its_file() {
        let document = Document::example();
        let loaded = Document::from_json(&document.to_json()).unwrap();
        assert_eq!(loaded.graph(), document.graph());
        // The same graph is the same work, so it shows the very same value.
        assert_eq!(loaded.shown(), document.shown());

        // A node nobody knows is still shown with its links.
        let mut graph = document.graph().clone();
        let [curve, _, _, _] = nodes(&document);
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
