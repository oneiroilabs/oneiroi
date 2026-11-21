//! What of a program depends on none of its parameters: found once, and
//! pinned in the store once it ran, so that it is never planned again.

use std::sync::Arc;

use crate::{
    engine::{Engine, Executor, keys::Keys},
    program::{Program, ProgramId, ProgramSet, SlotId, SlotSource},
    store::ContentKey,
};

/// What of a program does not depend on its parameters, found once for
/// every instance of it and every value they are set to.
///
/// Values a host does not expose are constants of the program, so most of
/// a graph is usually constant. Its keys are derived here once, an edit of
/// a parameter derives again only what depends on that parameter, and the
/// edge of the constant part, which everything else reads of it, is
/// pinned in the store for as long as the program is there: it is never
/// computed twice, and planning does not even look at it anymore.
pub(super) struct Statics {
    /// Whether each slot depends on a parameter.
    pub(super) dynamic: Vec<bool>,
    /// The key of each slot which does not, unset for the others.
    pub(super) keys: Vec<ContentKey>,
    /// The keys inside the calls which have only constant arguments.
    pub(super) calls: Vec<(u32, Arc<Keys>)>,
    /// The operations which depend on some parameter, in their order.
    pub(super) dynamic_ops: Vec<u32>,
    /// For each parameter, the operations which depend on it, in order.
    pub(super) param_ops: Vec<Vec<u32>>,
    /// The outputs which depend on some parameter: the only ones planned
    /// once the edge is pinned.
    pub(super) dynamic_outputs: Vec<usize>,
    /// The constant slots the rest reads, and the constant outputs with
    /// what comes along with them: all of the constant part that is ever
    /// needed once it ran.
    pub(super) edge: Vec<SlotId>,
    /// Lives as long as the program: what it pins stays for that long.
    alive: Arc<()>,
    /// Whether the edge is in the store and pinned there.
    pub(super) pinned: bool,
}

impl Statics {
    pub(super) fn new(programs: &ProgramSet, program: &Program) -> Self {
        let params = program.params.len();
        let words = params.div_ceil(64).max(1);
        // The parameters each slot depends on, a bit for each.
        let mut depends = vec![0u64; program.slots.len() * words];
        for (index, param) in program.params.iter().enumerate() {
            depends[param.slot.0 as usize * words + index / 64] |= 1 << (index % 64);
        }
        let mut param_ops = vec![Vec::new(); params];
        let mut dynamic_ops = Vec::new();
        let mut mask = vec![0u64; words];
        for (index, op) in program.ops.iter().enumerate() {
            mask.iter_mut().for_each(|word| *word = 0);
            for input in &op.inputs {
                let from = &depends[input.0 as usize * words..][..words];
                mask.iter_mut().zip(from).for_each(|(word, from)| *word |= from);
            }
            for output in &op.outputs {
                depends[output.0 as usize * words..][..words].copy_from_slice(&mask);
            }
            if mask.iter().any(|word| *word != 0) {
                dynamic_ops.push(index as u32);
                for (param, ops) in param_ops.iter_mut().enumerate() {
                    if mask[param / 64] & (1 << (param % 64)) != 0 {
                        ops.push(index as u32);
                    }
                }
            }
        }
        let dynamic: Vec<bool> = depends.chunks(words).map(|slot| slot.iter().any(|word| *word != 0)).collect();

        // The keys of the constant part, which need no parameter.
        let mut keys = Keys {
            slots: program
                .slots
                .iter()
                .map(|slot| match &slot.source {
                    SlotSource::Const { key, .. } => *key,
                    _ => ContentKey::UNSET,
                })
                .collect(),
            calls: Vec::new(),
        };
        let mut dynamic_op = dynamic_ops.iter().peekable();
        for index in 0..program.ops.len() {
            if dynamic_op.next_if(|op| **op == index as u32).is_none() {
                keys.derive_op(programs, program, index);
            }
        }

        // What of the constant part is read by what is not, and the
        // constant outputs with the values they come along with.
        let mut edge = Vec::new();
        for op in &dynamic_ops {
            edge.extend(program.ops[*op as usize].inputs.iter().filter(|slot| !dynamic[slot.0 as usize]));
        }
        for output in &program.outputs {
            let slot = program.slot(output.slot);
            let along = slot.prototype.map(|p| [p.elements, p.groups]).into_iter().flatten();
            let along = [output.slot].into_iter().chain(slot.groups).chain(along).chain(slot.total);
            edge.extend(along.filter(|slot| !dynamic[slot.0 as usize]));
        }
        edge.sort_unstable_by_key(|slot| slot.0);
        edge.dedup();

        let dynamic_outputs = (0..program.outputs.len())
            .filter(|index| dynamic[program.outputs[*index].slot.0 as usize])
            .collect();
        Self {
            dynamic,
            keys: keys.slots,
            calls: keys.calls,
            dynamic_ops,
            param_ops,
            dynamic_outputs,
            edge,
            alive: Arc::new(()),
            pinned: false,
        }
    }
}

impl<E: Executor> Engine<E> {
    /// Pins the edge of the constant part of every program which ran for
    /// the first time, now that it is in the store.
    pub(super) fn pin_edges(&mut self) {
        for id in std::mem::take(&mut self.to_pin) {
            let Some(Some(statics)) = self.statics.get_mut(id.0 as usize) else {
                continue;
            };
            let keys: Vec<ContentKey> = statics.edge.iter().map(|slot| statics.keys[slot.0 as usize]).collect();
            // Only once all of it is there: what was not planned yet is
            // pinned the next time.
            if keys.iter().all(|key| self.store.contains(*key)) {
                self.store.pin(keys, &statics.alive);
                statics.pinned = true;
            }
        }
    }

    /// Lets go of the constant part of programs which were removed, so what
    /// they pinned can be evicted.
    pub(super) fn forget_removed(&mut self) {
        for (index, statics) in self.statics.iter_mut().enumerate() {
            if statics.is_some() && !self.programs.contains(ProgramId(index as u32)) {
                *statics = None;
            }
        }
    }
}
