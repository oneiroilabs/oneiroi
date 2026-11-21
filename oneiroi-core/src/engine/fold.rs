//! Folding the constant part of a program into constants, once, for a host
//! which never edits it.

use crate::{
    engine::{Engine, Executor, constant::Statics, keys::Keys, planner::{Planner, Scope}},
    program::{ProgramId, SlotId, SlotSource},
    store::ContentKey,
};

impl<E: Executor> Engine<E> {
    /// The program `id` with its constant part folded: run once, here, and
    /// what the rest reads of it put into the new program as constants.
    /// Only what depends on a parameter is left to run.
    ///
    /// For a host which loads a graph and never edits it, as a step of its
    /// own after loading. The folded program has the same parameters and
    /// outputs, and gives the same results under the same keys: its
    /// constants keep the keys they were derived under, so whatever the
    /// program shares with others is still shared.
    pub fn fold(&mut self, id: ProgramId) -> ProgramId {
        let index = id.0 as usize;
        if self.statics.len() <= index {
            self.statics.resize_with(index + 1, || None);
        }
        let program = self.programs.get(id);
        let statics = self.statics[index].get_or_insert_with(|| Statics::new(&self.programs, program));
        let mut keep = vec![false; program.ops.len()];
        for op in &statics.dynamic_ops {
            keep[*op as usize] = true;
        }
        // What of the constant part is still needed, and is not a constant
        // already: those values are run for.
        let needed: Vec<SlotId> = program
            .needed(&keep)
            .iter()
            .enumerate()
            .filter(|(index, needed)| {
                **needed
                    && !statics.dynamic[*index]
                    && matches!(program.slots[*index].source, SlotSource::Op { .. })
            })
            .map(|(index, _)| SlotId(index as u32))
            .collect();

        let keys = Keys {
            slots: statics.keys.clone(),
            calls: statics.calls.clone(),
        };
        let scope = Scope {
            program,
            keys: &keys,
            caller: None,
            constant: None,
            links: &[],
        };
        let mut planner = Planner {
            programs: &self.programs,
            store: &mut self.store,
            defaults: true,
            work: &mut self.work,
            scheduled: &mut self.scheduled,
        };
        for slot in &needed {
            planner.ensure(&scope, *slot);
        }
        let values: Vec<(SlotId, ContentKey)> =
            needed.iter().map(|slot| (*slot, keys.slots[slot.0 as usize])).collect();
        self.flush();
        let wanted: Vec<ContentKey> = values.iter().map(|(_, key)| *key).collect();
        self.executor.download(&mut self.store, &wanted);
        let values = values
            .into_iter()
            .map(|(slot, key)| {
                let handle = self.store.get(key).expect("the constant part ran");
                (slot, (key, self.store.words(handle).into()))
            })
            .collect();

        let folded = self.programs.get(id).folded(&keep, &values);
        self.add_program(folded)
    }
}
