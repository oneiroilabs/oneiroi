//! The static, compiled form of a graph.
//!
//! A [`Program`] is built once per graph definition and shared by all of
//! its instances. A graph inside a graph is never copied into its parent:
//! the parent holds a call to the other program, so every definition
//! exists exactly once in a [`ProgramSet`].

mod builder;
mod fold;
mod set;
#[cfg(test)]
mod tests;

pub use builder::{BuildError, ProgramBuilder};
pub use set::ProgramSet;

use std::hash::Hash;

use crate::{node::{Card, Items, NodeDef}, store::ContentKey, types::TypeInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotId(pub(crate) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProgramId(pub(crate) u32);

/// Where the value of a slot comes from.
#[derive(Debug)]
pub enum SlotSource {
    /// Supplied from outside: by an instance, or by the calling program.
    Param(u32),
    /// Baked into the program.
    Const { words: Box<[u32]>, key: ContentKey },
    /// The `output`-th result of the `op`-th operation.
    Op { op: u32, output: u32 },
}

#[derive(Debug)]
pub struct SlotDecl {
    pub ty: TypeInfo,
    pub source: SlotSource,
    pub card: Card,
    /// The offsets of its groups, a `u32` slot, if it is grouped.
    pub groups: Option<SlotId>,
    /// For instances: the shape they refer to.
    pub prototype: Option<Prototype>,
    /// For instances: how many elements all of their copies have together,
    /// a single `u32`.
    pub total: Option<SlotId>,
}

/// The shape instances refer to: a value of the type of their prototype
/// and its offsets. These are the slots of the value itself, nothing is
/// copied for the instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Prototype {
    pub elements: SlotId,
    pub groups: SlotId,
}

#[derive(Debug)]
pub struct ParamDecl {
    pub name: String,
    pub slot: SlotId,
    pub default: Box<[u32]>,
    /// Holds the offsets of another parameter. Nobody sets it by name:
    /// it comes along with what is passed for that one.
    pub hidden: bool,
}

#[derive(Debug)]
pub struct OutputDecl {
    pub name: String,
    pub slot: SlotId,
}

/// The name of the hidden parameter which holds the groups of `param`.
pub fn groups_param(param: &str) -> String {
    format!("{param}#groups")
}

/// The name of the hidden parameter which holds the shape of `param`, a
/// parameter of instances. Its offsets are in [`groups_param`] of it.
pub fn prototype_param(param: &str) -> String {
    format!("{param}#prototype")
}

/// The name of the hidden parameter which holds the total of `param`, a
/// parameter of instances.
pub fn total_param(param: &str) -> String {
    format!("{param}#total")
}

#[derive(Debug, Clone, Copy)]
pub enum OpKind {
    Node(&'static NodeDef),
    /// Runs another program. `inputs` are its parameters, in order.
    Call(ProgramId),
    /// Runs another program once for every element of its parameter
    /// `element`, with that element alone as the value of it, or once for
    /// every group if what is passed for it is grouped. Every output is
    /// what all of the runs gave, one group per run. At most `max` runs are
    /// made, the rest is left out and reported.
    ///
    /// Its inputs are those of a call, and then the offsets of what the
    /// elements are taken of. Its outputs are those of a call, and then
    /// the offsets of every one of them.
    ForEach {
        callee: ProgramId,
        element: u32,
        max: u32,
    },
}

#[derive(Debug)]
pub struct Op {
    pub kind: OpKind,
    pub inputs: Box<[SlotId]>,
    pub outputs: Box<[SlotId]>,
}

#[derive(Debug)]
pub struct Program {
    pub name: String,
    /// Identifies the structure of the program, not its parameter values.
    pub hash: u64,
    pub slots: Box<[SlotDecl]>,
    pub params: Box<[ParamDecl]>,
    /// In dependency order: an operation only reads earlier slots.
    pub ops: Box<[Op]>,
    pub outputs: Box<[OutputDecl]>,
}

impl Program {
    pub fn param_index(&self, name: &str) -> Option<usize> {
        self.params.iter().position(|param| param.name == name)
    }

    pub fn output_index(&self, name: &str) -> Option<usize> {
        self.outputs.iter().position(|output| output.name == name)
    }

    pub fn slot(&self, slot: SlotId) -> &SlotDecl {
        &self.slots[slot.0 as usize]
    }

    /// How many things `slot` holds, see [`Card::items`].
    pub fn items(&self, slot: SlotId) -> Items {
        let slot = self.slot(slot);
        slot.card.items(&slot.ty)
    }

    /// The parameters somebody sets, without the hidden ones.
    pub fn visible_params(&self) -> impl Iterator<Item = &ParamDecl> {
        self.params.iter().filter(|param| !param.hidden)
    }

    /// The slot of the callee which output `index` of a call to it is: one
    /// of its outputs, behind those the offsets of a grouped one, and then
    /// the shape and its offsets of one which is instances.
    pub fn call_output_slot(&self, index: usize) -> SlotId {
        self.call_output_slots()
            .nth(index)
            .expect("a call has outputs for the callee's outputs and what comes along")
    }

    /// Every slot a call of this program gives back, see
    /// [`Program::call_output_slot`].
    fn call_output_slots(&self) -> impl Iterator<Item = SlotId> + '_ {
        let slots = self.outputs.iter().map(|output| self.slot(output.slot));
        self.outputs
            .iter()
            .map(|output| output.slot)
            .chain(slots.clone().filter_map(|slot| slot.groups))
            .chain(slots.flat_map(|slot| {
                let along = slot.prototype.zip(slot.total);
                along.map(|(p, total)| [p.elements, p.groups, total]).into_iter().flatten()
            }))
    }

    /// The hidden parameters which hold the shape of `param` and its
    /// offsets, if it is a parameter of instances.
    pub fn prototype_of_param(&self, param: usize) -> Option<(usize, usize)> {
        let name = prototype_param(&self.params[param].name);
        Some((self.param_index(&name)?, self.param_index(&groups_param(&name))?))
    }

    /// The hidden parameter which holds the total of `param`, if it is a
    /// parameter of instances.
    pub fn total_of_param(&self, param: usize) -> Option<usize> {
        self.param_index(&total_param(&self.params[param].name))
    }

    /// The hidden parameter which holds the groups of `param`, if any.
    pub fn groups_of_param(&self, param: usize) -> Option<usize> {
        self.param_index(&groups_param(&self.params[param].name))
    }
}
