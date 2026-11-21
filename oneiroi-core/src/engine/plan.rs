//! Planning instances, and running what was planned: a frame.

use std::sync::{Arc, atomic::Ordering};

use crate::{
    engine::{
        Engine,
        Executor,
        Instance,
        Step,
        constant::Statics,
        keys::Keys,
        planner::{Planner, Scope},
    },
    key::ContentKey,
    program::ProgramId,
};

impl<E: Executor> Engine<E> {
    /// Brings every output of every instance into the store.
    ///
    /// All instances are planned first and executed as a single batch, so
    /// a backend sees everything that is new this frame at once. Returns
    /// how many nodes had to run.
    pub fn evaluate<'a>(&mut self, instances: impl IntoIterator<Item = &'a mut Instance>) -> usize {
        self.plan(instances);
        self.flush()
    }

    /// The first half of [`Engine::evaluate`]: finds out what has to run
    /// for the outputs of the instances, without running it. Returns how
    /// many nodes that is.
    ///
    /// Planning never needs the backend. The keys of all outputs are known
    /// afterwards, and [`Engine::flush`] can happen later and somewhere
    /// else, for example where a graphics device is at hand.
    pub fn plan<'a>(&mut self, instances: impl IntoIterator<Item = &'a mut Instance>) -> usize {
        let before = self.work.len();
        self.forget_removed();
        self.plans += 1;
        let now = self.plans;
        // What follows others comes after everything else, so that it sees
        // what it follows as it is now.
        let mut follow = Vec::new();
        for instance in instances {
            if let Some(published) = instance.published.get() {
                published.listed.store(now, Ordering::Relaxed);
            }
            match instance.links.is_empty() {
                true => self.plan_one(instance, now),
                false => follow.push(instance),
            }
        }
        // And each after what it follows, if that is planned here too.
        while !follow.is_empty() {
            let waiting = follow.len();
            let mut index = 0;
            while index < follow.len() {
                let ready = follow[index].links.iter().all(|link| {
                    link.source.listed.load(Ordering::Relaxed) != now
                        || link.source.planned.load(Ordering::Relaxed) == now
                });
                if ready {
                    let instance = follow.swap_remove(index);
                    self.plan_one(instance, now);
                } else {
                    index += 1;
                }
            }
            assert!(follow.len() < waiting, "links never go round in a circle");
        }
        self.work.len() - before
    }

    fn plan_one(&mut self, instance: &mut Instance, now: u64) {
        self.plan_instance(instance);
        if let Some(published) = instance.published.get() {
            published.planned.store(now, Ordering::Relaxed);
        }
    }

    fn plan_instance(&mut self, instance: &mut Instance) {
        self.follow(instance);
        // Nothing changed, and its outputs are held: there is nothing to
        // plan, and nothing to keep from being evicted.
        if !instance.held.is_empty() && instance.changed.is_empty() {
            return;
        }
        for key in instance.held.drain(..) {
            self.store.release(key, &instance.alive);
        }
        self.derive_keys(instance);
        let planned = self.work.len();
        let wanted = self.ensure_outputs(instance);
        self.settle(instance, planned, wanted);
        self.publish(instance);
    }

    /// The constant part of program `id`, found the first time it is asked
    /// for.
    fn statics(&mut self, id: ProgramId) -> &Statics {
        let index = id.0 as usize;
        if self.statics.len() <= index {
            self.statics.resize_with(index + 1, || None);
        }
        let program = self.programs.get(id);
        self.statics[index].get_or_insert_with(|| Statics::new(&self.programs, program))
    }

    /// The keys of `instance` for the parameters it has now: all of them the
    /// first time, then only what depends on a parameter which changed.
    fn derive_keys(&mut self, instance: &mut Instance) {
        let id = instance.program;
        self.statics(id);
        let statics = self.statics[id.0 as usize].as_ref().unwrap();
        let program = self.programs.get(id);
        match &mut instance.keys {
            None => {
                let keys = Keys::of_instance(statics, &self.programs, program, &instance.param_keys);
                instance.keys = Some(Arc::new(keys));
            }
            // Only copied while somebody who follows it still has them.
            Some(keys) if !instance.changed.is_empty() => Arc::make_mut(keys).update(
                statics,
                &self.programs,
                program,
                &instance.param_keys,
                &instance.changed,
            ),
            Some(_) => {}
        }
        instance.changed.clear();
        if !statics.pinned && !self.to_pin.contains(&id) {
            self.to_pin.push(id);
        }
    }

    /// Plans whatever the outputs of `instance` need which is not there,
    /// and returns their keys, with those of what comes along with them.
    fn ensure_outputs(&mut self, instance: &Instance) -> Vec<ContentKey> {
        let program = self.programs.get(instance.program);
        let statics = self.statics[instance.program.0 as usize].as_ref().unwrap();
        let keys = instance.keys.as_ref().unwrap();
        // Once its edge is pinned, the constant part is there for sure.
        let constant = statics.pinned.then_some(statics.dynamic.as_slice());
        let scope = Scope {
            program,
            keys,
            caller: None,
            constant,
            links: &instance.links,
        };
        let mut planner = Planner {
            programs: &self.programs,
            store: &mut self.store,
            defaults: true,
            work: &mut self.work,
            scheduled: &mut self.scheduled,
        };
        // Once the edge is pinned, the constant outputs are there.
        let all: Vec<usize>;
        let outputs = match constant {
            Some(_) => &statics.dynamic_outputs,
            None => {
                all = (0..program.outputs.len()).collect();
                &all
            }
        };
        let mut wanted = Vec::new();
        for output in outputs.iter().map(|index| &program.outputs[*index]) {
            // What comes along with it is needed by whoever reads it.
            let slot = program.slot(output.slot);
            let along = slot.prototype.map(|p| [p.elements, p.groups]);
            let along = along.into_iter().flatten().chain(slot.total);
            for slot in [output.slot].into_iter().chain(slot.groups).chain(along) {
                planner.ensure(&scope, slot);
                wanted.push(keys.slots[slot.0 as usize]);
            }
        }
        wanted
    }

    /// Holds the outputs of `instance` once all of them are there and its
    /// constant part is pinned: until a parameter changes, planning it is
    /// skipped. `planned` is how much work there was before its outputs.
    fn settle(&mut self, instance: &mut Instance, planned: usize, wanted: Vec<ContentKey>) {
        let pinned = self.statics[instance.program.0 as usize].as_ref().unwrap().pinned;
        let settled =
            pinned && self.work.len() == planned && wanted.iter().all(|key| self.store.contains(*key));
        if settled {
            for key in &wanted {
                self.store.hold(*key, &instance.alive);
            }
            // An instance without outputs which change has nothing to
            // hold, and is planned again every time, which costs nothing.
            instance.held = wanted;
        }
    }

    /// The second half of [`Engine::evaluate`]: runs everything which was
    /// planned since the last time. Returns how many nodes ran.
    ///
    /// A call for each element can only be planned once it is known how
    /// many elements there are, so that part of the planning happens here.
    pub fn flush(&mut self) -> usize {
        let steps = std::mem::take(&mut self.work);
        self.scheduled.clear();
        self.overflows.clear();
        let ran = self.run(steps);
        self.pin_edges();
        ran
    }

    pub(super) fn run(&mut self, steps: Vec<Step>) -> usize {
        let mut ran = 0;
        let mut batch = Vec::new();
        for step in steps {
            match step {
                Step::Work(work) => batch.push(work),
                Step::ForEach(expansion) => {
                    // Whatever the elements come from has to be there.
                    ran += batch.len();
                    self.executor.execute(&mut self.store, &batch);
                    batch.clear();
                    ran += self.expand(expansion);
                }
            }
        }
        ran += batch.len();
        self.executor.execute(&mut self.store, &batch);
        ran
    }

    /// Forgets what was planned and did not run yet, for when it is not
    /// needed anymore. Nothing is lost: it is planned again when it is.
    pub fn discard(&mut self) {
        self.work.clear();
        self.scheduled.clear();
    }

    /// How many nodes are planned and did not run yet.
    pub fn pending(&self) -> usize {
        self.work.len()
    }

    /// Ends the frame: results nobody asked for lately are dropped until
    /// the store fits into `budget_words`.
    pub fn end_frame(&mut self, budget_words: u64) {
        self.forget_removed();
        self.store.evict(budget_words);
        self.store.end_frame();
    }
}
