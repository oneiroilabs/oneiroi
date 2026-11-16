//! The graph as it is authored, saved and loaded.
//!
//! A [`Graph`] only knows the ids of its nodes and how they are wired. It
//! is turned into something that runs by [`Graph::lower`], which looks the
//! ids up in a registry and builds a program out of them.
//!
//! Next to the wiring a graph carries what every editor wants to keep with
//! it, like the position of a node. None of that takes part in lowering.

use std::{
    collections::{BTreeMap, HashSet},
    fmt,
};

use serde::{Deserialize, Serialize};

mod library;
mod lower;

pub use library::{Library, LibraryError};
pub use lower::{Context, LowerError, LowerErrorKind};

/// The version of the file format this crate writes.
pub const FORMAT_VERSION: u32 = 1;

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
}

/// Where an input of a node gets its value from. An input without an entry
/// has the default of its port.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Input {
    /// Fixed for every instance of the graph.
    Value(Value),
    /// A parameter of the graph. Inputs exposed under the same name are the
    /// same parameter. Without a value it starts at the default of the port.
    Exposed {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value: Option<Value>,
    },
    /// The output of another node.
    Link { node: NodeId, output: String },
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

/// An output of a node which is an output of the whole graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphOutput {
    pub name: String,
    pub node: NodeId,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    version: u32,
    pub name: String,
    nodes: BTreeMap<NodeId, Node>,
    next_id: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    outputs: Vec<GraphOutput>,
    /// The node an editor shows in its preview.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    MissingNode(NodeId),
    /// The link would make a node depend on itself.
    Cycle,
    DuplicateOutput(String),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(f, "there is no node {}", node.0),
            Self::Cycle => write!(f, "a node can not depend on itself"),
            Self::DuplicateOutput(name) => write!(f, "there already is an output `{name}`"),
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
            outputs: Vec::new(),
            display: None,
        }
    }

    pub fn add_node(&mut self, kind: NodeKind) -> NodeId {
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

    /// Removes a node together with every link from it and every output of
    /// the graph it provides.
    pub fn remove_node(&mut self, id: NodeId) -> Option<Node> {
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

    /// Makes an input a parameter of the graph called `name`.
    pub fn expose(
        &mut self,
        node: NodeId,
        port: &str,
        name: &str,
        value: Option<Value>,
    ) -> Result<(), EditError> {
        let name = name.into();
        self.input(node, port, Input::Exposed { name, value })
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
        if self.depends_on(from, to) {
            return Err(EditError::Cycle);
        }
        let link = Input::Link {
            node: from,
            output: output.into(),
        };
        self.input(to, input, link)
    }

    /// Puts an input back to the default of its port.
    pub fn reset(&mut self, node: NodeId, port: &str) -> Option<Input> {
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

    /// The names of the graphs this one uses.
    pub fn dependencies(&self) -> impl Iterator<Item = &str> {
        self.nodes.values().filter_map(|node| match &node.kind {
            NodeKind::Graph(name) => Some(name.as_str()),
            NodeKind::Node(_) => None,
        })
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a graph is plain data")
    }

    /// Reads a graph back. Nodes this build does not know are kept as they
    /// are, they only matter once the graph is lowered.
    pub fn from_json(json: &str) -> Result<Self, FormatError> {
        let graph: Self = serde_json::from_str(json).map_err(FormatError::Syntax)?;
        if graph.version > FORMAT_VERSION {
            return Err(FormatError::Version(graph.version));
        }
        Ok(graph)
    }
}
