//! Every node and type an application knows, generic nodes which are made
//! for each type they are used with, and how types are drawn.

use std::collections::HashMap;

use crate::{node::{Card, NodeDef, PortDecl}, types::{DataType, PortDefault, Trait, TypeInfo}};

/// What a port of a [`GenericNode`] accepts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PortType {
    /// Exactly this data type.
    Exact(TypeInfo),
    /// Any data type which implements the trait.
    Trait(Trait),
}

#[derive(Debug, Clone, Copy)]
pub struct GenericPort {
    pub name: &'static str,
    pub ty: PortType,
    /// Only meaningful for inputs with an exact type.
    pub default: PortDefault,
    /// Every version has the same. Where the groups of an output come
    /// from can differ between the versions.
    pub card: Card,
    /// See [`PortDecl::choices`]. Every version has the same.
    pub choices: &'static [&'static str],
}

impl GenericPort {
    pub const fn exact<T: DataType>(name: &'static str) -> Self {
        Self {
            name,
            ty: PortType::Exact(T::INFO),
            default: PortDefault::EMPTY,
            card: Card::Many,
            choices: &[],
        }
    }

    pub const fn implementing(name: &'static str, tr: Trait) -> Self {
        Self {
            name,
            ty: PortType::Trait(tr),
            default: PortDefault::EMPTY,
            card: Card::Many,
            choices: &[],
        }
    }

    /// See [`PortDecl::with_default`]. A port of a trait takes a default
    /// of any type, which only versions of that type use.
    pub const fn with_default<T: DataType>(mut self, values: &'static [T]) -> Self {
        self.default = PortDefault::of(values);
        if let PortType::Exact(ty) = &self.ty {
            assert!(self.default.fits(ty), "a default must be of the type of its port");
        }
        self
    }

    pub const fn one(mut self) -> Self {
        self.card = Card::One;
        self
    }

    pub const fn grouped(mut self) -> Self {
        self.card = Card::Grouped;
        self
    }

    /// See [`PortDecl::choices`].
    pub const fn choices(mut self, choices: &'static [&'static str]) -> Self {
        self.card = Card::One;
        self.choices = choices;
        if self.default.is_empty() {
            self.default = PortDefault::of::<u32>(&[0]);
        }
        self
    }
}

/// A node as its author sees it: some of its ports take a [`Trait`]
/// instead of one data type.
///
/// It has no kernels of its own. Behind it stands one ordinary [`NodeDef`]
/// per combination of data types it supports, and lowering a graph picks
/// the one that fits what is connected. Nothing is decided at runtime.
#[derive(Debug)]
pub struct GenericNode {
    pub id: &'static str,
    pub inputs: &'static [GenericPort],
    pub outputs: &'static [GenericPort],
}

/// How the elements of a [`Drawable`] type turn into something visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topology {
    /// Every element is a patch of a surface: a point for every `uv` in
    /// the unit square.
    Surface,
    /// Every element is a piece of a line: a point for every `u` in `0..=1`.
    Line,
}

/// A data type a backend can draw as it is, without anybody having turned
/// it into triangles first.
///
/// `wgsl` holds the function which says where the points of an element
/// are. For a [`Topology::Surface`] that is
/// `fn surface_point(piece: Type, uv: vec2<f32>) -> SurfacePoint`, and
/// for a [`Topology::Line`] `fn line_point(piece: Type, u: f32) -> vec3<f32>`.
/// See [`shader::draw`](crate::shader::draw).
#[derive(Debug, Clone, Copy)]
pub struct Drawable {
    pub ty: TypeInfo,
    pub topology: Topology,
    pub wgsl: &'static str,
    /// Center and radius of a sphere around all of `words`, which are
    /// elements of the type. `None` if there is nothing to be around.
    pub bounds: fn(words: &[u32]) -> Option<(glam::Vec3, f32)>,
}

struct Generic {
    node: &'static GenericNode,
    instances: Vec<&'static NodeDef>,
}

/// All nodes known to an application, by id.
#[derive(Default)]
pub struct NodeRegistry {
    nodes: HashMap<&'static str, &'static NodeDef>,
    generics: HashMap<&'static str, Generic>,
    /// By the name of the data type.
    drawables: HashMap<&'static str, Drawable>,
    /// Nodes which are never placed by hand, by id.
    hidden: std::collections::HashSet<&'static str>,
    /// What turns a value into one of another type where only that one is
    /// taken, in the order they were added. See [`NodeRegistry::add_conversion`].
    conversions: Vec<&'static GenericNode>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Panics when a different node is already registered under the same id.
    pub fn register(&mut self, node: &'static NodeDef) {
        for port in node.inputs.iter().filter(|port| !port.choices.is_empty()) {
            assert!(
                port.ty == u32::INFO && port.card == Card::One,
                "`{}` of `{}` has choices, so it has to be a single `u32`",
                port.name,
                node.id
            );
        }
        if let Some(existing) = self.nodes.insert(node.id, node) {
            assert!(
                std::ptr::eq(existing, node),
                "two nodes share the id `{}`",
                node.id
            );
        }
    }

    /// Registers `node` as the version of `generic` for the data types of
    /// its ports. Those types implement the traits of the ports from now on.
    ///
    /// Panics when the ports of the two do not line up.
    pub fn register_instance(&mut self, generic: &'static GenericNode, node: &'static NodeDef) {
        let lines_up = |generic: &[GenericPort], node: &[PortDecl]| {
            generic.len() == node.len()
                && generic.iter().zip(node).all(|(generic, port)| {
                    generic.name == port.name
                        && generic.card == port.card
                        && generic.choices == port.choices
                        && match generic.ty {
                            PortType::Exact(ty) => ty == port.ty,
                            PortType::Trait(_) => true,
                        }
                })
        };
        assert!(
            lines_up(generic.inputs, node.inputs) && lines_up(generic.outputs, node.outputs),
            "`{}` is not a version of `{}`",
            node.id,
            generic.id
        );

        self.register(node);
        let entry = self.generics.entry(generic.id).or_insert(Generic {
            node: generic,
            instances: Vec::new(),
        });
        assert!(
            std::ptr::eq(entry.node, generic),
            "two generic nodes share the id `{}`",
            generic.id
        );
        if !entry.instances.iter().any(|known| std::ptr::eq(*known, node)) {
            entry.instances.push(node);
        }
    }

