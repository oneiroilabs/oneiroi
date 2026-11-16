//! What the editor edits: one graph, and the engine which previews it.
//!
//! Nothing in here knows about the user interface, so all of it can be
//! tested without opening a window.

use std::{collections::HashMap, sync::Arc};

use glam::Vec3;
use oneiroi_core::{
    exec::{Engine, Instance},
    node::{NodeRegistry, PortType},
    store::ContentKey,
    types::{DataType, Scalar, Trait, TypeInfo},
};
use oneiroi_graph::{Context, Graph, Input, LowerError, NodeId, NodeKind, Value};
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

/// The mesh of the displayed node.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewMesh {
    /// Different for every mesh this document ever had.
    pub generation: u64,
    pub positions: Vec<Vec3>,
    pub normals: Option<Vec<Vec3>>,
    pub indices: Vec<u32>,
    pub center: Vec3,
    pub radius: f32,
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

pub struct Document {
    graph: Graph,
    registry: NodeRegistry,
    engine: Engine,
    instance: Option<Instance>,
    mesh: Option<Arc<PreviewMesh>>,
    mesh_key: Option<[Option<ContentKey>; 3]>,
    generation: u64,
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
        let mut registry = NodeRegistry::new();
        oneiroi_nodes::register(&mut registry);
        let mut document = Self {
            graph,
            registry,
            engine: Engine::default(),
            instance: None,
            mesh: None,
            mesh_key: None,
            generation: 0,
            error: None,
            nodes_run: 0,
        };
        document.refresh();
        document
    }

    /// A circle swept along a curve, to have something to look at.
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
        let edits = [
            graph.set(curve, "points", Value::F32(points)),
            graph.set(curve, "knots", Value::F32(clamped_uniform_knots(8))),
            graph.set(circle, "radius", 0.25f32),
            graph.connect(curve, "segments", sweep, "spine"),
            graph.connect(circle, "circle", sweep, "profile"),
        ];
        debug_assert!(edits.iter().all(Result::is_ok));

        for (node, position) in [(curve, [40., 40.]), (circle, [40., 220.]), (sweep, [340., 110.])] {
            graph.node_mut(node).unwrap().meta.position = position;
        }
        graph.display = Some(sweep);
        Self::new(graph)
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// The ids every node which can be added is known by.
    pub fn palette(&self) -> Vec<&'static str> {
        let generic = self.registry.generics().map(|node| node.id);
        // The versions of a generic node are picked by lowering, never by hand.
        let plain = self
            .registry
            .iter()
            .filter(|node| !self.registry.generics().any(|g| self.registry.instances(g).iter().any(|i| std::ptr::eq(*i, *node))))
            .map(|node| node.id);
        let mut ids: Vec<_> = generic.chain(plain).collect();
        ids.sort_unstable();
        ids
    }

    pub fn view(&self, id: NodeId) -> Option<NodeView> {
        let node = self.graph.node(id)?;
        let name = match &node.kind {
            NodeKind::Node(id) | NodeKind::Graph(id) => id.as_str(),
        };
        let custom = node.meta.name.clone();

        if let Some(generic) = self.registry.generic(name) {
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
        if let Some(def) = self.registry.get(name) {
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
            .graph
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
        let node = self.graph.add(id);
        self.graph.node_mut(node).unwrap().meta.position = position;
        if self.graph.display.is_none() {
            self.graph.display = Some(node);
        }
        self.refresh();
        node
    }

    pub fn remove_nodes(&mut self, nodes: &[NodeId]) {
        for node in nodes {
            self.graph.remove_node(*node);
        }
        self.refresh();
    }

    pub fn move_nodes(&mut self, nodes: &[NodeId], delta: [f32; 2]) {
        for node in nodes {
            if let Some(node) = self.graph.node_mut(*node) {
                node.meta.position[0] += delta[0];
                node.meta.position[1] += delta[1];
            }
        }
    }

    /// Returns whether the link was made.
    pub fn connect(&mut self, from: NodeId, output: &str, to: NodeId, input: &str) -> bool {
        let connected = self.accepts(from, output, to, input)
            && self.graph.connect(from, output, to, input).is_ok();
        if connected {
            self.refresh();
        }
        connected
    }

    /// Removes the link into `input` of `to` if it comes from `from`.
    pub fn disconnect(&mut self, from: NodeId, output: &str, to: NodeId, input: &str) {
        let linked = self.graph.node(to).and_then(|node| node.inputs.get(input));
        if matches!(linked, Some(Input::Link { node, output: o }) if *node == from && o == output) {
            self.reset(to, input);
        }
    }

    /// Puts an input back to its default.
    pub fn reset(&mut self, node: NodeId, input: &str) {
        self.graph.reset(node, input);
        self.refresh();
    }

    pub fn set_display(&mut self, node: Option<NodeId>) {
        self.graph.display = node;
        self.refresh();
    }

    /// What an input holds as text, and whether that text can be edited.
    /// A linked input and one of a type nobody can type in can not.
    pub fn input_text(&self, node: NodeId, input: &str) -> Option<(String, bool)> {
        let view = self.view(node)?;
        let port = view.inputs.iter().find(|port| port.name == input)?;
        let stored = self.graph.node(node)?.inputs.get(input);
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
        if len % ty.words as usize != 0 || self.graph.set(node, input, value).is_err() {
            return false;
        }
        self.refresh();
        true
    }

    /// Lowers and evaluates the displayed node again. Everything which
    /// did not change is found in the store and not computed twice.
    fn refresh(&mut self) {
        self.error = None;
        self.instance = None;
        self.nodes_run = 0;

        let lowered = self.graph.display.map(|display| {
            self.graph.lower_node(
                Context {
                    registry: &self.registry,
                    programs: self.engine.programs(),
                    graphs: &HashMap::new(),
                },
                display,
            )
        });
        let program = match lowered {
            Some(Ok(program)) => program,
            Some(Err(error)) => {
                self.error = Some(error);
                return self.show(None);
            }
            None => return self.show(None),
        };

        let id = match self.engine.programs().find(program.hash) {
            Some(id) => id,
            None => self.engine.add_program(program),
        };
        // Whatever the graph lowered to before the edit is of no use now.
        self.engine.programs_mut().retain([id]);
        let mut instance = self.engine.instantiate(id);
        self.nodes_run = self.engine.evaluate([&mut instance]);
        self.instance = Some(instance);
        self.show(self.instance.as_ref().map(|_| ()));
        self.engine.end_frame(BUDGET_WORDS);
    }

    /// Takes the mesh out of the evaluated instance, if it has one.
    fn show(&mut self, evaluated: Option<()>) {
        let Some(instance) = self.instance.as_ref().filter(|_| evaluated.is_some()) else {
            self.mesh = None;
            self.mesh_key = None;
            return;
        };

        let program = self.engine.programs().get(instance.program());
        let typed = |name: &str, ty: TypeInfo| {
            let output = &program.outputs[program.output_index(name)?];
            (program.slot(output.slot).ty == ty)
                .then(|| self.engine.output_key(instance, name))
                .flatten()
        };
        let key = [
            typed("positions", Vec3::INFO),
            typed("indices", u32::INFO),
            typed("normals", Vec3::INFO),
        ];
        if self.mesh_key == Some(key) {
            return;
        }
        self.mesh_key = Some(key);

        let read = |key: Option<ContentKey>| key.and_then(|key| self.engine.store().read::<Vec3>(key));
        let indices = key[1].and_then(|key| self.engine.store().read::<u32>(key));
        self.mesh = match (read(key[0]), indices) {
            (Some(positions), Some(indices)) if !positions.is_empty() && !indices.is_empty() => {
                let (min, max) = positions
                    .iter()
                    .fold((Vec3::MAX, Vec3::MIN), |(min, max), p| (min.min(*p), max.max(*p)));
                self.generation += 1;
                Some(Arc::new(PreviewMesh {
                    generation: self.generation,
                    positions: positions.to_vec(),
                    normals: read(key[2]).map(<[Vec3]>::to_vec),
                    indices: indices.to_vec(),
                    center: (min + max) * 0.5,
                    radius: (max - min).length() * 0.5,
                }))
            }
            _ => None,
        };
    }

    pub fn mesh(&self) -> Option<&Arc<PreviewMesh>> {
        self.mesh.as_ref()
    }

    pub fn error(&self) -> Option<&LowerError> {
        self.error.as_ref()
    }

    /// One line about the state of the preview.
    pub fn status(&self) -> String {
        if let Some(error) = &self.error {
            return error.to_string();
        }
        match (&self.mesh, self.graph.display) {
            (Some(mesh), _) => format!(
                "{} vertices, {} triangles, {} nodes ran",
                mesh.positions.len(),
                mesh.indices.len() / 3,
                self.nodes_run
            ),
            (None, Some(_)) => "the displayed node has no mesh".into(),
            (None, None) => "no node is displayed".into(),
        }
    }

    pub fn to_json(&self) -> String {
        self.graph.to_json()
    }

    pub fn from_json(json: &str) -> Result<Self, oneiroi_graph::FormatError> {
        Graph::from_json(json).map(Self::new)
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

    #[test]
    fn the_example_has_a_mesh() {
        let document = Document::example();
        let mesh = document.mesh().unwrap();
        // 5 segments with 16 rings each, 24 sides and the seam.
        assert_eq!(mesh.positions.len(), (5 * 16 + 1) * 25);
        assert!(mesh.normals.is_some() && mesh.radius > 1.0);
        assert!(document.error().is_none());
        assert!(document.status().contains("3 nodes ran"));
    }

    #[test]
    fn edits_show_up_in_the_preview() {
        let mut document = Document::example();
        let [curve, circle, sweep] = nodes(&document);
        let before = document.mesh().unwrap().clone();

        assert!(document.set_input(sweep, "sides", "6"));
        let after = document.mesh().unwrap().clone();
        assert_ne!(before.generation, after.generation);
        assert_eq!(after.positions.len(), (5 * 16 + 1) * 7);
        // Only the sweep had to run again.
        assert!(document.status().contains("1 nodes ran"));

        // Text which is not a value of the input changes nothing.
        assert!(!document.set_input(sweep, "sides", "six"));
        assert!(!document.set_input(sweep, "sides", "1.5"));
        assert!(!document.set_input(curve, "points", "1, 2, 3"));
        assert!(!document.set_input(sweep, "profile", "1"));
        assert_eq!(document.mesh().unwrap().generation, after.generation);

        assert_eq!(document.input_text(circle, "radius").unwrap(), ("0.25".into(), true));
        assert_eq!(document.input_text(sweep, "rings").unwrap(), ("16".into(), true));
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
        let mut document = Document::example();
        let [_, _, sweep] = nodes(&document);
        for sides in 3..20 {
            assert!(document.set_input(sweep, "sides", &sides.to_string()));
        }
        assert_eq!(document.engine.programs().len(), 1);
    }

    #[test]
    fn links_are_checked_and_can_be_undone() {
        let mut document = Document::example();
        let [curve, circle, sweep] = nodes(&document);

        // A radius is not a profile and a curve is not a number.
        assert!(!document.connect(curve, "segments", circle, "radius"));
        assert!(!document.connect(sweep, "positions", sweep, "profile"));
        // A curve is a profile too.
        assert!(document.accepts(curve, "segments", sweep, "profile"));
        // A link back would be a cycle.
        assert!(!document.connect(sweep, "indices", sweep, "rings"));

        document.disconnect(circle, "circle", sweep, "profile");
        assert!(document.mesh().is_none());
        assert!(document.status().contains("something has to be connected"));
        assert_eq!(document.error().unwrap().node, Some(sweep));

        assert!(document.connect(circle, "circle", sweep, "profile"));
        assert!(document.mesh().is_some() && document.error().is_none());
    }

    #[test]
    fn any_node_can_be_displayed() {
        let mut document = Document::example();
        let [curve, _, sweep] = nodes(&document);

        document.set_display(Some(curve));
        assert!(document.mesh().is_none());
        assert_eq!(document.status(), "the displayed node has no mesh");

        document.set_display(Some(sweep));
        assert!(document.mesh().is_some());

        document.remove_nodes(&[sweep]);
        assert_eq!(document.graph().display, None);
        assert_eq!(document.status(), "no node is displayed");
    }

    #[test]
    fn nodes_are_shown_as_the_registry_describes_them() {
        let mut document = Document::example();
        let [_, _, sweep] = nodes(&document);

        let view = document.view(sweep).unwrap();
        assert_eq!(view.title, "Sweep");
        let inputs: Vec<_> = view.inputs.iter().map(|p| (p.name.as_str(), p.ty.name())).collect();
        assert_eq!(
            inputs,
            [
                ("spine", "cubic_nurbs_segment"),
                ("profile", "profile"),
                ("rings", "u32"),
                ("sides", "u32")
            ]
        );
        assert_eq!(view.outputs.len(), 4);

        assert_eq!(
            document.palette(),
            ["curve.circle", "curve.frames", "curve.nurbs", "curve.sample", "curve.sweep"]
        );

        let added = document.add_node("curve.frames", [10.0, 20.0]);
        assert_eq!(document.graph().node(added).unwrap().meta.position, [10.0, 20.0]);
        document.move_nodes(&[added], [5.0, -5.0]);
        assert_eq!(document.graph().node(added).unwrap().meta.position, [15.0, 15.0]);
    }

    #[test]
    fn a_document_survives_its_file() {
        let document = Document::example();
        let loaded = Document::from_json(&document.to_json()).unwrap();
        assert_eq!(loaded.graph(), document.graph());
        assert_eq!(
            loaded.mesh().unwrap().positions,
            document.mesh().unwrap().positions
        );

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
