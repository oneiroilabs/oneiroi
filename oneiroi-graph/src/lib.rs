//! The graph as it is authored, saved and loaded.
//!
//! A [`Graph`] only knows the ids of its nodes and how they are wired. It
//! is turned into something that runs by [`Graph::lower`], which looks the
//! ids up in a registry and builds a program out of them.
//!
//! What a graph takes and gives where another graph uses it are its
//! [parameters](GraphParam) and [outputs](GraphOutput). In the graph they
//! are the ports of its [input](NodeKind::Input) and
//! [output](NodeKind::Output) nodes.
//!
//! Next to the wiring a graph carries what every editor wants to keep with
//! it, like the position of a node, and what people read about it, like
//! other names for ports and their documentation. None of that takes part
//! in lowering.

use std::{
    collections::{BTreeMap, HashSet},
    fmt,
};

use oneiroi_core::node::Card;
use serde::{Deserialize, Serialize};

mod library;
mod lower;

pub use library::{Library, LibraryError};
pub use lower::{Context, LowerError, LowerErrorKind};

/// The version of the file format this crate writes.
///
/// 2: parameters are ports of input nodes instead of exposed inputs, and
/// the outputs are shown by an output node.
pub const FORMAT_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(u32);

impl NodeId {
    /// The number behind the id, for interfaces which want plain numbers.
    pub fn raw(self) -> u32 {
        self.0
    }
}

impl From<u32> for NodeId {
    fn from(raw: u32) -> Self {
        Self(raw)
    }
}

/// A value written out by hand: all elements of it, one number after the
/// other. A port of type `vec3` takes three `f32` per element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Value {
    F32(Vec<f32>),
    U32(Vec<u32>),
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Self::F32(vec![value])
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Self::U32(vec![value])
    }
}

/// What a node of the graph is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// A node from the registry, by its id. Generic nodes are named by the
    /// id of the generic node, never by the one of a version.
    Node(String),
    /// Another graph, by its name.
    Graph(String),
    /// The parameters of the graph, as its outputs. There can be several,
    /// all with the same ports.
    Input,
    /// The outputs of the graph, as its inputs. There is at most one.
    Output,
}

/// Where an input of a node gets its value from. An input without an entry
/// has the default of its port.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Input {
    /// Fixed for every instance of the graph.
    Value(Value),
    /// The output of another node. A parameter of the graph is an output
    /// of an [input node](NodeKind::Input).
    Link {
        node: NodeId,
        output: String,
        /// Only for a node which is another graph: it runs once for every
        /// element of the output, and its outputs are what all of the runs
        /// gave, one after the other.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        each: bool,
    },
}

/// How many elements a graph runs for at most, when it runs for each one.
/// The rest is left out, and that is reported.
pub const MAX_ELEMENTS: u32 = 4096;

/// What people read about a port where it should differ from what its node
/// says: another name, or documentation of how it is used here.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Note {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Markdown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

impl Note {
    pub fn is_empty(&self) -> bool {
        self.label.is_none() && self.doc.is_none()
    }
}

/// Sets or clears the note of `port` in `notes`, leaving out empty ones.
fn set_note(notes: &mut BTreeMap<String, Note>, port: &str, note: Note) {
    match note.is_empty() {
        true => notes.remove(port),
        false => notes.insert(port.into(), note),
    };
}

/// What an editor keeps with a node. It never changes what the graph does.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct NodeMeta {
    #[serde(default)]
    pub position: [f32; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Notes on its inputs, by the name of the port.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub inputs: BTreeMap<String, Note>,
    /// Notes on its outputs, by the name of the port.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub outputs: BTreeMap<String, Note>,
}

impl NodeMeta {
    pub fn set_input_note(&mut self, port: &str, note: Note) {
        set_note(&mut self.inputs, port, note);
    }

    pub fn set_output_note(&mut self, port: &str, note: Note) {
        set_note(&mut self.outputs, port, note);
    }
}

/// What people read about a graph where it is used as a node in another.
/// Its parameters and outputs without a note are described like the ports
/// they come from.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct About {
    /// What the graph does, as markdown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    /// Notes on its parameters, by their names.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, Note>,
    /// Notes on its outputs, by their names.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub outputs: BTreeMap<String, Note>,
}

impl About {
    pub fn is_empty(&self) -> bool {
        self.doc.is_none() && self.params.is_empty() && self.outputs.is_empty()
    }

    pub fn set_param_note(&mut self, param: &str, note: Note) {
        set_note(&mut self.params, param, note);
    }

