//! The static, compiled form of a graph.
//!
//! A [`Program`] is built once per graph definition and shared by all of
//! its instances. A graph inside a graph is never copied into its parent:
//! the parent holds a call to the other program, so every definition
//! exists exactly once in a [`ProgramSet`].

use std::{
    fmt,
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::{
    node::{GenericNode, NodeDef, NodeRegistry, PortType},
    store::ContentKey,
    types::{DataType, TypeInfo},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotId(pub(crate) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProgramId(pub(crate) u32);

/// Where the value of a slot comes from.
#[derive(Debug)]
pub enum SlotSource {
    /// Supplied from outside: by an instance, or by the calling program.
    Param(u32),
    /// Baked into the program.
    Const { words: Box<[u32]>, key: ContentKey },
    /// The `output`-th result of the `op`-th operation.
    Op { op: u32, output: u32 },
}

#[derive(Debug)]
pub struct SlotDecl {
    pub ty: TypeInfo,
    pub source: SlotSource,
}

#[derive(Debug)]
pub struct ParamDecl {
    pub name: String,
    pub slot: SlotId,
    pub default: Box<[u32]>,
}

#[derive(Debug)]
pub struct OutputDecl {
    pub name: String,
    pub slot: SlotId,
}

#[derive(Debug)]
pub enum OpKind {
    Node(&'static NodeDef),
    /// Runs another program. `inputs` are its parameters, in order.
    Call(ProgramId),
}

#[derive(Debug)]
pub struct Op {
    pub kind: OpKind,
    pub inputs: Box<[SlotId]>,
    pub outputs: Box<[SlotId]>,
}

#[derive(Debug)]
pub struct Program {
    pub name: String,
    /// Identifies the structure of the program, not its parameter values.
    pub hash: u64,
    pub slots: Box<[SlotDecl]>,
    pub params: Box<[ParamDecl]>,
    /// In dependency order: an operation only reads earlier slots.
    pub ops: Box<[Op]>,
    pub outputs: Box<[OutputDecl]>,
}

impl Program {
    pub fn param_index(&self, name: &str) -> Option<usize> {
        self.params.iter().position(|param| param.name == name)
    }

    pub fn output_index(&self, name: &str) -> Option<usize> {
        self.outputs.iter().position(|output| output.name == name)
    }

    pub fn slot(&self, slot: SlotId) -> &SlotDecl {
        &self.slots[slot.0 as usize]
    }
}

/// Every program of an application. A program can only call programs which
/// were added before it, so calls can never form a cycle.
#[derive(Debug, Default)]
pub struct ProgramSet {
    programs: Vec<Program>,
}

impl ProgramSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, program: Program) -> ProgramId {
        self.programs.push(program);
        ProgramId(self.programs.len() as u32 - 1)
    }

    /// The program with this [`Program::hash`], if there is one.
    pub fn find(&self, hash: u64) -> Option<ProgramId> {
        let index = self.programs.iter().position(|p| p.hash == hash)?;
        Some(ProgramId(index as u32))
    }

    pub fn get(&self, id: ProgramId) -> &Program {
        &self.programs[id.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.programs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.programs.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    Arity {
        target: String,
        expected: usize,
        found: usize,
    },
    Type {
        target: String,
        port: String,
        expected: &'static str,
        found: &'static str,
    },
    /// A port takes a trait and the connected data type does not implement it.
    Trait {
        target: String,
        port: String,
        expected: &'static str,
        found: &'static str,
    },
    /// Every port is fine on its own, but there is no version of the
    /// generic node for these data types together.
    Unresolved { target: String },
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arity {
                target,
                expected,
                found,
            } => write!(f, "`{target}` takes {expected} inputs but got {found}"),
            Self::Type {
                target,
                port,
                expected,
                found,
            } => write!(f, "`{target}.{port}` expects `{expected}` but got `{found}`"),
            Self::Trait {
                target,
                port,
                expected,
                found,
            } => write!(
                f,
                "`{target}.{port}` expects a `{expected}` but `{found}` is none"
            ),
            Self::Unresolved { target } => {
                write!(f, "`{target}` has no version for these inputs")
            }
        }
    }
}

impl std::error::Error for BuildError {}

/// Assembles a [`Program`] one operation at a time.
///
/// Lowering an authored graph means walking it in dependency order and
/// calling [`ProgramBuilder::node`] or [`ProgramBuilder::call`] per node.
pub struct ProgramBuilder {
    name: String,
    slots: Vec<SlotDecl>,
    params: Vec<ParamDecl>,
    ops: Vec<Op>,
    outputs: Vec<OutputDecl>,
    hasher: DefaultHasher,
}

impl ProgramBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            slots: Vec::new(),
            params: Vec::new(),
            ops: Vec::new(),
            outputs: Vec::new(),
            hasher: DefaultHasher::new(),
        }
    }

    fn push_slot(&mut self, ty: TypeInfo, source: SlotSource) -> SlotId {
        self.slots.push(SlotDecl { ty, source });
        SlotId(self.slots.len() as u32 - 1)
    }

    /// A value every instance can set. This is what an exposed property or
    /// an input socket of the graph lowers to.
    pub fn param<T: DataType>(&mut self, name: impl Into<String>, default: &[T]) -> SlotId {
        self.param_words(name, T::INFO, bytemuck::cast_slice(default))
    }

    /// [`ProgramBuilder::param`] for a type which is only known by its
    /// [`TypeInfo`]. `default` has to be whole elements of it.
    pub fn param_words(
        &mut self,
        name: impl Into<String>,
        ty: TypeInfo,
        default: &[u32],
    ) -> SlotId {
        let slot = self.push_slot(ty, SlotSource::Param(self.params.len() as u32));
        ("param", ty.name, default).hash(&mut self.hasher);
        self.params.push(ParamDecl {
            name: name.into(),
            slot,
            default: default.into(),
        });
        slot
    }

    /// A value which is the same for every instance. This is what a
    /// property that is not exposed lowers to.
    pub fn constant<T: DataType>(&mut self, value: &[T]) -> SlotId {
        self.constant_words(T::INFO, bytemuck::cast_slice(value))
    }

    /// [`ProgramBuilder::constant`] for a type which is only known by its
    /// [`TypeInfo`]. `words` has to be whole elements of it.
    pub fn constant_words(&mut self, ty: TypeInfo, words: &[u32]) -> SlotId {
        let key = ContentKey::of_data(&ty, words);
        ("const", key).hash(&mut self.hasher);
        self.push_slot(
            ty,
            SlotSource::Const {
                words: words.into(),
                key,
            },
        )
    }

    pub fn node(
        &mut self,
        node: &'static NodeDef,
        inputs: &[SlotId],
    ) -> Result<Vec<SlotId>, BuildError> {
        if inputs.len() != node.inputs.len() {
            return Err(BuildError::Arity {
                target: node.id.into(),
                expected: node.inputs.len(),
                found: inputs.len(),
            });
        }
        for (port, slot) in node.inputs.iter().zip(inputs) {
            self.check(node.id, port.name, port.ty, *slot)?;
        }

        ("node", node.id, inputs).hash(&mut self.hasher);
        let types = node.outputs.iter().map(|port| port.ty).collect();
        Ok(self.push_op(OpKind::Node(node), inputs.into(), types))
    }

    /// Adds the version of `generic` which fits the data types of `inputs`.
    pub fn generic(
        &mut self,
        registry: &NodeRegistry,
        generic: &GenericNode,
        inputs: &[SlotId],
    ) -> Result<Vec<SlotId>, BuildError> {
        if inputs.len() != generic.inputs.len() {
            return Err(BuildError::Arity {
                target: generic.id.into(),
                expected: generic.inputs.len(),
                found: inputs.len(),
            });
        }
        for (port, slot) in generic.inputs.iter().zip(inputs) {
            match port.ty {
                PortType::Exact(ty) => self.check(generic.id, port.name, ty, *slot)?,
                PortType::Trait(tr) => {
                    let found = self.slots[slot.0 as usize].ty;
                    if !registry.implements(&found, tr) {
                        return Err(BuildError::Trait {
                            target: generic.id.into(),
                            port: port.name.into(),
                            expected: tr.name,
                            found: found.name,
                        });
                    }
                }
            }
        }

        let fits = |node: &&&'static NodeDef| {
            node.inputs
                .iter()
                .zip(inputs)
                .all(|(port, slot)| port.ty == self.slots[slot.0 as usize].ty)
        };
        match registry.instances(generic).iter().find(fits) {
            Some(node) => self.node(node, inputs),
            None => Err(BuildError::Unresolved {
                target: generic.id.into(),
            }),
        }
    }

    /// Calls `callee`. An argument which is `None` uses the default of the
    /// parameter it stands for.
    pub fn call(
        &mut self,
        programs: &ProgramSet,
        callee: ProgramId,
        args: &[Option<SlotId>],
    ) -> Result<Vec<SlotId>, BuildError> {
        let program = programs.get(callee);
        if args.len() != program.params.len() {
            return Err(BuildError::Arity {
                target: program.name.clone(),
                expected: program.params.len(),
                found: args.len(),
            });
        }

        let mut inputs = Vec::with_capacity(args.len());
        for (param, arg) in program.params.iter().zip(args) {
            let ty = program.slot(param.slot).ty;
            inputs.push(match arg {
                Some(slot) => {
                    self.check(&program.name, &param.name, ty, *slot)?;
                    *slot
                }
                None => self.constant_words(ty, &param.default),
            });
        }

        ("call", program.hash, &inputs).hash(&mut self.hasher);
        let types = program
            .outputs
            .iter()
            .map(|output| program.slot(output.slot).ty)
            .collect();
        Ok(self.push_op(OpKind::Call(callee), inputs, types))
    }

    fn check(
        &self,
        target: &str,
        port: &str,
        expected: TypeInfo,
        slot: SlotId,
    ) -> Result<(), BuildError> {
        let found = self.slots[slot.0 as usize].ty;
        if found == expected {
            Ok(())
        } else {
            Err(BuildError::Type {
                target: target.into(),
                port: port.into(),
                expected: expected.name,
                found: found.name,
            })
        }
    }

    fn push_op(&mut self, kind: OpKind, inputs: Vec<SlotId>, outputs: Vec<TypeInfo>) -> Vec<SlotId> {
        let op = self.ops.len() as u32;
        let outputs: Vec<_> = outputs
            .into_iter()
            .enumerate()
            .map(|(output, ty)| {
                self.push_slot(
                    ty,
                    SlotSource::Op {
                        op,
                        output: output as u32,
                    },
                )
            })
            .collect();
        self.ops.push(Op {
            kind,
            inputs: inputs.into(),
            outputs: outputs.clone().into(),
        });
        outputs
    }

    pub fn slot_type(&self, slot: SlotId) -> TypeInfo {
        self.slots[slot.0 as usize].ty
    }

    pub fn output(&mut self, name: impl Into<String>, slot: SlotId) {
        ("output", slot).hash(&mut self.hasher);
        self.outputs.push(OutputDecl {
            name: name.into(),
            slot,
        });
    }

    pub fn finish(self) -> Program {
        Program {
            name: self.name,
            hash: self.hasher.finish(),
            slots: self.slots.into(),
            params: self.params.into(),
            ops: self.ops.into(),
            outputs: self.outputs.into(),
        }
    }
}
