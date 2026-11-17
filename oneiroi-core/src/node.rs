//! How a node describes itself to the core.
//!
//! A node is plain static data: its ports and one kernel per backend.
//! The CPU kernel is mandatory, GPU kernels are optional.

use std::collections::HashMap;

use crate::{
    store::{Handle, Store},
    types::{DataType, PortDefault, Trait, TypeInfo},
};

#[derive(Debug, Clone, Copy)]
pub struct PortDecl {
    pub name: &'static str,
    pub ty: TypeInfo,
    /// Only meaningful for inputs.
    pub default: PortDefault,
    pub card: Card,
    /// Only meaningful for grouped outputs.
    pub groups: Groups,
}

impl PortDecl {
    /// A port of many elements.
    pub const fn new<T: DataType>(name: &'static str) -> Self {
        Self {
            name,
            ty: T::INFO,
            default: PortDefault::Empty,
            card: Card::Many,
            groups: Groups::Own,
        }
    }

    pub const fn with_default(mut self, default: PortDefault) -> Self {
        self.default = default;
        self
    }

    /// A port of a single element.
    pub const fn one(mut self) -> Self {
        self.card = Card::One;
        self
    }

    /// An input which gets the groups of what is linked to it, or an
    /// output whose groups the node makes: see [`Outputs::set_groups`].
    pub const fn grouped(mut self) -> Self {
        self.card = Card::Grouped;
        self.groups = Groups::Own;
        self
    }

    /// An output which is grouped the same way as the input `input`: one
    /// element for each of its elements, or as many in every group.
    pub const fn grouped_like(mut self, input: usize) -> Self {
        self.card = Card::Grouped;
        self.groups = Groups::Like(input);
        self
    }
}

/// How many elements a port carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Card {
    /// A single one. Only something which is single can be linked to it.
    One,
    /// A list. Groups of what is linked to it are dropped.
    Many,
    /// A list of groups: elements with the offsets where every group starts.
    Grouped,
}

impl Card {
    /// Whether a value of this card can go where `to` is wanted.
    pub fn fits(self, to: Card) -> bool {
        to != Card::One || self == Card::One
    }
}

/// Where the groups of a grouped output come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Groups {
    /// The node makes them, as an output of its own.
    Own,
    /// They are the groups of an input, and nothing has to be made.
    Like(usize),
}

/// The inputs handed to a [`CpuKernel`], in the order of [`NodeDef::inputs`].
///
/// Grouped inputs come with their offsets: every group starts at an index
/// of the elements, and the last offset is where the last group ends.
/// No offsets at all means a single group of everything.
pub struct Inputs<'a> {
    pub(crate) store: &'a Store,
    pub(crate) ports: &'static [PortDecl],
    pub(crate) handles: &'a [Handle],
}

impl<'a> Inputs<'a> {
    pub fn new(store: &'a Store, node: &NodeDef, handles: &'a [Handle]) -> Self {
        Self {
            store,
            ports: node.inputs,
            handles,
        }
    }

    /// How many elements an input has. Unlike [`Inputs::get`] this never
    /// looks at the values.
    pub fn len(&self, index: usize) -> usize {
        (self.handles[index].len / self.ports[index].ty.words) as usize
    }

    pub fn get<T: DataType>(&self, index: usize) -> &'a [T] {
        debug_assert_eq!(self.ports[index].ty, T::INFO, "{}", self.ports[index].name);
        self.store.slice(self.handles[index])
    }

    /// The first element of an input, for ports which carry a single value.
    pub fn value<T: DataType>(&self, index: usize) -> T {
        self.get::<T>(index)
            .first()
            .copied()
            .unwrap_or_else(T::zeroed)
    }

    /// The offsets of a grouped input, if it has groups.
    pub fn offsets(&self, index: usize) -> Option<&'a [u32]> {
        let hidden = hidden_input(self.ports, index)?;
        let offsets: &[u32] = self.store.slice(self.handles[hidden]);
        (!offsets.is_empty()).then_some(offsets)
    }

    /// The groups of an input, as ranges of its elements. An input without
    /// groups is a single one.
    pub fn groups(&self, index: usize) -> Vec<std::ops::Range<usize>> {
        let len = self.len(index);
        match self.offsets(index) {
            Some(offsets) => offsets
                .windows(2)
                .map(|pair| (pair[0] as usize).min(len)..(pair[1] as usize).min(len))
                .collect(),
            None => vec![0..len],
        }
    }
}

/// Where the offsets of grouped input `index` are handed over: behind all
/// declared inputs, in the order of the grouped ones.
pub(crate) fn hidden_input(ports: &[PortDecl], index: usize) -> Option<usize> {
    (ports[index].card == Card::Grouped).then(|| {
        ports.len() + ports[..index].iter().filter(|p| p.card == Card::Grouped).count()
    })
}

/// Where the offsets of output `index` go, if the node makes them: behind
/// all declared outputs, in the order of the outputs which have their own.
pub(crate) fn hidden_output(ports: &[PortDecl], index: usize) -> Option<usize> {
    let own = |port: &PortDecl| port.card == Card::Grouped && port.groups == Groups::Own;
    own(&ports[index]).then(|| ports.len() + ports[..index].iter().filter(|p| own(p)).count())
}

/// The outputs a [`CpuKernel`] fills, in the order of [`NodeDef::outputs`].
/// An output which is not written stays empty.
pub struct Outputs {
    pub(crate) ports: &'static [PortDecl],
    pub(crate) buffers: Vec<Vec<u32>>,
}

