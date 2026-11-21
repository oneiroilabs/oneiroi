//! Calls of a program for each element, which can only be planned once it is
//! known how many elements there are.

use crate::{
    engine::{Engine, Executor, keys::Keys, planner::{Planner, Scope}},
    program::ProgramId,
    store::{ContentKey, KeySet},
    types::DataType,
};

/// A call for each element, to be expanded once the element count is known.
#[derive(Debug)]
pub(super) struct Expansion {
    pub(super) callee: ProgramId,
    pub(super) element: u32,
    pub(super) max: u32,
    pub(super) inputs: Box<[ContentKey]>,
    pub(super) outputs: Box<[ContentKey]>,
}

/// A call for each element which had more elements than it may run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overflow {
    /// The name of the program which was called.
    pub program: String,
    pub elements: u32,
    pub max: u32,
}

impl<E: Executor> Engine<E> {
    /// The calls for each element which had to leave elements out during
    /// the last [`Engine::flush`].
    pub fn overflows(&self) -> &[Overflow] {
        &self.overflows
    }

    /// Plans and runs the calls of one call for each element, and puts
    /// their outputs together.
    pub(super) fn expand(&mut self, expansion: Expansion) -> usize {
        let Expansion {
            callee,
            element,
            max,
            inputs,
            outputs,
        } = expansion;
        let program = self.programs.get(callee);
        let ty = program.slot(program.params[element as usize].slot).ty;
        let source = inputs[element as usize];
        let len = self
            .store
            .touch(source)
            .map_or(0, |handle| handle.len / ty.words);

        // A run for every group of what the elements are taken of, or for
        // every element if it has no groups.
        let groups = inputs[inputs.len() - 1];
        self.executor.download(&mut self.store, &[groups]);
        let runs: Vec<(u32, u32)> = match self.store.read::<u32>(groups) {
            Some(offsets) if !offsets.is_empty() => offsets
                .windows(2)
                .map(|pair| (pair[0].min(len), pair[1].min(len).max(pair[0].min(len))))
                .collect(),
            _ => (0..len).map(|index| (index, index + 1)).collect(),
        };
        if runs.len() > max as usize {
            self.overflows.push(Overflow {
                program: program.name.clone(),
                elements: runs.len() as u32,
                max,
            });
        }

        // A single group of everything, for the groups of the element.
        let no_groups = ContentKey::of_data(&u32::INFO, &[]);
        self.store.insert(no_groups, &[]);
        let hidden = program.groups_of_param(element as usize);

        let mut steps = Vec::new();
        let mut scheduled = KeySet::default();
        let mut parts = vec![Vec::new(); program.outputs.len()];
        let mut params = inputs[..inputs.len() - 1].to_vec();
        let resident = self.store.is_resident(source);
        let mut keys = Keys::default();
        for (index, (start, end)) in runs.iter().take(max as usize).enumerate() {
            // Where the values are at hand an element is known by what it
            // is: elements which did not change, or which are the same as
            // another one, find their results. Otherwise it is known by
            // where it is, which needs nothing from the backend.
            let (from, words) = (start * ty.words, (end - start) * ty.words);
            let key = match resident {
                true => {
                    let all = self.store.words(self.store.get(source).unwrap());
                    ContentKey::of_data(&ty, &all[from as usize..][..words as usize])
                }
                false => ContentKey::derive("element", index as u32, &[source, groups]),
            };
            // An element is a part of what it is an element of, so it needs
            // no copy on any backend.
            self.store.alias(key, source, from, words);
            params[element as usize] = key;
            if let Some(hidden) = hidden {
                params[hidden] = no_groups;
            }

            keys.rederive(&self.programs, program, &params);
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
                defaults: false,
                work: &mut steps,
                scheduled: &mut scheduled,
            };
            for (parts, output) in parts.iter_mut().zip(&program.outputs) {
                planner.ensure(&scope, output.slot);
                parts.push(keys.slots[output.slot.0 as usize]);
            }
        }

        let words: Vec<u32> = program
            .outputs
            .iter()
            .map(|output| program.slot(output.slot).ty.words)
            .collect();
        let ran = self.run(steps);
        let count = words.len();
        for (index, parts) in parts.iter().enumerate() {
            self.executor.gather(&mut self.store, parts, outputs[index]);

            // One group for what every run gave.
            let words = words[index];
            let mut offsets = vec![0u32];
            for part in parts {
                let len = self.store.get(*part).map_or(0, |handle| handle.len / words);
                offsets.push(offsets.last().unwrap() + len);
            }
            self.store.insert(outputs[count + index], &offsets);
        }
        ran
    }
}
