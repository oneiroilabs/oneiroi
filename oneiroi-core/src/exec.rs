//! Planning and execution.
//!
//! Evaluating an instance always starts with a planning step on the CPU:
//! every value the instance needs gets its [`ContentKey`], and whatever the
//! [`Store`] does not hold yet becomes [`Work`]. Only that work is handed to
//! the [`Executor`], which is the part that can run on the CPU or the GPU.

use std::{collections::HashSet, fmt};

use crate::{
    node::{Inputs, NodeDef, Outputs},
    program::{OpKind, Program, ProgramId, ProgramSet, SlotId, SlotSource},
    store::{ContentKey, Handle, Store},
    types::DataType,
};

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
        let mut words = Vec::new();
        for part in parts {
            if let Some(handle) = store.get(*part) {
                words.extend_from_slice(store.words(handle));
            }
        }
        store.insert(output, &words);
    }
}

/// A call for each element, to be expanded once the element count is known.
#[derive(Debug)]
struct Expansion {
    callee: ProgramId,
    element: u32,
    max: u32,
    inputs: Box<[ContentKey]>,
    outputs: Box<[ContentKey]>,
}

/// Something which was planned and has to run.
#[derive(Debug)]
enum Step {
    Work(Work),
    ForEach(Expansion),
}

/// A call for each element which had more elements than it may run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overflow {
    /// The name of the program which was called.
    pub program: String,
    pub elements: u32,
    pub max: u32,
}

/// Runs every node through its CPU kernel, one after the other.
#[derive(Default)]
pub struct CpuExecutor {
    handles: Vec<Handle>,
    buffers: Vec<Vec<u32>>,
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

            let mut buffers = std::mem::take(&mut self.buffers);
            buffers.iter_mut().for_each(Vec::clear);
            // The declared outputs and the offsets of the grouped ones.
            buffers.resize_with(item.outputs.len().max(buffers.len()), Vec::new);
            let mut outputs = Outputs {
                ports: item.node.outputs,
                buffers,
            };

            let inputs = Inputs {
                store,
                ports: item.node.inputs,
                handles: &self.handles,
            };
            (item.node.cpu)(&inputs, &mut outputs);

            for (key, words) in item.outputs.iter().zip(&outputs.buffers) {
                store.insert(*key, words);
            }
            self.buffers = outputs.buffers;
        }
    }
}

/// The keys of every slot of one program for one set of parameter values.
struct Keys {
    slots: Vec<ContentKey>,
    /// The keys inside each program this one calls, by operation index.
    calls: Vec<(u32, Keys)>,
}

impl Keys {
    fn derive(programs: &ProgramSet, program: &Program, params: &[ContentKey]) -> Self {
        let mut slots: Vec<_> = program
            .slots
            .iter()
            .map(|slot| match &slot.source {
                SlotSource::Param(index) => params[*index as usize],
                SlotSource::Const { key, .. } => *key,
                // Filled in below, operations only read earlier slots.
                SlotSource::Op { .. } => ContentKey::UNSET,
            })
            .collect();
        let mut calls = Vec::new();

        let mut inputs = Vec::new();
        for (index, op) in program.ops.iter().enumerate() {
            inputs.clear();
            inputs.extend(op.inputs.iter().map(|slot| slots[slot.0 as usize]));

            match op.kind {
                OpKind::Node(node) => {
                    for (output, slot) in op.outputs.iter().enumerate() {
                        slots[slot.0 as usize] =
                            ContentKey::derive_node(node.id, node.version, output as u32, &inputs);
                    }
                }
                OpKind::Call(callee) => {
                    let callee = programs.get(callee);
                    let keys = Keys::derive(programs, callee, &inputs);
                    for (index, slot) in op.outputs.iter().enumerate() {
                        let inner = callee.call_output_slot(index);
                        slots[slot.0 as usize] = keys.slots[inner.0 as usize];
                    }
                    calls.push((index as u32, keys));
                }
                OpKind::ForEach {
                    callee,
                    element,
                    max,
                } => {
                    // What comes out depends on what goes in and on what
                    // is done for each element, not on how many there are.
                    let callee = programs.get(callee).hash;
                    let id = format!("for_each/{callee:x}/{element}/{max}");
                    for (output, slot) in op.outputs.iter().enumerate() {
                        slots[slot.0 as usize] = ContentKey::derive(&id, output as u32, &inputs);
                    }
                }
            }
        }

        Self { slots, calls }
    }

    fn call(&self, op: u32) -> &Keys {
        let index = self.calls.partition_point(|(index, _)| *index < op);
        &self.calls[index].1
    }
}

/// A program seen from one place it runs at: directly as an instance, or
/// through a call from another program.
struct Scope<'a> {
    program: &'a Program,
    keys: &'a Keys,
    /// The calling scope and the slots it passes as arguments.
    caller: Option<(&'a Scope<'a>, &'a [SlotId])>,
}

