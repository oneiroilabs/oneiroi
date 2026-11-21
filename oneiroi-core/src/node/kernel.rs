//! The kernels a node brings: one for the CPU, and any for GPUs.

use crate::{node::{Inputs, Outputs}, types::TypeInfo};

pub type CpuKernel = fn(&Inputs<'_>, &mut Outputs<'_>);

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
    /// One per output, in the order of the outputs, then one per output
    /// which makes its own groups: the offsets of those, as `u32`, and then
    /// one per output of instances: their total, a single `u32` which has
    /// to be known on the CPU, see [`Inputs::total`].
    pub outputs: &'static [GpuOutput],
    pub requires: Caps,
    /// The inputs `len` and `accepts` read the values of, by their index
    /// among all inputs, the offsets behind the declared ones included. Of
    /// every other input they may only ask how long it is, since its
    /// values can be on the GPU only.
    pub reads: &'static [usize],
    /// Whether the kernel can compute the outputs for these inputs. The
    /// node runs on the CPU where it can not. Without it, it always can.
    pub accepts: Option<fn(&Inputs<'_>) -> bool>,
    /// Inputs to run both kernels with, which every test of a backend
    /// compares the results of. As words, for every input and then the
    /// offsets of every grouped one.
    pub examples: &'static [fn() -> Vec<Vec<u32>>],
    /// Results in between, made before the outputs in this order, which
    /// the passes after them read. The graph never sees them.
    pub stages: &'static [GpuStage],
}

/// A result in between of a [`GpuKernel`], made by a pass of its own:
/// `fn entry(index: u32) -> ty` for every element. Every pass after it
/// reads it with `stage_name(index)` and `stage_name_len()`.
#[derive(Debug, Clone, Copy)]
pub struct GpuStage {
    pub name: &'static str,
    pub ty: TypeInfo,
    /// Not used when the stage is made on the CPU.
    pub entry: &'static str,
    /// How many elements the stage has, asked before any pass runs. It may
    /// read what [`GpuKernel::reads`] lists.
    pub len: fn(&Inputs<'_>) -> usize,
    /// The words of the stage, made on the CPU instead of by a pass, from
    /// what [`GpuKernel::reads`] lists as well.
    pub words: Option<fn(&Inputs<'_>) -> Vec<u32>>,
}

#[derive(Debug, Clone, Copy)]
pub struct GpuOutput {
    pub entry: &'static str,
    /// How many elements the output will have. This is asked before the
    /// kernel runs, to make room for them.
    pub len: fn(&Inputs<'_>) -> usize,
    /// The words of the output, made on the CPU instead of by `entry`, from
    /// what `len` may read as well. For small outputs which are known
    /// before the kernel runs, like where the groups of a bigger one start:
    /// the CPU then has them without asking the device.
    pub words: Option<fn(&Inputs<'_>) -> Vec<u32>>,
}
