//! How a node describes itself to the core.
//!
//! A node is plain static data: its ports and one kernel per backend.
//! The CPU kernel is mandatory, GPU kernels are optional.

use std::collections::HashMap;

use crate::{
    store::{Handle, Store},
    types::{DataType, TypeInfo},
};

#[derive(Debug, Clone, Copy)]
pub struct PortDecl {
    pub name: &'static str,
    pub ty: TypeInfo,
}

impl PortDecl {
    pub const fn new<T: DataType>(name: &'static str) -> Self {
        Self {
            name,
            ty: T::INFO,
        }
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

/// All nodes known to an application, by id.
#[derive(Default)]
pub struct NodeRegistry {
    nodes: HashMap<&'static str, &'static NodeDef>,
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

    pub fn get(&self, id: &str) -> Option<&'static NodeDef> {
        self.nodes.get(id).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = &'static NodeDef> {
        self.nodes.values().copied()
    }
}