    pub fn set_output_note(&mut self, output: &str, note: Note) {
        set_note(&mut self.outputs, output, note);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub kind: NodeKind,
    /// By the name of the port.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub inputs: BTreeMap<String, Input>,
    #[serde(default)]
    pub meta: NodeMeta,
}

/// An output of a node which is an output of the whole graph: a port of
/// the [output node](NodeKind::Output) and what is linked to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphOutput {
    pub name: String,
    pub node: NodeId,
    pub output: String,
}

/// What a graph takes where another graph uses it: a port of its [input
/// nodes](NodeKind::Input).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphParam {
    pub name: String,
    /// The name of its data type. Without one it has the type of the first
    /// port it is linked to, and how many elements that takes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ty: Option<String>,
    /// How many elements it carries, if it has a type of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<Card>,
    /// What it is unless it is set. Without one it is the default of the
    /// first port it is linked to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
}

impl GraphParam {
    /// A parameter which is like the first port it is linked to.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ty: None,
            card: None,
            default: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    version: u32,
    pub name: String,
    nodes: BTreeMap<NodeId, Node>,
    next_id: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    params: Vec<GraphParam>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    outputs: Vec<GraphOutput>,
    /// The node an editor shows in its preview.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<NodeId>,
    #[serde(default, skip_serializing_if = "About::is_empty")]
    pub about: About,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    MissingNode(NodeId),
    /// The link would make a node depend on itself.
    Cycle,
    DuplicateOutput(String),
    DuplicateParam(String),
    UnknownParam(String),
    /// The output node has no outputs to link from.
    FromOutputNode,
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(f, "there is no node {}", node.0),
            Self::Cycle => write!(f, "a node can not depend on itself"),
            Self::DuplicateOutput(name) => write!(f, "there already is an output `{name}`"),
            Self::DuplicateParam(name) => write!(f, "there already is a parameter `{name}`"),
            Self::UnknownParam(name) => write!(f, "there is no parameter `{name}`"),
            Self::FromOutputNode => write!(f, "the outputs of the graph go into its output node, nothing comes out"),
        }
    }
}

impl std::error::Error for EditError {}

#[derive(Debug)]
pub enum FormatError {
    Syntax(serde_json::Error),
    /// The file was written by a newer version of the format.
    Version(u32),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(error) => write!(f, "{error}"),
            Self::Version(version) => write!(
                f,
                "the graph has format version {version}, this build reads up to {FORMAT_VERSION}"
            ),
        }
    }
}

impl std::error::Error for FormatError {}

