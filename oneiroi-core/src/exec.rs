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
            buffers.resize_with(item.node.outputs.len().max(buffers.len()), Vec::new);
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
                            ContentKey::derive(node.id, output as u32, &inputs);
                    }
                }
                OpKind::Call(callee) => {
                    let callee = programs.get(callee);
                    let keys = Keys::derive(programs, callee, &inputs);
                    for (slot, output) in op.outputs.iter().zip(&callee.outputs) {
                        slots[slot.0 as usize] = keys.slots[output.slot.0 as usize];
                    }
                    calls.push((index as u32, keys));
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
    work: &'a mut Vec<Work>,
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
                        self.work.push(Work {
                            node,
                            inputs: keys(&op.inputs),
                            outputs,
                        });
                    }
                    OpKind::Call(callee) => {
                        let program = self.programs.get(callee);
                        let callee = Scope {
                            program,
                            keys: scope.keys.call(op_index),
                            caller: Some((scope, &op.inputs)),
                        };
                        self.ensure(&callee, program.outputs[*output as usize].slot);
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
    work: Vec<Work>,
    scheduled: HashSet<ContentKey>,
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

        let words: &[u32] = bytemuck::cast_slice(value);
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
        self.work.clear();
        self.scheduled.clear();

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

        self.executor.execute(&mut self.store, &self.work);
        self.work.len()
    }

    /// The key of an output, once the instance was evaluated.
    pub fn output_key(&self, instance: &Instance, name: &str) -> Option<ContentKey> {
        let program = self.programs.get(instance.program);
        let slot = program.outputs[program.output_index(name)?].slot;
        Some(instance.keys.as_ref()?.slots[slot.0 as usize])
    }

    /// An output of an evaluated instance. `None` if there is no such
    /// output, or it is not in the store (anymore).
    pub fn read<T: DataType>(&self, instance: &Instance, name: &str) -> Option<&[T]> {
        self.store.read(self.output_key(instance, name)?)
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
        node::{PortDecl, Shape},
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
            shape: Shape::Map,
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
        shape: Shape::Map,
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
            shape: Shape::Map,
            cpu: |i, o| o.set(0, &i.get::<f32>(0).iter().map(|v| v * 2.).collect::<Vec<_>>()),
            gpu: &[],
        };
        static DOUBLE_U32: NodeDef = NodeDef {
            id: "test.double<u32>",
            display_name: "Double",
            inputs: &[PortDecl::new::<u32>("values")],
            outputs: &[PortDecl::new::<u32>("values")],
            shape: Shape::Map,
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
