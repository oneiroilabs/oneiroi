//! The keys of the slots of a program for the values of its parameters:
//! derived once, and again only where a parameter changed.

use std::sync::Arc;

use crate::{
    engine::constant::Statics,
    program::{OpKind, Program, ProgramSet, SlotSource},
    store::ContentKey,
};

/// The keys of every slot of one program for one set of parameter values.
#[derive(Default, Clone)]
pub(super) struct Keys {
    pub(super) slots: Vec<ContentKey>,
    /// The keys inside each program this one calls, by operation index.
    pub(super) calls: Vec<(u32, Arc<Keys>)>,
}

impl Keys {
    pub(super) fn derive(programs: &ProgramSet, program: &Program, params: &[ContentKey]) -> Self {
        let mut keys = Self::default();
        keys.rederive(programs, program, params);
        keys
    }

    /// Derives the keys anew in the room these had, for one call after
    /// another of the same program.
    pub(super) fn rederive(&mut self, programs: &ProgramSet, program: &Program, params: &[ContentKey]) {
        self.slots.clear();
        self.slots.extend(program.slots.iter().map(|slot| match &slot.source {
            SlotSource::Param(index) => params[*index as usize],
            SlotSource::Const { key, .. } => *key,
            // Filled in below, operations only read earlier slots.
            SlotSource::Op { .. } => ContentKey::UNSET,
        }));
        self.calls.clear();
        for index in 0..program.ops.len() {
            self.derive_op(programs, program, index);
        }
    }

    /// The keys of one instance of the program of `statics`: those of its
    /// constant part as they are, and the rest derived from `params`.
    pub(super) fn of_instance(statics: &Statics, programs: &ProgramSet, program: &Program, params: &[ContentKey]) -> Self {
        let mut keys = Self {
            slots: statics.keys.clone(),
            calls: statics.calls.clone(),
        };
        for (index, slot) in program.params.iter().map(|param| param.slot).enumerate() {
            keys.slots[slot.0 as usize] = params[index];
        }
        for op in &statics.dynamic_ops {
            keys.derive_op(programs, program, *op as usize);
        }
        keys.calls.sort_unstable_by_key(|(op, _)| *op);
        keys
    }

    /// The keys after the parameters `changed` changed to what `params`
    /// says: only the operations which depend on one of them again.
    pub(super) fn update(
        &mut self,
        statics: &Statics,
        programs: &ProgramSet,
        program: &Program,
        params: &[ContentKey],
        changed: &[usize],
    ) {
        let mut ops: Vec<u32> = Vec::new();
        for param in changed {
            self.slots[program.params[*param].slot.0 as usize] = params[*param];
            ops.extend(&statics.param_ops[*param]);
        }
        // Operations only read earlier slots, so in their order each sees
        // the keys of what it reads already updated.
        ops.sort_unstable();
        ops.dedup();
        for op in ops {
            self.derive_op(programs, program, op as usize);
        }
    }

    /// Derives the keys of the outputs of operation `index` from the keys of
    /// its inputs, which are derived already.
    pub(super) fn derive_op(&mut self, programs: &ProgramSet, program: &Program, index: usize) {
        const ON_STACK: usize = 16;
        let op = &program.ops[index];
        let mut stack = [ContentKey::UNSET; ON_STACK];
        let mut heap = Vec::new();
        let inputs = match op.inputs.len() <= ON_STACK {
            true => &mut stack[..op.inputs.len()],
            false => {
                heap.resize(op.inputs.len(), ContentKey::UNSET);
                &mut heap[..]
            }
        };
        for (input, slot) in inputs.iter_mut().zip(&op.inputs) {
            *input = self.slots[slot.0 as usize];
        }
        let inputs = &*inputs;
        let slots = &mut self.slots;

        match op.kind {
            OpKind::Node(node) => {
                for (output, slot) in op.outputs.iter().enumerate() {
                    slots[slot.0 as usize] = ContentKey::derive_node(node.id, node.version, output as u32, inputs);
                }
            }
            OpKind::Call(callee) => {
                let callee = programs.get(callee);
                let keys = Keys::derive(programs, callee, inputs);
                for (output, slot) in op.outputs.iter().enumerate() {
                    let inner = callee.call_output_slot(output);
                    slots[slot.0 as usize] = keys.slots[inner.0 as usize];
                }
                let keys = Arc::new(keys);
                match self.calls.binary_search_by_key(&(index as u32), |(op, _)| *op) {
                    Ok(at) => self.calls[at].1 = keys,
                    Err(at) => self.calls.insert(at, (index as u32, keys)),
                }
            }
            OpKind::ForEach { callee, element, max } => {
                // What comes out depends on what goes in and on what is done
                // for each element, not on how many there are.
                let callee = programs.get(callee).hash;
                for (output, slot) in op.outputs.iter().enumerate() {
                    slots[slot.0 as usize] =
                        ContentKey::derive_for_each(callee, element as u32, max, output as u32, inputs);
                }
            }
        }
    }

    pub(super) fn call(&self, op: u32) -> &Keys {
        let index = self.calls.partition_point(|(index, _)| *index < op);
        &self.calls[index].1
    }
}