impl Graph {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            version: FORMAT_VERSION,
            name: name.into(),
            nodes: BTreeMap::new(),
            next_id: 0,
            params: Vec::new(),
            outputs: Vec::new(),
            display: None,
            about: About::default(),
        }
    }

    /// Adds a node. There is only one output node: asked for another, the
    /// one there is comes back.
    pub fn add_node(&mut self, kind: NodeKind) -> NodeId {
        if kind == NodeKind::Output
            && let Some(output) = self.output_node()
        {
            return output;
        }
        let id = NodeId(self.next_id);
        self.next_id += 1;
        self.nodes.insert(
            id,
            Node {
                kind,
                inputs: BTreeMap::new(),
                meta: NodeMeta::default(),
            },
        );
        id
    }

    /// Adds a node from the registry by its id.
    pub fn add(&mut self, node: &str) -> NodeId {
        self.add_node(NodeKind::Node(node.into()))
    }

    /// Adds a use of another graph by its name.
    pub fn add_graph(&mut self, graph: &str) -> NodeId {
        self.add_node(NodeKind::Graph(graph.into()))
    }

    /// The node which shows the parameters of the graph, the first one if
    /// there are several.
    pub fn input_node(&self) -> Option<NodeId> {
        self.nodes.iter().find(|(_, node)| node.kind == NodeKind::Input).map(|(id, _)| *id)
    }

    /// The node which shows the outputs of the graph.
    pub fn output_node(&self) -> Option<NodeId> {
        self.nodes.iter().find(|(_, node)| node.kind == NodeKind::Output).map(|(id, _)| *id)
    }

    fn is_kind(&self, id: NodeId, kind: &NodeKind) -> bool {
        self.nodes.get(&id).is_some_and(|node| node.kind == *kind)
    }

    /// Removes a node together with every link from it and every output of
    /// the graph it provides. Without the output node the graph has no
    /// outputs, they are what is linked to it.
    pub fn remove_node(&mut self, id: NodeId) -> Option<Node> {
        if self.is_kind(id, &NodeKind::Output) {
            self.outputs.clear();
        }
        let node = self.nodes.remove(&id)?;
        for other in self.nodes.values_mut() {
            other
                .inputs
                .retain(|_, input| !matches!(input, Input::Link { node, .. } if *node == id));
        }
        self.outputs.retain(|output| output.node != id);
        if self.display == Some(id) {
            self.display = None;
        }
        Some(node)
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }

    pub fn nodes(&self) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes.iter().map(|(id, node)| (*id, node))
    }

    fn input(&mut self, node: NodeId, port: &str, input: Input) -> Result<(), EditError> {
        self.nodes
            .get_mut(&node)
            .ok_or(EditError::MissingNode(node))?
            .inputs
            .insert(port.into(), input);
        Ok(())
    }

    /// Gives an input a fixed value.
    pub fn set(&mut self, node: NodeId, port: &str, value: impl Into<Value>) -> Result<(), EditError> {
        self.input(node, port, Input::Value(value.into()))
    }

    /// Makes an input the parameter of the graph called `name`: links it
    /// to that port of the input node, and makes both if they are not
    /// there yet. A new parameter starts at `value`, if there is one.
    pub fn expose(
        &mut self,
        node: NodeId,
        port: &str,
        name: &str,
        value: Option<Value>,
    ) -> Result<(), EditError> {
        if !self.nodes.contains_key(&node) {
            return Err(EditError::MissingNode(node));
        }
        if self.param(name).is_none() {
            self.params.push(GraphParam {
                default: value,
                ..GraphParam::new(name)
            });
        }
        let input = match self.input_node() {
            Some(input) => input,
            None => {
                let input = self.add_node(NodeKind::Input);
                let left = self.nodes.values().map(|node| node.meta.position[0]).fold(0.0, f32::min);
                self.nodes.get_mut(&input).unwrap().meta.position = [left - 220.0, 0.0];
                input
            }
        };
        self.connect(input, name, node, port)
    }

    /// The parameter called `name` an input of `node` is linked to, if it
    /// is linked to one.
    pub fn exposed_as(&self, node: NodeId, port: &str) -> Option<&str> {
        match self.input_of(node, port)? {
            Input::Link { node: from, output, .. } if self.is_kind(*from, &NodeKind::Input) => Some(output),
            _ => None,
        }
    }

    pub fn params(&self) -> &[GraphParam] {
        &self.params
    }

    pub fn param(&self, name: &str) -> Option<&GraphParam> {
        self.params.iter().find(|param| param.name == name)
    }

    pub fn param_mut(&mut self, name: &str) -> Option<&mut GraphParam> {
        self.params.iter_mut().find(|param| param.name == name)
    }

    /// Adds a parameter after the others.
    pub fn add_param(&mut self, param: GraphParam) -> Result<(), EditError> {
        if self.param(&param.name).is_some() {
            return Err(EditError::DuplicateParam(param.name));
        }
        self.params.push(param);
        Ok(())
    }

    /// Removes a parameter, every link from it and its note.
    pub fn remove_param(&mut self, name: &str) -> Option<GraphParam> {
        let index = self.params.iter().position(|param| param.name == name)?;
        let inputs: HashSet<_> = self.input_nodes().collect();
        for node in self.nodes.values_mut() {
            node.inputs.retain(|_, input| {
                !matches!(input, Input::Link { node, output, .. } if inputs.contains(node) && output == name)
            });
        }
        self.about.params.remove(name);
        Some(self.params.remove(index))
    }

    /// Gives a parameter another name, which every link from it and its
    /// note follow.
    pub fn rename_param(&mut self, name: &str, to: &str) -> Result<(), EditError> {
        if name == to {
            return Ok(());
        }
        if self.param(to).is_some() {
            return Err(EditError::DuplicateParam(to.into()));
        }
        let param = self.param_mut(name).ok_or_else(|| EditError::UnknownParam(name.into()))?;
        param.name = to.into();
        let inputs: HashSet<_> = self.input_nodes().collect();
        for node in self.nodes.values_mut() {
            for input in node.inputs.values_mut() {
                if let Input::Link { node, output, .. } = input
                    && inputs.contains(node)
                    && output == name
                {
                    *output = to.into();
                }
            }
        }
        if let Some(note) = self.about.params.remove(name) {
            self.about.params.insert(to.into(), note);
        }
        Ok(())
    }

    /// Puts a parameter at `index` among them.
    pub fn move_param(&mut self, name: &str, index: usize) -> Result<(), EditError> {
        let at = self
            .params
            .iter()
            .position(|param| param.name == name)
            .ok_or_else(|| EditError::UnknownParam(name.into()))?;
        let param = self.params.remove(at);
        self.params.insert(index.min(self.params.len()), param);
        Ok(())
    }

    /// Every input node.
    pub fn input_nodes(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes
            .iter()
            .filter(|(_, node)| node.kind == NodeKind::Input)
            .map(|(id, _)| *id)
    }

    /// Links the `output` of `from` to the `input` of `to`, replacing
    /// whatever that input had before.
    pub fn connect(
        &mut self,
        from: NodeId,
        output: &str,
        to: NodeId,
        input: &str,
    ) -> Result<(), EditError> {
        if !self.nodes.contains_key(&from) {
            return Err(EditError::MissingNode(from));
        }
        if self.is_kind(from, &NodeKind::Output) {
            return Err(EditError::FromOutputNode);
        }
        // What goes into the output node is an output of the graph.
        if self.is_kind(to, &NodeKind::Output) {
            let link = GraphOutput {
                name: input.into(),
                node: from,
                output: output.into(),
            };
            match self.outputs.iter_mut().find(|output| output.name == input) {
                Some(existing) => *existing = link,
                None => self.outputs.push(link),
            }
            return Ok(());
        }
        if self.depends_on(from, to) {
            return Err(EditError::Cycle);
        }
        let link = Input::Link {
            node: from,
            output: output.into(),
            each: false,
        };
        self.input(to, input, link)
    }

    /// Makes a linked input one the node runs for each element of, or a
    /// plain link again. Returns whether the input is linked.
    pub fn set_each(&mut self, node: NodeId, input: &str, each: bool) -> bool {
        let input = self.nodes.get_mut(&node).and_then(|node| node.inputs.get_mut(input));
        match input {
            Some(Input::Link { each: flag, .. }) => {
                *flag = each;
                true
            }
            _ => false,
        }
    }

    /// Puts an input back to the default of its port. For the output node
    /// that removes the output.
    pub fn reset(&mut self, node: NodeId, port: &str) -> Option<Input> {
        if self.is_kind(node, &NodeKind::Output) {
            let index = self.outputs.iter().position(|output| output.name == port)?;
            let output = self.outputs.remove(index);
            return Some(Input::Link {
                node: output.node,
                output: output.output,
                each: false,
            });
        }
        self.nodes.get_mut(&node)?.inputs.remove(port)
    }

    /// The nodes `id` has links from.
    fn sources(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes
            .get(&id)
            .into_iter()
            .flat_map(|node| node.inputs.values())
            .filter_map(|input| match input {
                Input::Link { node, .. } => Some(*node),
                _ => None,
            })
    }

    /// Whether `node` is `on` or gets anything from it, however indirect.
    fn depends_on(&self, node: NodeId, on: NodeId) -> bool {
        let mut seen = HashSet::new();
        let mut open = vec![node];
        while let Some(id) = open.pop() {
            if id == on {
                return true;
            }
            if seen.insert(id) {
                open.extend(self.sources(id));
            }
        }
        false
    }

    pub fn outputs(&self) -> &[GraphOutput] {
        &self.outputs
    }

    /// Makes the `output` of `node` an output of the graph called `name`.
    pub fn add_output(&mut self, name: &str, node: NodeId, output: &str) -> Result<(), EditError> {
        if !self.nodes.contains_key(&node) {
            return Err(EditError::MissingNode(node));
        }
        if self.outputs.iter().any(|output| output.name == name) {
            return Err(EditError::DuplicateOutput(name.into()));
        }
        self.outputs.push(GraphOutput {
            name: name.into(),
            node,
            output: output.into(),
        });
        Ok(())
    }

    pub fn remove_output(&mut self, name: &str) {
        self.outputs.retain(|output| output.name != name);
    }

    /// Gives an output another name, which its note follows.
    pub fn rename_output(&mut self, name: &str, to: &str) -> Result<(), EditError> {
        if name == to {
            return Ok(());
        }
        if self.outputs.iter().any(|output| output.name == to) {
            return Err(EditError::DuplicateOutput(to.into()));
        }
        if let Some(output) = self.outputs.iter_mut().find(|output| output.name == name) {
            output.name = to.into();
        }
        if let Some(note) = self.about.outputs.remove(name) {
            self.about.outputs.insert(to.into(), note);
        }
        Ok(())
    }

    /// Puts an output at `index` among them.
    pub fn move_output(&mut self, name: &str, index: usize) {
        if let Some(at) = self.outputs.iter().position(|output| output.name == name) {
            let output = self.outputs.remove(at);
            self.outputs.insert(index.min(self.outputs.len()), output);
        }
    }

    /// The names of the graphs this one uses.
    pub fn dependencies(&self) -> impl Iterator<Item = &str> {
        self.nodes.values().filter_map(|node| match &node.kind {
            NodeKind::Graph(name) => Some(name.as_str()),
            _ => None,
        })
    }

    /// The input linked to parameter `name` which comes first, as its node
    /// and its port. A parameter without a type or a note of its own is
    /// like it.
    pub fn param_source(&self, name: &str) -> Option<(NodeId, &str)> {
        self.nodes.iter().find_map(|(id, node)| {
            node.inputs.iter().find_map(|(port, input)| match input {
                Input::Link { node: from, output, .. }
                    if output == name && self.is_kind(*from, &NodeKind::Input) =>
                {
                    Some((*id, port.as_str()))
                }
                _ => None,
            })
        })
    }

    /// The input a port of a node has right now, if anything is set on it.
    pub fn input_of(&self, node: NodeId, port: &str) -> Option<&Input> {
        self.nodes.get(&node)?.inputs.get(port)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a graph is plain data")
    }

    /// Reads a graph back. Nodes this build does not know are kept as they
    /// are, they only matter once the graph is lowered.
    pub fn from_json(json: &str) -> Result<Self, FormatError> {
        let mut file: serde_json::Value = serde_json::from_str(json).map_err(FormatError::Syntax)?;
        let version = file.get("version").and_then(serde_json::Value::as_u64).unwrap_or(0) as u32;
        if version > FORMAT_VERSION {
            return Err(FormatError::Version(version));
        }
        if version < 2 {
            upgrade_exposed(&mut file);
        }
        let mut graph: Self = serde_json::from_value(file).map_err(FormatError::Syntax)?;
        graph.version = FORMAT_VERSION;
        Ok(graph)
    }
}

