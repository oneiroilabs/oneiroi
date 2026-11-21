//! Planning and execution.
//!
//! Evaluating an instance always starts with a planning step on the CPU:
//! every value the instance needs gets its [`ContentKey`], and whatever the
//! [`Store`] does not hold yet becomes [`Work`]. Only that work is handed to
//! the [`Executor`], which is the part that can run on the CPU or the GPU.

mod constant;
mod executor;
mod fold;
mod for_each;
mod instance;
mod keys;
mod link;
mod plan;
mod planner;
#[cfg(test)]
mod tests;

pub use executor::{CpuExecutor, Executor, Phases, Work};
pub use for_each::Overflow;
pub use instance::{Instance, ParamError};
pub use link::LinkError;

use crate::{
    engine::{constant::Statics, for_each::Expansion},
    program::{Program, ProgramId, ProgramSet},
    store::{KeySet, Store},
};

/// Something which was planned and has to run.
#[derive(Debug)]
enum Step {
    Work(Work),
    ForEach(Expansion),
}

/// Owns the programs, the store and the backend that fills it.
pub struct Engine<E: Executor = CpuExecutor> {
    programs: ProgramSet,
    store: Store,
    executor: E,
    work: Vec<Step>,
    scheduled: KeySet,
    overflows: Vec<Overflow>,
    /// The constant part of every program which was planned, by its id.
    statics: Vec<Option<Statics>>,
    /// The programs whose constant edge is pinned once what was planned ran.
    to_pin: Vec<ProgramId>,
    /// How many times anything was planned: whether an instance was given
    /// to a plan, or planned in it, is known by this.
    plans: u64,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new(CpuExecutor::default())
    }
}

impl<E: Executor> Engine<E> {
    pub fn new(executor: E) -> Self {
        Self::with_store(executor, Store::new())
    }

    pub fn with_store(executor: E, store: Store) -> Self {
        Self {
            programs: ProgramSet::new(),
            store,
            executor,
            work: Vec::new(),
            scheduled: KeySet::default(),
            overflows: Vec::new(),
            statics: Vec::new(),
            to_pin: Vec::new(),
            plans: 0,
        }
    }

    pub fn programs(&self) -> &ProgramSet {
        &self.programs
    }

    pub fn programs_mut(&mut self) -> &mut ProgramSet {
        &mut self.programs
    }

    pub fn add_program(&mut self, program: Program) -> ProgramId {
        self.programs.add(program)
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn executor(&self) -> &E {
        &self.executor
    }

    pub fn executor_mut(&mut self) -> &mut E {
        &mut self.executor
    }

    /// The store together with the backend that fills it, for whoever
    /// has to use both at once.
    pub fn parts_mut(&mut self) -> (&mut Store, &mut E) {
        (&mut self.store, &mut self.executor)
    }
}
