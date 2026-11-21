//! What runs the work which was planned: the interface of a backend, and
//! the backend which runs everything on the CPU.

use std::time::{Duration, Instant};

use crate::{node::{Inputs, NodeDef, Outputs}, store::{ContentKey, Handle, Store}};

/// One node to run. Every input is either in the store already or an
/// output of work that comes earlier in the same batch.
#[derive(Debug)]
pub struct Work {
    pub node: &'static NodeDef,
    pub inputs: Box<[ContentKey]>,
    pub outputs: Box<[ContentKey]>,
}

/// A backend. It has to leave every output of every piece of work in the
/// store, under the key the work names.
pub trait Executor {
    fn execute(&mut self, store: &mut Store, work: &[Work]);

    /// Brings the values of `keys` which only the backend has so far into
    /// the store, so they can be read. A backend which keeps nothing to
    /// itself has nothing to do.
    fn download(&mut self, _store: &mut Store, _keys: &[ContentKey]) {}

    /// Stores the values of `parts`, one after the other, as the value of
    /// `output`. Every part is in the store already.
    ///
    /// This does it on the CPU. A backend which keeps values to itself can
    /// do it without bringing them over.
    fn gather(&mut self, store: &mut Store, parts: &[ContentKey], output: ContentKey) {
        self.download(store, parts);
        let parts: Vec<Handle> = parts.iter().filter_map(|part| store.get(*part)).collect();
        let len = parts.iter().map(|part| part.len).sum();
        // Each part is copied once, right where the whole is kept.
        let whole = store.reserve_output(output, len);
        // SAFETY: the room of the whole is new, so no part is in it, and
        // nothing is evicted or written while the parts are copied.
        let whole = unsafe { store.output_words(whole) };
        let mut at = 0;
        for part in parts {
            let part = unsafe { store.words_unbound(part) };
            whole[at..at + part.len()].copy_from_slice(part);
            at += part.len();
        }
        store.commit(output);
    }
}

/// Time an executor spent on named parts of its work, added up until
/// somebody takes it. Only parts which do not contain each other are
/// timed, so they can be added up.
#[derive(Debug, Default, Clone)]
pub struct Phases(Vec<(&'static str, Duration)>);

impl Phases {
    pub fn add(&mut self, name: &'static str, time: Duration) {
        match self.0.iter_mut().find(|(phase, _)| *phase == name) {
            Some((_, total)) => *total += time,
            None => self.0.push((name, time)),
        }
    }

    /// Adds the time since `start` to `name`.
    pub fn since(&mut self, name: &'static str, start: Instant) {
        self.add(name, start.elapsed());
    }

    /// What `name` added up to so far.
    pub fn get(&self, name: &str) -> Duration {
        self.0
            .iter()
            .find(|(phase, _)| *phase == name)
            .map_or(Duration::ZERO, |(_, time)| *time)
    }

    /// Everything added up so far, and starts again from nothing.
    pub fn take(&mut self) -> Vec<(&'static str, Duration)> {
        std::mem::take(&mut self.0)
    }
}

/// Runs every node through its CPU kernel, one after the other.
#[derive(Default)]
pub struct CpuExecutor {
    handles: Vec<Handle>,
    phases: Phases,
}

impl CpuExecutor {
    /// How long the kernels took, and writing what they made into the
    /// store, since the last call.
    pub fn take_phases(&mut self) -> Vec<(&'static str, Duration)> {
        self.phases.take()
    }
}

impl Executor for CpuExecutor {
    fn execute(&mut self, store: &mut Store, work: &[Work]) {
        for item in work {
            self.handles.clear();
            self.handles.extend(item.inputs.iter().map(|key| {
                store
                    .get(*key)
                    .expect("planned inputs are stored before they are read")
            }));

            // SAFETY: the inputs were stored before the work was planned
            // and nothing is evicted while it runs. The outputs are kept
            // under other keys, in room which is reserved anew, so writing
            // them never touches what the inputs read.
            let inputs = unsafe { Inputs::unbound(store, item.node.inputs, &self.handles) };
            let mut outputs = Outputs::new(store, item.node.outputs, &item.outputs);
            let start = Instant::now();
            (item.node.cpu)(&inputs, &mut outputs);
            self.phases.since("kernels", start);

            let start = Instant::now();
            outputs.finish();
            self.phases.since("store", start);
        }
    }
}