/// Format 1 to 2: every input exposed as a parameter becomes a link from
/// that port of a new input node, and the outputs get an output node.
fn upgrade_exposed(file: &mut serde_json::Value) {
    use serde_json::{Value as Json, json};

    let mut next = file["next_id"].as_u64().unwrap_or(0);
    let Some(nodes) = file.get_mut("nodes").and_then(Json::as_object_mut) else {
        return;
    };
    let (mut left, mut right, mut top) = (0.0f64, 0.0f64, f64::MAX);
    for node in nodes.values() {
        let x = node["meta"]["position"][0].as_f64().unwrap_or(0.0);
        let y = node["meta"]["position"][1].as_f64().unwrap_or(0.0);
        (left, right, top) = (left.min(x), right.max(x), top.min(y));
    }
    let top = if top == f64::MAX { 0.0 } else { top };

    // The parameters in the order they come up, each with the first value
    // it was given.
    let input = next;
    let mut params: Vec<(String, Option<Json>)> = Vec::new();
    for node in nodes.values_mut() {
        let Some(inputs) = node.get_mut("inputs").and_then(Json::as_object_mut) else {
            continue;
        };
        for value in inputs.values_mut() {
            let Some(exposed) = value.get("exposed").cloned() else {
                continue;
            };
            let name = exposed["name"].as_str().unwrap_or_default().to_string();
            match params.iter_mut().find(|(known, _)| *known == name) {
                Some((_, default @ None)) => *default = exposed.get("value").cloned(),
                Some(_) => {}
                None => params.push((name.clone(), exposed.get("value").cloned())),
            }
            *value = json!({ "link": { "node": input, "output": name } });
        }
    }
    if !params.is_empty() {
        nodes.insert(input.to_string(), json!({ "kind": "input", "meta": { "position": [left - 220.0, top] } }));
        next += 1;
    }
    let has_outputs = file.get("outputs").and_then(Json::as_array).is_some_and(|outputs| !outputs.is_empty());
    if has_outputs {
        let nodes = file["nodes"].as_object_mut().unwrap();
        nodes.insert(next.to_string(), json!({ "kind": "output", "meta": { "position": [right + 260.0, top] } }));
        next += 1;
    }
    file["next_id"] = json!(next);
    if !params.is_empty() {
        let params: Vec<Json> = params
            .into_iter()
            .map(|(name, default)| match default {
                Some(default) => json!({ "name": name, "default": default }),
                None => json!({ "name": name }),
            })
            .collect();
        file["params"] = Json::Array(params);
    }
}