struct Planner<'a> {
    programs: &'a ProgramSet,
    store: &'a mut Store,
    /// The parameter values of the instance being planned.
    params: &'a [Box<[u32]>],
    work: &'a mut Vec<Step>,
    scheduled: &'a mut HashSet<ContentKey>,
}

impl Planner<'_> {
    /// Makes sure `slot` is in the store once the planned work has run.
    /// Nothing is planned for values which are stored already, so only the
    /// part of the graph behind a missing value is ever visited.
    fn ensure(&mut self, scope: &Scope<'_>, slot: SlotId) {
        let key = scope.keys.slots[slot.0 as usize];
        if self.scheduled.contains(&key) || self.store.touch(key).is_some() {
            return;
        }

        match &scope.program.slot(slot).source {
            SlotSource::Param(index) => match scope.caller {
                Some((caller, args)) => self.ensure(caller, args[*index as usize]),
                None => {
                    self.store.insert(key, &self.params[*index as usize]);
                }
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
                        };
                        self.ensure(&callee, program.call_output_slot(*output as usize));
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamError {
    Unknown(String),
    Type {
        param: String,
        expected: &'static str,
        found: &'static str,
    },
    /// The words are not whole elements of the type of the parameter.
    Words {
        param: String,
        expected: &'static str,
    },
}

impl fmt::Display for ParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(param) => write!(f, "there is no parameter `{param}`"),
            Self::Type {
                param,
                expected,
                found,
            } => write!(f, "`{param}` expects `{expected}` but got `{found}`"),
            Self::Words { param, expected } => {
                write!(f, "`{param}` expects whole elements of `{expected}`")
            }
        }
    }
}

impl std::error::Error for ParamError {}

/// One use of a program with its own parameter values.
///
/// It owns nothing but those values. All results live in the [`Store`] and
/// are shared with every other instance that asks for the same thing.
pub struct Instance {
    program: ProgramId,
    params: Vec<Box<[u32]>>,
    param_keys: Vec<ContentKey>,
    /// Outdated once a parameter changes.
    keys: Option<Keys>,
}

impl Instance {
    pub fn program(&self) -> ProgramId {
        self.program
    }
}

/// Owns the programs, the store and the backend that fills it.
pub struct Engine<E: Executor = CpuExecutor> {
    programs: ProgramSet,
    store: Store,
    executor: E,
    work: Vec<Step>,
    scheduled: HashSet<ContentKey>,
    overflows: Vec<Overflow>,
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
            scheduled: HashSet::new(),
            overflows: Vec::new(),
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

    /// A new instance with every parameter at its default.
    pub fn instantiate(&self, program: ProgramId) -> Instance {
        let decl = self.programs.get(program);
        let params: Vec<_> = decl.params.iter().map(|p| p.default.clone()).collect();
        let param_keys = decl
            .params
            .iter()
            .map(|param| ContentKey::of_data(&decl.slot(param.slot).ty, &param.default))
            .collect();
        Instance {
            program,
            params,
            param_keys,
            keys: None,
        }
    }

    pub fn set_param<T: DataType>(
        &self,
        instance: &mut Instance,
        name: &str,
        value: &[T],
    ) -> Result<(), ParamError> {
        let program = self.programs.get(instance.program);
        let index = program
            .param_index(name)
            .ok_or_else(|| ParamError::Unknown(name.into()))?;
        let ty = program.slot(program.params[index].slot).ty;
        if ty != T::INFO {
            return Err(ParamError::Type {
                param: name.into(),
                expected: ty.name,
                found: T::NAME,
            });
        }
        self.set_param_words(instance, name, bytemuck::cast_slice(value))
    }

