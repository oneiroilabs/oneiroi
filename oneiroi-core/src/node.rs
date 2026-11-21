//! How a node describes itself to the core.
//!
//! A node is plain static data: its ports and one kernel per backend.
//! The CPU kernel is mandatory, GPU kernels are optional.

mod io;
mod kernel;
mod port;
mod registry;
#[cfg(test)]
mod tests;

pub use io::{Inputs, Outputs};
pub use kernel::{Caps, CpuKernel, GpuKernel, GpuOutput, GpuStage};
pub(crate) use port::{hidden_input, hidden_output, hidden_prototype, hidden_total};
pub use port::{Card, Groups, Items, PortDecl};
pub use registry::{Drawable, GenericNode, GenericPort, NodeRegistry, PortType, Topology};

pub struct NodeDef {
    /// Unique and stable. It is part of every [`ContentKey`] the node
    /// produces, so it has to change when the node's results change.
    ///
    /// [`ContentKey`]: crate::store::ContentKey
    pub id: &'static str,
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

    /// The inputs of instances, whose shapes are handed over behind the
    /// offsets of the grouped ones: the shape, its offsets and the total of
    /// the instances each.
    pub fn prototype_inputs(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.inputs.len()).filter(|index| self.inputs[*index].ty.prototype.is_some())
    }

    /// How many inputs a call of the node gets: the declared ones and
    /// everything which comes along with them.
    pub fn all_inputs(&self) -> usize {
        self.inputs.len() + self.hidden_inputs().count() + 3 * self.prototype_inputs().count()
    }

    /// The offsets which are made behind the declared outputs: one for
    /// every output which makes its own groups, by its index.
    pub fn hidden_outputs(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.outputs.len()).filter(|index| hidden_output(self.outputs, *index).is_some())
    }

    /// The outputs of instances, whose totals are made behind the offsets
    /// of the outputs with groups of their own.
    pub fn total_outputs(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.outputs.len()).filter(|index| self.outputs[*index].ty.prototype.is_some())
    }

    /// How many outputs a call of the node makes: the declared ones, the
    /// offsets of those with groups of their own, and the totals of
    /// instances. All of them but the declared ones are single `u32`s or
    /// lists of them.
    pub fn all_outputs(&self) -> usize {
        self.outputs.len() + self.hidden_outputs().count() + self.total_outputs().count()
    }
}

impl std::fmt::Debug for NodeDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeDef").field("id", &self.id).finish()
    }
}