impl Outputs {
    pub fn set<T: DataType>(&mut self, index: usize, values: &[T]) {
        self.alloc(index, values.len()).copy_from_slice(values);
    }

    /// Zeroed space for `len` elements to write the output in place.
    pub fn alloc<T: DataType>(&mut self, index: usize, len: usize) -> &mut [T] {
        debug_assert_eq!(self.ports[index].ty, T::INFO, "{}", self.ports[index].name);
        let buffer = &mut self.buffers[index];
        buffer.clear();
        buffer.resize(len * T::INFO.words as usize, 0);
        bytemuck::cast_slice_mut(buffer)
    }

    /// Where the groups of a grouped output start, and where the last one
    /// ends. Leaving them out makes a single group of everything.
    pub fn set_groups(&mut self, index: usize, offsets: &[u32]) {
        let hidden = hidden_output(self.ports, index)
            .expect("only an output which is grouped on its own has offsets to set");
        self.buffers[hidden] = offsets.to_vec();
    }
}

pub type CpuKernel = fn(&Inputs<'_>, &mut Outputs);

/// What a GPU kernel needs from the device it runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Caps {
    pub subgroups: bool,
    pub mesh_shaders: bool,
}

impl Caps {
    pub const NONE: Self = Self {
        subgroups: false,
        mesh_shaders: false,
    };

    pub const fn satisfies(self, required: Self) -> bool {
        (self.subgroups || !required.subgroups) && (self.mesh_shaders || !required.mesh_shaders)
    }
}

/// Premade shader functions implementing a node.
///
/// For every output there is a function `fn entry(index: u32) -> Type`
/// inside `wgsl` which computes one element of it. An element of an output
/// can be anything: one per input element (a map), several (an expansion),
/// or one per group which walks through the group (a scan within groups).
///
/// The inputs are read through functions which are generated around it,
/// per input port `name`: `in_name(index: u32) -> Type` and
/// `in_name_len() -> u32`, and for a grouped one also `in_name_groups()`,
/// `in_name_start(group)`, `in_name_end(group)` and `in_name_group_of(index)`.
/// See [`shader::compute`](crate::shader::compute).
#[derive(Debug, Clone, Copy)]
pub struct GpuKernel {
    pub wgsl: &'static str,
    /// One per output, in the order of the outputs, and then one per
    /// output which makes its own groups: the offsets of those, as `u32`.
    pub outputs: &'static [GpuOutput],
    pub requires: Caps,
    /// The inputs `len` reads the values of. Of every other input it may
    /// only ask how long it is, since its values can be on the GPU only.
    pub reads: &'static [usize],
    /// Inputs to run both kernels with, which every test of a backend
    /// compares the results of. As words, for every input and then the
    /// offsets of every grouped one.
    pub examples: &'static [fn() -> Vec<Vec<u32>>],
}

#[derive(Debug, Clone, Copy)]
pub struct GpuOutput {
    pub entry: &'static str,
    /// How many elements the output will have. This is asked before the
    /// kernel runs, to make room for them.
    pub len: fn(&Inputs<'_>) -> usize,
}

pub struct NodeDef {
    /// Unique and stable. It is part of every [`ContentKey`] the node
    /// produces, so it has to change when the node's results change.
    ///
    /// [`ContentKey`]: crate::store::ContentKey
    pub id: &'static str,
    pub display_name: &'static str,
    /// Counts up whenever what the node computes changes. Results of an
    /// older version are never mistaken for ones of the newer.
    pub version: u32,
    pub inputs: &'static [PortDecl],
    pub outputs: &'static [PortDecl],
    pub cpu: CpuKernel,
    /// Variants of the same kernel, the first one the device satisfies wins.
    pub gpu: &'static [GpuKernel],
}

impl NodeDef {
    pub fn gpu_kernel(&self, caps: Caps) -> Option<&'static GpuKernel> {
        self.gpu.iter().find(|kernel| caps.satisfies(kernel.requires))
    }

    /// The offsets which are handed over behind the declared inputs: one
    /// for every grouped input, by its index.
    pub fn hidden_inputs(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.inputs.len()).filter(|index| hidden_input(self.inputs, *index).is_some())
    }

    /// The offsets which are made behind the declared outputs: one for
    /// every output which makes its own groups, by its index.
    pub fn hidden_outputs(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.outputs.len()).filter(|index| hidden_output(self.outputs, *index).is_some())
    }
}

impl std::fmt::Debug for NodeDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeDef").field("id", &self.id).finish()
    }
}

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
}

impl GenericPort {
    pub const fn exact<T: DataType>(name: &'static str) -> Self {
        Self {
            name,
            ty: PortType::Exact(T::INFO),
            default: PortDefault::Empty,
            card: Card::Many,
        }
    }

    pub const fn implementing(name: &'static str, tr: Trait) -> Self {
        Self {
            name,
            ty: PortType::Trait(tr),
            default: PortDefault::Empty,
            card: Card::Many,
        }
    }

    pub const fn with_default(mut self, default: PortDefault) -> Self {
        self.default = default;
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
    pub display_name: &'static str,
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
    pub bounds: fn(words: &[u32]) -> Option<([f32; 3], f32)>,
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
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Panics when a different node is already registered under the same id.
    pub fn register(&mut self, node: &'static NodeDef) {
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