    /// [`Engine::set_param`] for a value which is only known by its words.
    pub fn set_param_words(
        &self,
        instance: &mut Instance,
        name: &str,
        words: &[u32],
    ) -> Result<(), ParamError> {
        let program = self.programs.get(instance.program);
        let index = program
            .param_index(name)
            .ok_or_else(|| ParamError::Unknown(name.into()))?;
        let ty = program.slot(program.params[index].slot).ty;
        if words.len() % ty.words as usize != 0 {
            return Err(ParamError::Words {
                param: name.into(),
                expected: ty.name,
            });
        }

        let key = ContentKey::of_data(&ty, words);
        if key != instance.param_keys[index] {
            instance.params[index] = words.into();
            instance.param_keys[index] = key;
            instance.keys = None;
        }
        Ok(())
    }

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
        for instance in instances {
            let program = self.programs.get(instance.program);
            let keys = instance
                .keys
                .get_or_insert_with(|| Keys::derive(&self.programs, program, &instance.param_keys));
            let scope = Scope {
                program,
                keys,
                caller: None,
            };
            let mut planner = Planner {
                programs: &self.programs,
                store: &mut self.store,
                params: &instance.params,
                work: &mut self.work,
                scheduled: &mut self.scheduled,
            };
            for output in &program.outputs {
                planner.ensure(&scope, output.slot);
            }
        }
        self.work.len() - before
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
        self.run(steps)
    }

    /// The calls for each element which had to leave elements out during
    /// the last [`Engine::flush`].
    pub fn overflows(&self) -> &[Overflow] {
        &self.overflows
    }

    fn run(&mut self, steps: Vec<Step>) -> usize {
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

    /// Plans and runs the calls of one call for each element, and puts
    /// their outputs together.
    fn expand(&mut self, expansion: Expansion) -> usize {
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
        let mut scheduled = HashSet::new();
        let mut parts = vec![Vec::new(); program.outputs.len()];
        let mut params = inputs[..inputs.len() - 1].to_vec();
        let resident = self.store.is_resident(source);
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

            let keys = Keys::derive(&self.programs, program, &params);
            let scope = Scope {
                program,
                keys: &keys,
                caller: None,
            };
            let mut planner = Planner {
                programs: &self.programs,
                store: &mut self.store,
                // Every parameter is in the store already.
                params: &[],
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

    /// The key of an output, once the instance was evaluated.
    pub fn output_key(&self, instance: &Instance, name: &str) -> Option<ContentKey> {
        let program = self.programs.get(instance.program);
        let slot = program.outputs[program.output_index(name)?].slot;
        Some(instance.keys.as_ref()?.slots[slot.0 as usize])
    }

    /// An output of an evaluated instance. `None` if there is no such
    /// output, it is not in the store (anymore), or only the backend has
    /// it. Use [`Engine::fetch`] to get it in the last case too.
    pub fn read<T: DataType>(&self, instance: &Instance, name: &str) -> Option<&[T]> {
        self.store.read(self.output_key(instance, name)?)
    }

    /// The offsets of the groups of an output, if it is grouped. `None` if
    /// there is no such output, it is not grouped, or only the backend has
    /// the offsets so far.
    pub fn groups(&self, instance: &Instance, name: &str) -> Option<&[u32]> {
        self.store.read(self.groups_key(instance, name)?)
    }

    fn groups_key(&self, instance: &Instance, name: &str) -> Option<ContentKey> {
        let program = self.programs.get(instance.program);
        let slot = program.outputs[program.output_index(name)?].slot;
        let groups = program.slot(slot).groups?;
        Some(instance.keys.as_ref()?.slots[groups.0 as usize])
    }

    /// Like [`Engine::groups`], but downloads the offsets first if need be.
    pub fn fetch_groups(&mut self, instance: &Instance, name: &str) -> Option<&[u32]> {
        self.fetch_key(self.groups_key(instance, name)?)
    }

    /// Like [`Engine::read`], but downloads the output first if only the
    /// backend has it. That can take a while, so do it for what the CPU
    /// really needs to see and not for everything.
    pub fn fetch<T: DataType>(&mut self, instance: &Instance, name: &str) -> Option<&[T]> {
        self.fetch_key(self.output_key(instance, name)?)
    }

    /// [`Engine::fetch`] for a value which is known by its key. With `u32`
    /// as type this gives the plain words of any value.
    pub fn fetch_key<T: DataType>(&mut self, key: ContentKey) -> Option<&[T]> {
        if self.store.contains(key) && !self.store.is_resident(key) {
            self.executor.download(&mut self.store, &[key]);
        }
        self.store.read(key)
    }

    /// Ends the frame: results nobody asked for lately are dropped until
    /// the store fits into `budget_words`.
    pub fn end_frame(&mut self, budget_words: u64) {
        self.store.evict(budget_words);
        self.store.end_frame();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        node::PortDecl,
        program::{BuildError, ProgramBuilder},
    };

    fn zip(inputs: &Inputs<'_>, outputs: &mut Outputs, f: fn(f32, f32) -> f32) {
        let (a, b) = (inputs.get::<f32>(0), inputs.value::<f32>(1));
        let out = outputs.alloc::<f32>(0, a.len());
        for (out, a) in out.iter_mut().zip(a) {
            *out = f(*a, b);
        }
    }

    const BINARY_IN: &[PortDecl] = &[PortDecl::new::<f32>("values"), PortDecl::new::<f32>("by")];
    const BINARY_OUT: &[PortDecl] = &[PortDecl::new::<f32>("values")];

    const fn binary(id: &'static str, cpu: crate::node::CpuKernel) -> NodeDef {
        NodeDef {
            id,
            display_name: id,
            inputs: BINARY_IN,
            outputs: BINARY_OUT,
            version: 0,
            cpu,
            gpu: &[],
        }
    }

    static SCALE: NodeDef = binary("test.scale", |i, o| zip(i, o, |a, b| a * b));
    static OFFSET: NodeDef = binary("test.offset", |i, o| zip(i, o, |a, b| a + b));
    static SUM: NodeDef = NodeDef {
        id: "test.sum",
        display_name: "Sum",
        inputs: &[PortDecl::new::<f32>("a"), PortDecl::new::<f32>("b")],
        outputs: &[PortDecl::new::<f32>("sum"), PortDecl::new::<u32>("count")],
        version: 0,
        cpu: |inputs, outputs| {
            let (a, b) = (inputs.get::<f32>(0), inputs.get::<f32>(1));
            let sum: Vec<_> = a.iter().zip(b).map(|(a, b)| a + b).collect();
            outputs.set(0, &sum);
            outputs.set(1, &[sum.len() as u32]);
        },
        gpu: &[],
    };

    /// out = values * scale + values * 2 + offset
    fn program() -> crate::program::Program {
        let mut b = ProgramBuilder::new("test");
        let values = b.param::<f32>("values", &[1., 2., 3.]);
        let scale = b.param::<f32>("scale", &[10.]);
        let offset = b.param::<f32>("offset", &[0.5]);
        let two = b.constant::<f32>(&[2.]);

        let scaled = b.node(&SCALE, &[values, scale]).unwrap()[0];
        let doubled = b.node(&SCALE, &[values, two]).unwrap()[0];
        let shifted = b.node(&OFFSET, &[doubled, offset]).unwrap()[0];
        let sum = b.node(&SUM, &[scaled, shifted]).unwrap();
        // Nothing reads this, so it must never run.
        b.node(&OFFSET, &[scaled, offset]).unwrap();

        b.output("out", sum[0]);
        b.output("count", sum[1]);
        b.finish()
    }

    #[test]
    fn evaluates_only_what_the_outputs_need() {
        let mut engine = Engine::default();
        let id = engine.add_program(program());
        let mut instance = engine.instantiate(id);

        assert_eq!(engine.evaluate([&mut instance]), 4);
        assert_eq!(
            engine.read::<f32>(&instance, "out"),
            Some(&[12.5, 24.5, 36.5][..])
        );
        assert_eq!(engine.read::<u32>(&instance, "count"), Some(&[3][..]));
        assert_eq!(engine.evaluate([&mut instance]), 0);
    }

    #[test]
    fn equal_parameters_share_results() {
        let mut engine = Engine::default();
        let id = engine.add_program(program());
        let mut a = engine.instantiate(id);
        let mut b = engine.instantiate(id);
        let mut c = engine.instantiate(id);
        engine.set_param(&mut c, "offset", &[1.5f32]).unwrap();

        // `a` and `b` are the same work, `c` only differs behind `offset`.
        assert_eq!(engine.evaluate([&mut a, &mut b, &mut c]), 4 + 2);
        assert_eq!(engine.output_key(&a, "out"), engine.output_key(&b, "out"));
        assert_ne!(engine.output_key(&a, "out"), engine.output_key(&c, "out"));
        assert_eq!(
            engine.read::<f32>(&c, "out"),
            Some(&[13.5, 25.5, 37.5][..])
        );
    }

    #[test]
    fn a_change_reruns_only_what_depends_on_it() {
        let mut engine = Engine::default();
        let id = engine.add_program(program());
        let mut instance = engine.instantiate(id);
        engine.evaluate([&mut instance]);

        engine.set_param(&mut instance, "scale", &[1f32]).unwrap();
        // scale and sum, but neither the doubling nor the offset.
        assert_eq!(engine.evaluate([&mut instance]), 2);
        assert_eq!(
            engine.read::<f32>(&instance, "out"),
            Some(&[3.5, 6.5, 9.5][..])
        );

        // Going back to a value that was seen before costs nothing.
        engine.set_param(&mut instance, "scale", &[10f32]).unwrap();
        assert_eq!(engine.evaluate([&mut instance]), 0);
    }

    #[test]
    fn evicted_results_are_recomputed() {
        let mut engine = Engine::default();
        let id = engine.add_program(program());
        let mut instance = engine.instantiate(id);
        engine.evaluate([&mut instance]);

        engine.end_frame(u64::MAX);
        engine.end_frame(0);
        assert!(engine.store().is_empty());
        assert_eq!(engine.read::<f32>(&instance, "out"), None);

        assert_eq!(engine.evaluate([&mut instance]), 4);
        assert_eq!(
            engine.read::<f32>(&instance, "out"),
            Some(&[12.5, 24.5, 36.5][..])
        );
    }

    #[test]
    fn a_program_can_call_another_one() {
        let mut engine = Engine::default();
        let inner = engine.add_program(program());

        let mut b = ProgramBuilder::new("outer");
        let values = b.param::<f32>("values", &[1., 2., 3.]);
        let three = b.constant::<f32>(&[3.]);
        let tripled = b.node(&SCALE, &[values, three]).unwrap()[0];
        // `scale` and `offset` keep the defaults of the inner program.
        let direct = b
            .call(engine.programs(), inner, &[Some(values), None, None])
            .unwrap();
        let nested = b
            .call(engine.programs(), inner, &[Some(tripled), None, None])
            .unwrap();
        b.output("direct", direct[0]);
        b.output("nested", nested[0]);
        let outer = engine.add_program(b.finish());

        let mut standalone = engine.instantiate(inner);
        assert_eq!(engine.evaluate([&mut standalone]), 4);

        // The direct call is the standalone instance again, so only the
        // tripling and the second call are new.
        let mut instance = engine.instantiate(outer);
        assert_eq!(engine.evaluate([&mut instance]), 1 + 4);
        assert_eq!(
            engine.output_key(&instance, "direct"),
            engine.output_key(&standalone, "out")
        );
        assert_eq!(
            engine.read::<f32>(&instance, "nested"),
            Some(&[36.5, 72.5, 108.5][..])
        );
    }

    mod generic {
        use super::*;
        use crate::{
            node::{GenericNode, GenericPort, NodeRegistry},
            types::Trait,
        };

        const NUMBER: Trait = Trait { name: "number" };

        static DOUBLE: GenericNode = GenericNode {
            id: "test.double",
            display_name: "Double",
            inputs: &[GenericPort::implementing("values", NUMBER)],
            outputs: &[GenericPort::implementing("values", NUMBER)],
        };
        static DOUBLE_F32: NodeDef = NodeDef {
            id: "test.double<f32>",
            display_name: "Double",
            inputs: &[PortDecl::new::<f32>("values")],
            outputs: &[PortDecl::new::<f32>("values")],
            version: 0,
            cpu: |i, o| o.set(0, &i.get::<f32>(0).iter().map(|v| v * 2.).collect::<Vec<_>>()),
            gpu: &[],
        };
        static DOUBLE_U32: NodeDef = NodeDef {
            id: "test.double<u32>",
            display_name: "Double",
            inputs: &[PortDecl::new::<u32>("values")],
            outputs: &[PortDecl::new::<u32>("values")],
            version: 0,
            cpu: |i, o| o.set(0, &i.get::<u32>(0).iter().map(|v| v * 2).collect::<Vec<_>>()),
            gpu: &[],
        };

        fn registry() -> NodeRegistry {
            let mut registry = NodeRegistry::new();
            registry.register_instance(&DOUBLE, &DOUBLE_F32);
            registry.register_instance(&DOUBLE, &DOUBLE_U32);
            registry
        }

        #[test]
        fn the_connected_type_picks_the_version() {
            let registry = registry();
            let mut b = ProgramBuilder::new("generic");
            let floats = b.constant::<f32>(&[1.5]);
            let ints = b.constant::<u32>(&[4]);
            let floats = b.generic(&registry, &DOUBLE, &[floats]).unwrap()[0];
            let ints = b.generic(&registry, &DOUBLE, &[ints]).unwrap()[0];
            b.output("floats", floats);
            b.output("ints", ints);

            let mut engine = Engine::default();
            let id = engine.add_program(b.finish());
            let mut instance = engine.instantiate(id);
            engine.evaluate([&mut instance]);
            assert_eq!(engine.read::<f32>(&instance, "floats"), Some(&[3.0][..]));
            assert_eq!(engine.read::<u32>(&instance, "ints"), Some(&[8][..]));
        }

        #[test]
        fn a_type_has_to_implement_the_trait() {
            let registry = registry();
            let mut b = ProgramBuilder::new("generic");
            let vector = b.constant::<glam::Vec3>(&[glam::Vec3::ONE]);
            assert_eq!(
                b.generic(&registry, &DOUBLE, &[vector]),
                Err(BuildError::Trait {
                    target: "test.double".into(),
                    port: "values".into(),
                    expected: "number",
                    found: "vec3",
                })
            );
        }

        #[test]
        fn the_registry_knows_who_implements_what() {
            let registry = registry();
            let mut names: Vec<_> = registry
                .implementors(NUMBER)
                .iter()
                .map(|ty| ty.name)
                .collect();
            names.sort();
            assert_eq!(names, ["f32", "u32"]);
            assert_eq!(registry.instances(&DOUBLE).len(), 2);
            assert!(registry.generic("test.double").is_some());
            assert!(registry.get("test.double<u32>").is_some());
        }

        #[test]
        #[should_panic(expected = "is not a version of")]
        fn versions_have_to_line_up_with_their_generic_node() {
            NodeRegistry::new().register_instance(&DOUBLE, &SCALE);
        }
    }

    #[test]
    fn planning_and_running_can_be_apart() {
        let mut engine = Engine::default();
        let id = engine.add_program(program());
        let mut a = engine.instantiate(id);
        let mut b = engine.instantiate(id);
        engine.set_param(&mut b, "offset", &[1.5f32]).unwrap();

        // Planned in two goes, the second one only adds what is new.
        assert_eq!(engine.plan([&mut a]), 4);
        assert_eq!(engine.plan([&mut b]), 2);
        assert_eq!(engine.pending(), 6);

        // The keys are known, the values are not there yet.
        assert!(engine.output_key(&a, "out").is_some());
        assert_eq!(engine.read::<f32>(&a, "out"), None);

        assert_eq!(engine.flush(), 6);
        assert_eq!(engine.read::<f32>(&a, "out"), Some(&[12.5, 24.5, 36.5][..]));

        // What is not needed anymore can be dropped, and planned again.
        engine.set_param(&mut a, "scale", &[3f32]).unwrap();
        assert_eq!(engine.plan([&mut a]), 2);
        engine.discard();
        assert_eq!((engine.pending(), engine.flush()), (0, 0));
        assert_eq!(engine.plan([&mut a]), 2);
        engine.flush();
        assert_eq!(engine.read::<f32>(&b, "out"), Some(&[13.5, 25.5, 37.5][..]));
        assert_eq!((engine.pending(), engine.flush()), (0, 0));
    }

    mod for_each {
        use super::*;

        /// `value * 10`, and how many values there were.
        fn tenfold(engine: &mut Engine) -> ProgramId {
            let mut b = ProgramBuilder::new("tenfold");
            let value = b.param::<f32>("value", &[0.]);
            let ten = b.constant::<f32>(&[10.]);
            let out = b.node(&SCALE, &[value, ten]).unwrap()[0];
            let sum = b.node(&SUM, &[out, value]).unwrap();
            b.output("out", out);
            b.output("count", sum[1]);
            engine.add_program(b.finish())
        }

        fn each(engine: &mut Engine, max: u32) -> ProgramId {
            let inner = tenfold(engine);
            let mut b = ProgramBuilder::new("each");
            let values = b.param::<f32>("values", &[1., 2., 3.]);
            let out = b
                .for_each(engine.programs(), inner, &[Some(values)], 0, max)
                .unwrap();
            b.output("out", out[0]);
            b.output("count", out[1]);
            engine.add_program(b.finish())
        }

        #[test]
        fn a_program_runs_for_each_element() {
            let mut engine = Engine::default();
            let id = each(&mut engine, 100);
            let mut instance = engine.instantiate(id);

            // Two nodes for each of the three elements.
            assert_eq!(engine.evaluate([&mut instance]), 6);
            assert_eq!(engine.read::<f32>(&instance, "out"), Some(&[10., 20., 30.][..]));
            // Every run saw a single element.
            assert_eq!(engine.read::<u32>(&instance, "count"), Some(&[1, 1, 1][..]));
            assert!(engine.overflows().is_empty());
            assert_eq!(engine.evaluate([&mut instance]), 0);
        }

        #[test]
        fn elements_which_stay_the_same_are_not_run_again() {
            let mut engine = Engine::default();
            let id = each(&mut engine, 100);
            let mut instance = engine.instantiate(id);
            engine.evaluate([&mut instance]);

            // One new element, and one which is there twice now.
            engine
                .set_param(&mut instance, "values", &[1f32, 2., 5., 5.])
                .unwrap();
            assert_eq!(engine.evaluate([&mut instance]), 2);
            assert_eq!(
                engine.read::<f32>(&instance, "out"),
                Some(&[10., 20., 50., 50.][..])
            );

            engine.set_param(&mut instance, "values", &[] as &[f32]).unwrap();
            assert_eq!(engine.evaluate([&mut instance]), 0);
            assert_eq!(engine.read::<f32>(&instance, "out"), Some(&[][..]));
        }

        #[test]
        fn more_elements_than_allowed_are_left_out_and_reported() {
            let mut engine = Engine::default();
            let id = each(&mut engine, 2);
            let mut instance = engine.instantiate(id);
            engine.evaluate([&mut instance]);

            assert_eq!(engine.read::<f32>(&instance, "out"), Some(&[10., 20.][..]));
            assert_eq!(
                engine.overflows(),
                [Overflow {
                    program: "tenfold".into(),
                    elements: 3,
                    max: 2
                }]
            );
            // Nothing ran the next time, so nothing overflowed.
            engine.evaluate([&mut instance]);
            assert!(engine.overflows().is_empty());
        }

        #[test]
        fn calls_for_each_element_can_be_nested() {
            let mut engine = Engine::default();
            let each = each(&mut engine, 100);

            // For each list of values (here: twice the same), all of them.
            let mut b = ProgramBuilder::new("each of each");
            let values = b.param::<f32>("values", &[1., 2.]);
            let pairs = b.node(&SUM, &[values, values]).unwrap()[0];
            let out = b
                .for_each(engine.programs(), each, &[Some(pairs)], 0, 100)
                .unwrap();
            b.output("out", out[0]);
            let id = engine.add_program(b.finish());
            let mut instance = engine.instantiate(id);
            engine.evaluate([&mut instance]);

            // 2 and 4, each a list of one, times ten.
            assert_eq!(engine.read::<f32>(&instance, "out"), Some(&[20., 40.][..]));
            assert_eq!(engine.programs_mut().retain([id]), 0);
        }

        #[test]
        fn something_has_to_be_given_to_take_the_elements_of() {
            let mut engine = Engine::default();
            let inner = tenfold(&mut engine);
            let mut b = ProgramBuilder::new("broken");
            assert!(matches!(
                b.for_each(engine.programs(), inner, &[None], 0, 1),
                Err(BuildError::Element { .. })
            ));
            assert!(matches!(
                b.for_each(engine.programs(), inner, &[None], 3, 1),
                Err(BuildError::Element { .. })
            ));
        }
    }

    mod groups {
        use super::*;
        use crate::node::Card;

        /// The sum of every group, and how many elements each had.
        static SUMS: NodeDef = NodeDef {
            id: "test.sums",
            display_name: "Sums",
            version: 0,
            inputs: &[PortDecl::new::<f32>("values").grouped()],
            outputs: &[
                PortDecl::new::<f32>("sums"),
                PortDecl::new::<u32>("lengths"),
            ],
            cpu: |inputs, outputs| {
                let values = inputs.get::<f32>(0);
                let groups = inputs.groups(0);
                let sums: Vec<f32> = groups.iter().map(|g| values[g.clone()].iter().sum()).collect();
                let lengths: Vec<u32> = groups.iter().map(|g| g.len() as u32).collect();
                outputs.set(0, &sums);
                outputs.set(1, &lengths);
            },
            gpu: &[],
        };

        /// Splits a list into groups of `size` elements.
        static SPLIT: NodeDef = NodeDef {
            id: "test.split",
            display_name: "Split",
            version: 0,
            inputs: &[
                PortDecl::new::<f32>("values"),
                PortDecl::new::<u32>("size").one(),
            ],
            outputs: &[PortDecl::new::<f32>("values").grouped()],
            cpu: |inputs, outputs| {
                let values = inputs.get::<f32>(0);
                let size = inputs.value::<u32>(1).max(1) as usize;
                outputs.set(0, values);
                let offsets: Vec<u32> = (0..values.len().div_ceil(size))
                    .map(|group| (group * size) as u32)
                    .chain([values.len() as u32])
                    .collect();
                outputs.set_groups(0, &offsets);
            },
            gpu: &[],
        };

        /// Doubles every element and keeps the groups as they are.
        static DOUBLE: NodeDef = NodeDef {
            id: "test.double_grouped",
            display_name: "Double",
            version: 0,
            inputs: &[PortDecl::new::<f32>("values").grouped()],
            outputs: &[PortDecl::new::<f32>("values").grouped_like(0)],
            cpu: |inputs, outputs| {
                let doubled: Vec<f32> = inputs.get::<f32>(0).iter().map(|v| v * 2.0).collect();
                outputs.set(0, &doubled);
            },
            gpu: &[],
        };

        fn split(b: &mut ProgramBuilder, values: &[f32], size: u32) -> SlotId {
            let values = b.constant(values);
            let size = b.constant(&[size]);
            b.node(&SPLIT, &[values, size]).unwrap()[0]
        }

        #[test]
        fn groups_go_from_node_to_node() {
            let mut b = ProgramBuilder::new("groups");
            let split = split(&mut b, &[1., 2., 3., 4., 5.], 2);
            assert_eq!(b.slot(split).card, Card::Grouped);
            let doubled = b.node(&DOUBLE, &[split]).unwrap()[0];
            // Groups which are passed along are the very same slot.
            assert_eq!(b.slot(doubled).groups, b.slot(split).groups);
            let sums = b.node(&SUMS, &[doubled]).unwrap();
            b.output("doubled", doubled);
            b.output("sums", sums[0]);
            b.output("lengths", sums[1]);

            let mut engine = Engine::default();
            let id = engine.add_program(b.finish());
            let mut instance = engine.instantiate(id);
            assert_eq!(engine.evaluate([&mut instance]), 3);
            assert_eq!(engine.read::<f32>(&instance, "sums"), Some(&[6., 14., 10.][..]));
            assert_eq!(engine.read::<u32>(&instance, "lengths"), Some(&[2, 2, 1][..]));
            assert_eq!(engine.groups(&instance, "doubled"), Some(&[0, 2, 4, 5][..]));
            assert_eq!(engine.groups(&instance, "sums"), None);
        }

        #[test]
        fn a_list_without_groups_is_a_single_group() {
            let mut b = ProgramBuilder::new("one group");
            let values = b.constant::<f32>(&[1., 2., 3.]);
            let sums = b.node(&SUMS, &[values]).unwrap();
            b.output("sums", sums[0]);
            let mut engine = Engine::default();
            let id = engine.add_program(b.finish());
            let mut instance = engine.instantiate(id);
            engine.evaluate([&mut instance]);
            assert_eq!(engine.read::<f32>(&instance, "sums"), Some(&[6.][..]));
        }

        #[test]
        fn a_single_value_is_not_a_list() {
            let mut b = ProgramBuilder::new("cards");
            let values = b.constant::<f32>(&[1., 2.]);
            let sizes = b.constant::<u32>(&[1, 2]);
            assert_eq!(
                b.node(&SPLIT, &[values, sizes]),
                Err(BuildError::Card {
                    target: "test.split".into(),
                    port: "size".into()
                })
            );
            // A parameter can be single too.
            let size = b.param_one("size", u32::INFO, &[2]);
            assert!(b.node(&SPLIT, &[values, size]).is_ok());
            let many = b.param::<u32>("many", &[2]);
            assert!(b.node(&SPLIT, &[values, many]).is_err());
        }

        #[test]
        fn a_program_runs_for_each_group() {
            let mut engine = Engine::default();
            let mut b = ProgramBuilder::new("sum of one group");
            let values = b.param::<f32>("values", &[]);
            let sums = b.node(&SUMS, &[values]).unwrap();
            b.output("sum", sums[0]);
            let inner = engine.add_program(b.finish());

            let mut b = ProgramBuilder::new("each group");
            let split = split(&mut b, &[1., 2., 3., 4., 5.], 2);
            let each = b
                .for_each(engine.programs(), inner, &[Some(split)], 0, 100)
                .unwrap();
            assert_eq!(b.slot(each[0]).card, Card::Grouped);
            b.output("sums", each[0]);
            let id = engine.add_program(b.finish());

            let mut instance = engine.instantiate(id);
            engine.evaluate([&mut instance]);
            // One run per group, and every run is a group of its own.
            assert_eq!(engine.read::<f32>(&instance, "sums"), Some(&[3., 7., 5.][..]));
            assert_eq!(engine.groups(&instance, "sums"), Some(&[0, 1, 2, 3][..]));
        }

        #[test]
        fn groups_go_into_and_out_of_programs() {
            let mut engine = Engine::default();
            // Doubles what it is given and keeps its groups.
            let mut b = ProgramBuilder::new("double");
            let values = b.param_grouped("values", f32::INFO, &[]);
            let doubled = b.node(&DOUBLE, &[values]).unwrap()[0];
            b.output("doubled", doubled);
            let inner = engine.add_program(b.finish());
            assert_eq!(engine.programs().get(inner).visible_params().count(), 1);

            let mut b = ProgramBuilder::new("outer");
            let split = split(&mut b, &[1., 2., 3.], 1);
            let doubled = b.call(engine.programs(), inner, &[Some(split)]).unwrap()[0];
            let sums = b.node(&SUMS, &[doubled]).unwrap();
            b.output("sums", sums[0]);
            let id = engine.add_program(b.finish());

            let mut instance = engine.instantiate(id);
            engine.evaluate([&mut instance]);
            assert_eq!(engine.read::<f32>(&instance, "sums"), Some(&[2., 4., 6.][..]));
        }
    }

    #[test]
    fn unused_programs_can_be_removed() {
        let mut engine = Engine::default();
        let inner = engine.add_program(program());
        let unused = engine.add_program(program());

        let mut b = ProgramBuilder::new("outer");
        let values = b.param::<f32>("values", &[1.]);
        let called = b
            .call(engine.programs(), inner, &[Some(values), None, None])
            .unwrap();
        b.output("out", called[0]);
        let outer = engine.add_program(b.finish());

        // `inner` stays because `outer` calls it.
        assert_eq!(engine.programs_mut().retain([outer]), 1);
        assert!(engine.programs().contains(inner) && !engine.programs().contains(unused));
        assert_eq!(engine.programs().len(), 2);

        let mut instance = engine.instantiate(outer);
        engine.evaluate([&mut instance]);
        assert_eq!(engine.read::<f32>(&instance, "out"), Some(&[12.5][..]));

        // Ids are not reused, so an old one never names a new program.
        let next = engine.add_program(program());
        assert!(next != unused && engine.programs().find(program().hash).is_some());

        assert_eq!(engine.programs_mut().retain([]), 3);
        assert!(engine.programs().is_empty());
    }

    #[test]
    fn types_are_checked() {
        let mut b = ProgramBuilder::new("broken");
        let count = b.constant::<u32>(&[1]);
        let values = b.param::<f32>("values", &[]);
        assert!(matches!(
            b.node(&SCALE, &[values, count]),
            Err(BuildError::Type { .. })
        ));
        assert!(matches!(
            b.node(&SCALE, &[values]),
            Err(BuildError::Arity { .. })
        ));

        let mut engine = Engine::default();
        let id = engine.add_program(program());
        let mut instance = engine.instantiate(id);
        assert!(matches!(
            engine.set_param(&mut instance, "scale", &[1u32]),
            Err(ParamError::Type { .. })
        ));
        assert_eq!(
            engine.set_param(&mut instance, "nope", &[1f32]),
            Err(ParamError::Unknown("nope".into()))
        );
    }
}
