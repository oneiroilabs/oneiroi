//! What has to run for a value to be in the store: only the part of a
//! program behind what is missing is ever visited.

use crate::{
    engine::{Step, Work, for_each::Expansion, keys::Keys, link::{Link, followed}},
    program::{OpKind, Program, ProgramSet, SlotId, SlotSource},
    store::{KeySet, Store},
};

/// A program seen from one place it runs at: directly as an instance, or
/// through a call from another program.
pub(super) struct Scope<'a> {
    pub(super) program: &'a Program,
    pub(super) keys: &'a Keys,
    /// The calling scope and the slots it passes as arguments.
    pub(super) caller: Option<(&'a Scope<'a>, &'a [SlotId])>,
    /// For an instance: whether each slot depends on a parameter, once its
    /// constant part is pinned. The constant slots are there then, and are
    /// not even looked up.
    pub(super) constant: Option<&'a [bool]>,
    /// For an instance: the parameters which follow an output of another.
    pub(super) links: &'a [Link],
}

pub(super) struct Planner<'a> {
    pub(super) programs: &'a ProgramSet,
    pub(super) store: &'a mut Store,
    /// Whether a missing parameter is at its default. Otherwise every
    /// parameter is in the store already.
    pub(super) defaults: bool,
    pub(super) work: &'a mut Vec<Step>,
    pub(super) scheduled: &'a mut KeySet,
}

impl Planner<'_> {
    /// Makes sure `slot` is in the store once the planned work has run.
    /// Nothing is planned for values which are stored already, so only the
    /// part of the graph behind a missing value is ever visited.
    pub(super) fn ensure(&mut self, scope: &Scope<'_>, slot: SlotId) {
        if scope.constant.is_some_and(|dynamic| !dynamic[slot.0 as usize]) {
            return;
        }
        let key = scope.keys.slots[slot.0 as usize];
        if self.scheduled.contains(&key) || self.store.touch(key).is_some() {
            return;
        }

        match &scope.program.slot(slot).source {
            SlotSource::Param(index) => match scope.caller {
                Some((caller, args)) => self.ensure(caller, args[*index as usize]),
                None => match followed(scope.links, *index as usize, key) {
                    // What it follows is planned as it was when it was last
                    // planned itself.
                    Some((program, keys, links, slot)) => {
                        let scope = Scope {
                            program: self.programs.get(program),
                            keys: &keys,
                            caller: None,
                            constant: None,
                            links: &links,
                        };
                        self.ensure(&scope, slot);
                    }
                    // A value which was set is held in the store, so only a
                    // default can be missing.
                    None => {
                        assert!(self.defaults, "parameters are stored before they are planned");
                        self.store.insert(key, &scope.program.params[*index as usize].default);
                    }
                },
            },
            SlotSource::Const { words, .. } => {
                self.store.insert(key, words);
            }
            SlotSource::Op { op, output } => {
                let op_index = *op;
                let op = &scope.program.ops[op_index as usize];
                match op.kind {
                    OpKind::Node(node) => {
                        for input in &op.inputs {
                            self.ensure(scope, *input);
                        }
                        let keys = |slots: &[SlotId]| {
                            slots
                                .iter()
                                .map(|slot| scope.keys.slots[slot.0 as usize])
                                .collect::<Box<[_]>>()
                        };
                        let outputs = keys(&op.outputs);
                        self.scheduled.extend(outputs.iter().copied());
                        self.work.push(Step::Work(Work {
                            node,
                            inputs: keys(&op.inputs),
                            outputs,
                        }));
                    }
                    OpKind::ForEach {
                        callee,
                        element,
                        max,
                    } => {
                        for input in &op.inputs {
                            self.ensure(scope, *input);
                        }
                        let keys = |slots: &[SlotId]| {
                            slots
                                .iter()
                                .map(|slot| scope.keys.slots[slot.0 as usize])
                                .collect::<Box<[_]>>()
                        };
                        let outputs = keys(&op.outputs);
                        self.scheduled.extend(outputs.iter().copied());
                        self.work.push(Step::ForEach(Expansion {
                            callee,
                            element,
                            max,
                            inputs: keys(&op.inputs),
                            outputs,
                        }));
                    }
                    OpKind::Call(callee) => {
                        let program = self.programs.get(callee);
                        let callee = Scope {
                            program,
                            keys: scope.keys.call(op_index),
                            caller: Some((scope, &op.inputs)),
                            constant: None,
                            links: &[],
                        };
                        self.ensure(&callee, program.call_output_slot(*output as usize));
                    }
                }
            }
        }
    }
}
