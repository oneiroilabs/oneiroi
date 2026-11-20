//! Turning a [`Graph`] into a [`Program`].

use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use oneiroi_core::{
    node::{Card, GenericNode, NodeDef, NodeRegistry, PortType},
    program::{BuildError, Program, ProgramBuilder, ProgramId, ProgramSet, SlotId},
    types::{Scalar, TypeInfo},
};

use crate::{Graph, Input, NodeId, NodeKind, Value};

/// What the ids and names inside a graph are looked up in.
#[derive(Clone, Copy)]
pub struct Context<'a> {
    pub registry: &'a NodeRegistry,
    pub programs: &'a ProgramSet,
    /// The programs of the graphs which can be used as nodes, by name.
    pub graphs: &'a HashMap<String, ProgramId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LowerError {
    /// The node the graph is broken at, to point at it in an editor.
    pub node: Option<NodeId>,
    pub port: Option<String>,
    pub kind: LowerErrorKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LowerErrorKind {
    /// No node with this id is registered.
    UnknownNode(String),
    /// No graph with this name is known.
    UnknownGraph(String),
    /// The node has no input with the name of the port.
    UnknownInput,
    /// A link comes from a node which is gone.
    MissingNode(NodeId),
    /// A link comes from an output its node does not have.
    UnknownOutput { node: NodeId, output: String },
    /// A node depends on itself.
    Cycle,
    /// The port takes a trait, so only a link can tell which type it has.
    NeedsLink,
    /// A written out value does not fit the type of the port.
    Value { expected: &'static str },
    /// The port takes a single value, and a list was written out for it.
    NotOne,
    /// The parameter has no type of its own and is linked to no port of
    /// one type it could take.
    ParamType(String),
    /// Only a node which is another graph can run for each element.
    EachOnNode,
    /// A graph runs for each element of one input, not of several.
    EachTwice,
    DuplicateOutput(String),
    Build(BuildError),
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(node) = self.node {
            write!(f, "node {}", node.0)?;
            if let Some(port) = &self.port {
                write!(f, ", `{port}`")?;
            }
            write!(f, ": ")?;
        }
        match &self.kind {
            LowerErrorKind::UnknownNode(id) => write!(f, "there is no node `{id}`"),
            LowerErrorKind::UnknownGraph(name) => write!(f, "there is no graph `{name}`"),
            LowerErrorKind::UnknownInput => write!(f, "the node has no such input"),
            LowerErrorKind::MissingNode(node) => {
                write!(f, "linked to node {} which is gone", node.0)
            }
            LowerErrorKind::UnknownOutput { node, output } => {
                write!(f, "node {} has no output `{output}`", node.0)
            }
            LowerErrorKind::Cycle => write!(f, "the node depends on itself"),
            LowerErrorKind::NeedsLink => write!(f, "something has to be connected here"),
            LowerErrorKind::Value { expected } => {
                write!(f, "the value is not a list of `{expected}`")
            }
            LowerErrorKind::NotOne => write!(f, "only a single value can go here"),
            LowerErrorKind::ParamType(name) => write!(
                f,
                "the parameter `{name}` needs a type of its own, or a link to a port of one type"
            ),
            LowerErrorKind::EachOnNode => {
                write!(f, "only a graph can run for each element")
            }
            LowerErrorKind::EachTwice => {
                write!(f, "a graph can run for each element of one input only")
            }
            LowerErrorKind::DuplicateOutput(name) => {
                write!(f, "there are two outputs `{name}`")
            }
            LowerErrorKind::Build(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for LowerError {}

impl Value {
    /// The value as elements of `ty`, if it is some.
    fn words(&self, ty: &TypeInfo) -> Option<&[u32]> {
        let words: &[u32] = match (self, ty.scalar) {
            (Self::F32(values), Scalar::F32) => bytemuck::cast_slice(values),
            (Self::U32(values), Scalar::U32) => values,
            _ => return None,
        };
        (words.len() % ty.words as usize == 0).then_some(words)
    }
}

/// What a node of the graph turned out to be.
#[derive(Clone, Copy)]
enum Target<'a> {
    Node(&'static NodeDef),
    Generic(&'static GenericNode),
    Graph(ProgramId, &'a Program),
}

struct Port {
    name: String,
    ty: PortType,
    card: Card,
    default: Vec<u32>,
}

impl Target<'_> {
    fn inputs(&self) -> Vec<Port> {
        match self {
            Self::Node(node) => node
                .inputs
                .iter()
                .map(|port| Port {
                    name: port.name.into(),
                    ty: PortType::Exact(port.ty),
                    card: port.card,
                    default: port.default.words().into(),
                })
                .collect(),
            Self::Generic(node) => node
                .inputs
                .iter()
                .map(|port| Port {
                    name: port.name.into(),
                    ty: port.ty,
                    card: port.card,
                    default: port.default.words().into(),
                })
                .collect(),
            // The offsets of grouped parameters go along with them, and
            // are no ports of their own.
            Self::Graph(_, program) => program
                .visible_params()
                .map(|param| Port {
                    name: param.name.clone(),
                    ty: PortType::Exact(program.slot(param.slot).ty),
                    card: program.slot(param.slot).card,
                    default: param.default.to_vec(),
                })
                .collect(),
        }
    }

    fn outputs(&self) -> Vec<String> {
        match self {
            Self::Node(node) => node.outputs.iter().map(|p| p.name.into()).collect(),
            Self::Generic(node) => node.outputs.iter().map(|p| p.name.into()).collect(),
            Self::Graph(_, program) => program.outputs.iter().map(|o| o.name.clone()).collect(),
        }
    }
}

struct Lowerer<'a> {
    graph: &'a Graph,
    context: Context<'a>,
    builder: ProgramBuilder,
    /// The outputs of every node that is lowered already, by name.
    done: HashMap<NodeId, Vec<(String, SlotId)>>,
    visiting: HashSet<NodeId>,
    /// The parameters of the graph, by name, as far as they have a type.
    params: Vec<(String, SlotId)>,
}

impl<'a> Lowerer<'a> {
    fn target(&self, kind: &NodeKind) -> Result<Target<'a>, LowerErrorKind> {
        match kind {
            NodeKind::Node(id) => {
                let registry = self.context.registry;
                if let Some(generic) = registry.generic(id) {
                    Ok(Target::Generic(generic))
                } else if let Some(node) = registry.get(id) {
                    Ok(Target::Node(node))
                } else {
                    Err(LowerErrorKind::UnknownNode(id.clone()))
                }
            }
            NodeKind::Graph(name) => match self.context.graphs.get(name) {
                Some(id) => Ok(Target::Graph(*id, self.context.programs.get(*id))),
                None => Err(LowerErrorKind::UnknownGraph(name.clone())),
            },
            // Their ports are the parameters and outputs of the graph, see
            // `Lowerer::node`.
            NodeKind::Input => Err(LowerErrorKind::UnknownNode("input".into())),
            NodeKind::Output => Err(LowerErrorKind::UnknownNode("output".into())),
        }
    }

    /// Makes every parameter of the graph, in their order, which has a
    /// type: its own, or the one of the first port it is linked to.
    fn params(&mut self) -> Result<(), LowerError> {
        for param in self.graph.params() {
            let at = |kind| LowerError {
                node: None,
                port: Some(param.name.clone()),
                kind,
            };
            let source = self.graph.param_source(&param.name).and_then(|(node, port)| {
                let node = self.graph.node(node)?;
                let target = self.target(&node.kind).ok()?;
                target.inputs().into_iter().find(|input| input.name == port)
            });
            let registry = self.context.registry;
            let (ty, card) = match (&param.ty, source.as_ref()) {
                (Some(name), _) => {
                    let ty = registry
                        .type_named(name)
                        .ok_or_else(|| at(LowerErrorKind::Value { expected: "a known type" }))?;
                    let card = param.card.or(source.as_ref().map(|port| port.card)).unwrap_or(Card::Many);
                    (ty, card)
                }
                (None, Some(Port { ty: PortType::Exact(ty), card, .. })) => (*ty, *card),
                // Nothing to tell its type by, so it can not be made. Where
                // something takes it anyway, that is an error there.
                (None, _) => continue,
            };
            let from_port = source.as_ref().filter(|port| port.ty == PortType::Exact(ty));
            let default: Vec<u32> = match &param.default {
                Some(value) => value
                    .words(&ty)
                    .ok_or_else(|| at(LowerErrorKind::Value { expected: ty.name }))?
                    .to_vec(),
                None => from_port.map(|port| port.default.clone()).unwrap_or_default(),
            };
            let default = match (card, default.is_empty()) {
                (Card::One, true) => vec![0; ty.words as usize],
                (Card::One, false) if default.len() != ty.words as usize => {
                    return Err(at(LowerErrorKind::NotOne));
                }
                _ => default,
            };
            let name = param.name.clone();
            let slot = match card {
                Card::One => self.builder.param_one(name.clone(), ty, &default),
                Card::Many => self.builder.param_words(name.clone(), ty, &default),
                Card::Grouped => self.builder.param_grouped(name.clone(), ty, &default),
            };
            self.params.push((name, slot));
        }
        Ok(())
    }

    /// Lowers `id` and everything it depends on.
    fn node(&mut self, id: NodeId) -> Result<(), LowerError> {
        if self.done.contains_key(&id) {
            return Ok(());
        }
        // The input node gives the parameters, the output node gives nothing.
        match self.graph.node(id).map(|node| &node.kind) {
            Some(NodeKind::Input) => {
                self.done.insert(id, self.params.clone());
                return Ok(());
            }
            Some(NodeKind::Output) => {
                self.done.insert(id, Vec::new());
                return Ok(());
            }
            _ => {}
        }
        let at = |port: Option<&str>, kind| LowerError {
            node: Some(id),
            port: port.map(Into::into),
            kind,
        };

        let node = self
            .graph
            .node(id)
            .ok_or_else(|| at(None, LowerErrorKind::MissingNode(id)))?;
        if !self.visiting.insert(id) {
            return Err(at(None, LowerErrorKind::Cycle));
        }
        let target = self.target(&node.kind).map_err(|kind| at(None, kind))?;

        let ports = target.inputs();
        if let Some(unknown) = node
            .inputs
            .keys()
            .find(|name| !ports.iter().any(|port| port.name == **name))
        {
            return Err(at(Some(unknown), LowerErrorKind::UnknownInput));
        }

        let mut slots = Vec::with_capacity(ports.len());
        // The input the node runs for each element of, if there is one.
        let mut each = None;
        for port in &ports {
            let exact = |kind| match port.ty {
                PortType::Exact(ty) => Ok(ty),
                PortType::Trait(_) => Err(at(Some(&port.name), kind)),
            };
            let words = |value: &'a Value, ty: &TypeInfo| {
                let words = value.words(ty).ok_or_else(|| {
                    at(Some(&port.name), LowerErrorKind::Value { expected: ty.name })
                })?;
                match port.card == Card::One && words.len() != ty.words as usize {
                    true => Err(at(Some(&port.name), LowerErrorKind::NotOne)),
                    false => Ok(words),
                }
            };

            let slot = match node.inputs.get(&port.name) {
                // A graph fills in its own defaults.
                None if matches!(target, Target::Graph(..)) => None,
                None => {
                    let ty = exact(LowerErrorKind::NeedsLink)?;
                    Some(self.builder.constant_words(ty, &port.default))
                }
                Some(Input::Value(value)) => {
                    let ty = exact(LowerErrorKind::NeedsLink)?;
                    Some(self.builder.constant_words(ty, words(value, &ty)?))
                }
                Some(Input::Link {
                    node,
                    output,
                    each: per_element,
                }) => {
                    if *per_element {
                        let kind = match (&target, each) {
                            (Target::Graph(..), None) => None,
                            (Target::Graph(..), Some(_)) => Some(LowerErrorKind::EachTwice),
                            _ => Some(LowerErrorKind::EachOnNode),
                        };
                        if let Some(kind) = kind {
                            return Err(at(Some(&port.name), kind));
                        }
                        each = Some(slots.len());
                    }
                    if self.graph.node(*node).is_none() {
                        return Err(at(Some(&port.name), LowerErrorKind::MissingNode(*node)));
                    }
                    self.node(*node)?;
                    let slot = self.done[node]
                        .iter()
                        .find(|(name, _)| name == output)
                        .map(|(_, slot)| *slot)
                        .ok_or_else(|| {
                            let kind = match self.graph.node(*node).map(|node| &node.kind) {
                                Some(NodeKind::Input) if self.graph.param(output).is_some() => {
                                    LowerErrorKind::ParamType(output.clone())
                                }
                                _ => LowerErrorKind::UnknownOutput {
                                    node: *node,
                                    output: output.clone(),
                                },
                            };
                            at(Some(&port.name), kind)
                        })?;
                    Some(self.realized(slot, port).map_err(|error| {
                        at(Some(&port.name), LowerErrorKind::Build(error))
                    })?)
                }
            };
            slots.push(slot);
        }

        let given = || slots.iter().map(|slot| slot.unwrap()).collect::<Vec<_>>();
        let outputs = match target {
            Target::Node(node) => self.builder.node(node, &given()),
            Target::Generic(node) => {
                self.builder
                    .generic(self.context.registry, node, &given())
            }
            Target::Graph(callee, _) => match each {
                Some(element) => self.builder.for_each(
                    self.context.programs,
                    callee,
                    &slots,
                    element,
                    crate::MAX_ELEMENTS,
                ),
                None => self.builder.call(self.context.programs, callee, &slots),
            },
        }
        .map_err(|error| {
            let port = match &error {
                BuildError::Type { port, .. }
                | BuildError::Trait { port, .. }
                | BuildError::Card { port, .. } => Some(port.clone()),
                _ => None,
            };
            at(port.as_deref(), LowerErrorKind::Build(error))
        })?;

        self.visiting.remove(&id);
        self.done
            .insert(id, target.outputs().into_iter().zip(outputs).collect());
        Ok(())
    }

    /// `slot` as `port` takes it: a value of a type the port does not take
    /// goes through a conversion into one it does, like instances linked
    /// to a port which takes their shape.
    fn realized(&mut self, slot: SlotId, port: &Port) -> Result<SlotId, BuildError> {
        let registry = self.context.registry;
        let ty = self.builder.slot_type(slot);
        let takes = |ty: &TypeInfo| match port.ty {
            PortType::Exact(expected) => expected == *ty,
            PortType::Trait(tr) => registry.implements(ty, tr),
        };
        if takes(&ty) {
            return Ok(slot);
        }
        match registry.conversion(&ty, takes) {
            Some((conversion, _)) => Ok(self.builder.generic(registry, conversion, &[slot])?[0]),
            None => Ok(slot),
        }
    }

    fn output(&mut self, name: &str, node: NodeId, output: &str) -> Result<(), LowerError> {
        self.node(node)?;
        let slot = self.done[&node]
            .iter()
            .find(|(name, _)| name == output)
            .map(|(_, slot)| *slot)
            .ok_or_else(|| LowerError {
                node: Some(node),
                port: None,
                kind: LowerErrorKind::UnknownOutput {
                    node,
                    output: output.into(),
                },
            })?;
        self.builder.output(name, slot);
        Ok(())
    }
}

impl Graph {
    fn lowerer<'a>(&'a self, context: Context<'a>) -> Lowerer<'a> {
        Lowerer {
            graph: self,
            context,
            builder: ProgramBuilder::new(self.name.clone()),
            done: HashMap::new(),
            visiting: HashSet::new(),
            params: Vec::new(),
        }
    }

    /// The program which computes the outputs of the graph.
    ///
    /// Only what those outputs depend on is looked at. A node nothing
    /// reads can be broken or unknown without getting in the way.
    pub fn lower(&self, context: Context<'_>) -> Result<Program, LowerError> {
        let mut lowerer = self.lowerer(context);
        lowerer.params()?;
        let mut names = HashSet::new();
        for output in self.outputs() {
            if !names.insert(&output.name) {
                return Err(LowerError {
                    node: Some(output.node),
                    port: None,
                    kind: LowerErrorKind::DuplicateOutput(output.name.clone()),
                });
            }
            lowerer.output(&output.name, output.node, &output.output)?;
        }
        Ok(lowerer.builder.finish())
    }

    /// The program which computes every output of one node, under the
    /// names of its ports. This is what a preview of that node runs.
    pub fn lower_node(&self, context: Context<'_>, node: NodeId) -> Result<Program, LowerError> {
        self.lower_preview(context, node, |_| true)
    }

    /// [`Graph::lower_node`] for a preview which can only show what
    /// `shown` says yes to: an output of another type which a conversion
    /// turns into one it can show is converted.
    pub fn lower_preview(
        &self,
        context: Context<'_>,
        node: NodeId,
        shown: impl Fn(&TypeInfo) -> bool,
    ) -> Result<Program, LowerError> {
        let registry = context.registry;
        let mut lowerer = self.lowerer(context);
        lowerer.params()?;
        // The output node shows every output of the graph.
        let shown_outputs = match self.node(node).map(|node| &node.kind) {
            Some(NodeKind::Output) => {
                let mut outputs = Vec::new();
                for output in self.outputs() {
                    let at = |kind| LowerError {
                        node: Some(output.node),
                        port: None,
                        kind,
                    };
                    lowerer.node(output.node)?;
                    let slot = lowerer.done[&output.node]
                        .iter()
                        .find(|(name, _)| *name == output.output)
                        .map(|(_, slot)| *slot)
                        .ok_or_else(|| {
                            at(LowerErrorKind::UnknownOutput {
                                node: output.node,
                                output: output.output.clone(),
                            })
                        })?;
                    outputs.push((output.name.clone(), slot));
                }
                outputs
            }
            _ => {
                lowerer.node(node)?;
                lowerer.done[&node].clone()
            }
        };
        for (name, slot) in shown_outputs {
            let ty = lowerer.builder.slot_type(slot);
            let conversion = match shown(&ty) {
                true => None,
                false => registry.conversion(&ty, &shown),
            };
            let slot = match conversion {
                Some((conversion, _)) => lowerer
                    .builder
                    .generic(registry, conversion, &[slot])
                    .map_err(|error| LowerError {
                        node: Some(node),
                        port: Some(name.clone()),
                        kind: LowerErrorKind::Build(error),
                    })?[0],
                None => slot,
            };
            lowerer.builder.output(name, slot);
        }
        Ok(lowerer.builder.finish())
    }
}