    /// Says which generic node copies instances back into their shape: it
    /// takes instances as its only input and gives the shape. It is put in
    /// front of a port which takes a shape where instances are linked.
    /// Lets `conversion` turn values into others implicitly: a value
    /// linked to a port which does not take it goes through the version of
    /// `conversion` whose input is its type and whose output the port takes.
    /// Copying instances back into their shape is one, making a curve of a
    /// circle another.
    ///
    /// Panics unless `conversion` has a single input and a single output.
    pub fn add_conversion(&mut self, conversion: &'static GenericNode) {
        assert!(
            conversion.inputs.len() == 1 && conversion.outputs.len() == 1,
            "`{}` takes one value and makes one, or it is no conversion",
            conversion.id
        );
        if !self.conversions.iter().any(|known| std::ptr::eq(*known, conversion)) {
            self.conversions.push(conversion);
        }
    }

    /// The conversion of values of `from` into a type `takes` says yes to,
    /// the first one there is, and that type.
    pub fn conversion(
        &self,
        from: &TypeInfo,
        takes: impl Fn(&TypeInfo) -> bool,
    ) -> Option<(&'static GenericNode, TypeInfo)> {
        self.conversions.iter().find_map(|conversion| {
            self.instances(conversion)
                .iter()
                .find(|node| node.inputs[0].ty == *from && takes(&node.outputs[0].ty))
                .map(|node| (*conversion, node.outputs[0].ty))
        })
    }

    /// Whether values of `ty` can be turned into anything else.
    pub fn converts(&self, ty: &TypeInfo) -> bool {
        self.conversion(ty, |_| true).is_some()
    }

    /// The data type called `name`, if any port of a registered node has
    /// it.
    pub fn type_named(&self, name: &str) -> Option<TypeInfo> {
        self.nodes
            .values()
            .flat_map(|node| node.inputs.iter().chain(node.outputs))
            .map(|port| port.ty)
            .find(|ty| ty.name == name)
    }

    /// Says how values of a data type can be drawn.
    pub fn register_drawable(&mut self, drawable: Drawable) {
        self.drawables.insert(drawable.ty.name, drawable);
    }

    /// Every data type which can be drawn as it is.
    pub fn drawables(&self) -> impl Iterator<Item = &Drawable> {
        self.drawables.values()
    }

    /// How values of `ty` can be drawn, if they can.
    pub fn drawable(&self, ty: &TypeInfo) -> Option<&Drawable> {
        self.drawables.get(ty.name).filter(|drawable| drawable.ty == *ty)
    }

    pub fn get(&self, id: &str) -> Option<&'static NodeDef> {
        self.nodes.get(id).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = &'static NodeDef> {
        self.nodes.values().copied()
    }

    pub fn generic(&self, id: &str) -> Option<&'static GenericNode> {
        self.generics.get(id).map(|generic| generic.node)
    }

    pub fn generics(&self) -> impl Iterator<Item = &'static GenericNode> {
        self.generics.values().map(|generic| generic.node)
    }

    /// Keeps the node or generic node with this id out of the
    /// [palette](NodeRegistry::palette). It can still be used, by lowering
    /// or by a host which inserts it where it is needed, and it is still
    /// found by its id when a file names it.
    pub fn hide(&mut self, id: &'static str) {
        self.hidden.insert(id);
    }

    pub fn is_hidden(&self, id: &str) -> bool {
        self.hidden.contains(id)
    }

    /// The ids of everything an artist places by hand, sorted: the generic
    /// nodes, and the nodes which are no version of one, unless hidden.
    pub fn palette(&self) -> Vec<&'static str> {
        let versions: std::collections::HashSet<_> = self
            .generics
            .values()
            .flat_map(|generic| generic.instances.iter().map(|node| node.id))
            .collect();
        let generics = self.generics.keys().copied();
        let plain = self.nodes.keys().copied().filter(|id| !versions.contains(id));
        let mut ids: Vec<_> = generics.chain(plain).filter(|id| !self.is_hidden(id)).collect();
        ids.sort_unstable();
        ids
    }

    /// Every registered version of a generic node.
    pub fn instances(&self, generic: &GenericNode) -> &[&'static NodeDef] {
        self.generics
            .get(generic.id)
            .map_or(&[], |generic| generic.instances.as_slice())
    }

    /// Every data type that implements `tr`, in no particular order.
    pub fn implementors(&self, tr: Trait) -> Vec<TypeInfo> {
        let mut types: Vec<TypeInfo> = Vec::new();
        for generic in self.generics.values() {
            for instance in &generic.instances {
                let implementing = generic
                    .node
                    .inputs
                    .iter()
                    .zip(instance.inputs)
                    .chain(generic.node.outputs.iter().zip(instance.outputs))
                    .filter(|(port, _)| port.ty == PortType::Trait(tr))
                    .map(|(_, port)| port.ty);
                for ty in implementing {
                    if !types.contains(&ty) {
                        types.push(ty);
                    }
                }
            }
        }
        types
    }

    pub fn implements(&self, ty: &TypeInfo, tr: Trait) -> bool {
        self.implementors(tr).contains(ty)
    }
}
