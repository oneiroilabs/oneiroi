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
}

impl PortDecl {
    pub const fn new<T: DataType>(name: &'static str) -> Self {
        Self {
            name,
            ty: T::INFO,
            default: PortDefault::Empty,
        }
    }

    pub const fn with_default(mut self, default: PortDefault) -> Self {
        self.default = default;
        self
    }
}

/// How the elements of a node's outputs depend on the elements of its
/// inputs. This is what a GPU backend plans its passes with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// One output element per input element, each independent of the others.
    Map,
    /// Several output elements per input element.
    Expand,
    /// An output element depends on the ones before it.
    Scan,
    /// Many input elements collapse into one output element.
    Reduce,
}

/// The inputs handed to a [`CpuKernel`], in the order of [`NodeDef::inputs`].
pub struct Inputs<'a> {
    pub(crate) store: &'a Store,
    pub(crate) ports: &'static [PortDecl],
    pub(crate) handles: &'a [Handle],
}

impl<'a> Inputs<'a> {
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

/// A premade shader function implementing a node.
#[derive(Debug, Clone, Copy)]
pub struct GpuKernel {
    pub wgsl: &'static str,
    pub entry: &'static str,
    pub requires: Caps,
}

pub struct NodeDef {
    /// Unique and stable. It is part of every [`ContentKey`] the node
    /// produces, so it has to change when the node's results change.
    ///
    /// [`ContentKey`]: crate::store::ContentKey
    pub id: &'static str,
    pub display_name: &'static str,
    pub inputs: &'static [PortDecl],
    pub outputs: &'static [PortDecl],
    pub shape: Shape,
    pub cpu: CpuKernel,
    /// Variants of the same kernel, the first one the device satisfies wins.
    pub gpu: &'static [GpuKernel],
}

impl NodeDef {
    pub fn gpu_kernel(&self, caps: Caps) -> Option<&'static GpuKernel> {
        self.gpu.iter().find(|kernel| caps.satisfies(kernel.requires))
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
}

impl GenericPort {
    pub const fn exact<T: DataType>(name: &'static str) -> Self {
        Self {
            name,
            ty: PortType::Exact(T::INFO),
            default: PortDefault::Empty,
        }
    }

    pub const fn implementing(name: &'static str, tr: Trait) -> Self {
        Self {
            name,
            ty: PortType::Trait(tr),
            default: PortDefault::Empty,
        }
    }

    pub const fn with_default(mut self, default: PortDefault) -> Self {
        self.default = default;
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

struct Generic {
    node: &'static GenericNode,
    instances: Vec<&'static NodeDef>,
}

/// All nodes known to an application, by id.
#[derive(Default)]
pub struct NodeRegistry {
    nodes: HashMap<&'static str, &'static NodeDef>,
    generics: HashMap<&'static str, Generic>,
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
