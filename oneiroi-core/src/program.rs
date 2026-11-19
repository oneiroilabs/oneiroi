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
    node::{Card, GenericNode, Groups, NodeDef, NodeRegistry, PortType},
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
    pub card: Card,
    /// The offsets of its groups, a `u32` slot, if it is grouped.
    pub groups: Option<SlotId>,
    /// For instances: the shape they refer to.
    pub prototype: Option<Prototype>,
    /// For instances: how many elements all of their copies have together,
    /// a single `u32`.
    pub total: Option<SlotId>,
}

/// The shape instances refer to: a value of the type of their prototype
/// and its offsets. These are the slots of the value itself, nothing is
/// copied for the instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Prototype {
    pub elements: SlotId,
    pub groups: SlotId,
}

#[derive(Debug)]
pub struct ParamDecl {
    pub name: String,
    pub slot: SlotId,
    pub default: Box<[u32]>,
    /// Holds the offsets of another parameter. Nobody sets it by name:
    /// it comes along with what is passed for that one.
    pub hidden: bool,
}

#[derive(Debug)]
pub struct OutputDecl {
    pub name: String,
    pub slot: SlotId,
}

/// The name of the hidden parameter which holds the groups of `param`.
pub fn groups_param(param: &str) -> String {
    format!("{param}#groups")
}

/// The name of the hidden parameter which holds the shape of `param`, a
/// parameter of instances. Its offsets are in [`groups_param`] of it.
pub fn prototype_param(param: &str) -> String {
    format!("{param}#prototype")
}

/// The name of the hidden parameter which holds the total of `param`, a
/// parameter of instances.
pub fn total_param(param: &str) -> String {
    format!("{param}#total")
}

#[derive(Debug)]
pub enum OpKind {
    Node(&'static NodeDef),
    /// Runs another program. `inputs` are its parameters, in order.
    Call(ProgramId),
    /// Runs another program once for every element of its parameter
    /// `element`, with that element alone as the value of it, or once for
    /// every group if what is passed for it is grouped. Every output is
    /// what all of the runs gave, one group per run. At most `max` runs are
    /// made, the rest is left out and reported.
    ///
    /// Its inputs are those of a call, and then the offsets of what the
    /// elements are taken of. Its outputs are those of a call, and then
    /// the offsets of every one of them.
    ForEach {
        callee: ProgramId,
        element: u32,
        max: u32,
    },
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

    /// The parameters somebody sets, without the hidden ones.
    pub fn visible_params(&self) -> impl Iterator<Item = &ParamDecl> {
        self.params.iter().filter(|param| !param.hidden)
    }

    /// The slot of the callee which output `index` of a call to it is: one
    /// of its outputs, behind those the offsets of a grouped one, and then
    /// the shape and its offsets of one which is instances.
    pub fn call_output_slot(&self, index: usize) -> SlotId {
        self.call_output_slots()
            .nth(index)
            .expect("a call has outputs for the callee's outputs and what comes along")
    }

    /// Every slot a call of this program gives back, see
    /// [`Program::call_output_slot`].
    fn call_output_slots(&self) -> impl Iterator<Item = SlotId> + '_ {
        let slots = self.outputs.iter().map(|output| self.slot(output.slot));
        self.outputs
            .iter()
            .map(|output| output.slot)
            .chain(slots.clone().filter_map(|slot| slot.groups))
            .chain(slots.flat_map(|slot| {
                let along = slot.prototype.zip(slot.total);
                along.map(|(p, total)| [p.elements, p.groups, total]).into_iter().flatten()
            }))
    }

    /// The hidden parameters which hold the shape of `param` and its
    /// offsets, if it is a parameter of instances.
    pub fn prototype_of_param(&self, param: usize) -> Option<(usize, usize)> {
        let name = prototype_param(&self.params[param].name);
        Some((self.param_index(&name)?, self.param_index(&groups_param(&name))?))
    }

    /// The hidden parameter which holds the total of `param`, if it is a
    /// parameter of instances.
    pub fn total_of_param(&self, param: usize) -> Option<usize> {
        self.param_index(&total_param(&self.params[param].name))
    }

    /// The hidden parameter which holds the groups of `param`, if any.
    pub fn groups_of_param(&self, param: usize) -> Option<usize> {
        self.param_index(&groups_param(&self.params[param].name))
    }
}

/// Every program of an application. A program can only call programs which
/// were added before it, so calls can never form a cycle.
///
/// An id is never handed out twice, also not after its program was removed.
#[derive(Debug, Default)]
pub struct ProgramSet {
    programs: Vec<Option<Program>>,
}

impl ProgramSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, program: Program) -> ProgramId {
        self.programs.push(Some(program));
        ProgramId(self.programs.len() as u32 - 1)
    }

    /// The program with this [`Program::hash`], if there is one.
    pub fn find(&self, hash: u64) -> Option<ProgramId> {
        self.iter()
            .find(|(_, program)| program.hash == hash)
            .map(|(id, _)| id)
    }

    /// Panics when the program was removed.
    pub fn get(&self, id: ProgramId) -> &Program {
        self.try_get(id).expect("the program was removed")
    }

    pub fn try_get(&self, id: ProgramId) -> Option<&Program> {
        self.programs.get(id.0 as usize)?.as_ref()
    }

    pub fn contains(&self, id: ProgramId) -> bool {
        self.try_get(id).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = (ProgramId, &Program)> {
        self.programs
            .iter()
            .enumerate()
            .filter_map(|(index, program)| Some((ProgramId(index as u32), program.as_ref()?)))
    }

    /// The programs `id` calls directly.
    fn callees(&self, id: ProgramId) -> impl Iterator<Item = ProgramId> + '_ {
        self.try_get(id)
            .into_iter()
            .flat_map(|program| program.ops.iter())
            .filter_map(|op| match op.kind {
                OpKind::Call(callee) | OpKind::ForEach { callee, .. } => Some(callee),
                OpKind::Node(_) => None,
            })
    }

    /// Removes every program which is neither one of `roots` nor called by
    /// one of them, however indirect. Returns how many were removed.
    ///
    /// The roots are the programs something outside still holds on to:
    /// the ones with instances, and the ones graphs currently lower to.
    /// An instance of a removed program must not be evaluated again.
    pub fn retain(&mut self, roots: impl IntoIterator<Item = ProgramId>) -> usize {
        let mut used = vec![false; self.programs.len()];
        let mut open: Vec<_> = roots.into_iter().collect();
        while let Some(id) = open.pop() {
            let seen = used.get_mut(id.0 as usize);
            if let Some(seen @ false) = seen {
                *seen = true;
                open.extend(self.callees(id));
            }
        }

        let mut removed = 0;
        for (program, used) in self.programs.iter_mut().zip(used) {
            if !used && program.take().is_some() {
                removed += 1;
            }
        }
        removed
    }

    /// How many programs there are right now.
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
    /// A call for each element has to be given something to take the
    /// elements of.
    Element { target: String },
    /// A port takes a single element and is given a list.
    Card { target: String, port: String },
    /// A program which gives or takes instances is called for each element.
    Instances { target: String },
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
            Self::Element { target } => {
                write!(f, "`{target}` is called for each element of nothing")
            }
            Self::Card { target, port } => {
                write!(f, "`{target}.{port}` takes a single value but is given a list")
            }
            Self::Instances { target } => {
                write!(f, "`{target}` gives or takes instances and can not run for each element")
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
        let card = match &source {
            SlotSource::Const { words, .. } if words.len() == ty.words as usize => Card::One,
            _ => Card::Many,
        };
        self.slots.push(SlotDecl {
            ty,
            source,
            card,
            groups: None,
            prototype: None,
            total: None,
        });
        SlotId(self.slots.len() as u32 - 1)
    }

    fn slot_mut(&mut self, slot: SlotId) -> &mut SlotDecl {
        &mut self.slots[slot.0 as usize]
    }

    /// Offsets which say: a single group of everything.
    fn no_groups(&mut self) -> SlotId {
        self.constant_words(u32::INFO, &[])
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
        let name = name.into();
        let slot = self.push_slot(ty, SlotSource::Param(self.params.len() as u32));
        ("param", ty.name, default).hash(&mut self.hasher);
        self.params.push(ParamDecl {
            name: name.clone(),
            slot,
            default: default.into(),
            hidden: false,
        });
        // Instances come with their shape, in hidden parameters of its own.
        if let Some(prototype) = ty.prototype {
            let shape = prototype_param(&name);
            let elements = self.param_words(shape.clone(), *prototype, &[]);
            self.params.last_mut().unwrap().hidden = true;
            let groups = self.param_words(groups_param(&shape), u32::INFO, &[]);
            self.params.last_mut().unwrap().hidden = true;
            let total = self.param_words(total_param(&name), u32::INFO, &[0]);
            self.params.last_mut().unwrap().hidden = true;
            let decl = self.slot_mut(slot);
            decl.prototype = Some(Prototype { elements, groups });
            decl.total = Some(total);
        }
        slot
    }

    /// A parameter of a single element.
    pub fn param_one_of<T: DataType>(&mut self, name: impl Into<String>, default: T) -> SlotId {
        self.param_one(name, T::INFO, bytemuck::cast_slice(std::slice::from_ref(&default)))
    }

    /// [`ProgramBuilder::param_words`] for a parameter of a single element.
    pub fn param_one(&mut self, name: impl Into<String>, ty: TypeInfo, default: &[u32]) -> SlotId {
        let slot = self.param_words(name, ty, default);
        self.slot_mut(slot).card = Card::One;
        ("one", slot).hash(&mut self.hasher);
        slot
    }

    /// [`ProgramBuilder::param_words`] for a parameter which keeps the
    /// groups of what is passed for it, in a hidden parameter of its own.
    pub fn param_grouped(
        &mut self,
        name: impl Into<String>,
        ty: TypeInfo,
        default: &[u32],
    ) -> SlotId {
        let name = name.into();
        let slot = self.param_words(name.clone(), ty, default);
        let groups = self.param_words(groups_param(&name), u32::INFO, &[]);
        self.params.last_mut().unwrap().hidden = true;
        let decl = self.slot_mut(slot);
        decl.card = Card::Grouped;
        decl.groups = Some(groups);
        ("grouped", slot).hash(&mut self.hasher);
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
            if !self.slot(*slot).card.fits(port.card) {
                return Err(BuildError::Card {
                    target: node.id.into(),
                    port: port.name.into(),
                });
            }
        }

        // The offsets of every grouped input go along, behind the others,
        // and then the shapes of instances.
        let mut all = inputs.to_vec();
        for index in node.hidden_inputs() {
            let groups = match self.slot(inputs[index]).groups {
                Some(groups) => groups,
                None => self.no_groups(),
            };
            all.push(groups);
        }
        for index in node.prototype_inputs() {
            let prototype = self.prototype_of(inputs[index]);
            let total = self.total_of(inputs[index]);
            all.extend([prototype.elements, prototype.groups, total]);
        }

        ("node", node.id, node.version, &all).hash(&mut self.hasher);
        let types = node
            .outputs
            .iter()
            .map(|port| port.ty)
            .chain(node.hidden_outputs().map(|_| u32::INFO))
            .chain(node.total_outputs().map(|_| u32::INFO))
            .collect();
        let slots = self.push_op(OpKind::Node(node), all.clone(), types);

        let visible = node.outputs.len();
        let mut own = slots[visible..].iter();
        let mut totals = slots[visible + node.hidden_outputs().count()..].iter();
        for (port, slot) in node.outputs.iter().zip(&slots) {
            let groups = match (port.card, port.groups) {
                (Card::Grouped, Groups::Own) => own.next().copied(),
                (Card::Grouped, Groups::Like(input)) => self.slot(inputs[input]).groups,
                _ => None,
            };
            let prototype = port.prototype.map(|input| self.shape_of(inputs[input]));
            let total = port.ty.prototype.and_then(|_| totals.next().copied());
            let decl = self.slot_mut(*slot);
            decl.card = match (port.card, groups) {
                (Card::Grouped, None) => Card::Many,
                (card, _) => card,
            };
            decl.groups = groups;
            decl.prototype = prototype;
            decl.total = total;
        }
        Ok(slots[..visible].to_vec())
    }

    /// The shape of instances in `slot`, or an empty one if nothing says
    /// what it is.
    fn prototype_of(&mut self, slot: SlotId) -> Prototype {
        if let Some(prototype) = self.slot(slot).prototype {
            return prototype;
        }
        let ty = self.slot(slot).ty.prototype.expect("only instances have a shape");
        let elements = self.constant_words(*ty, &[]);
        let groups = self.no_groups();
        Prototype { elements, groups }
    }

    /// The total of instances in `slot`, or none at all if nothing says.
    fn total_of(&mut self, slot: SlotId) -> SlotId {
        match self.slot(slot).total {
            Some(total) => total,
            None => self.constant::<u32>(&[0]),
        }
    }

    /// The shape `slot` is or refers to: its own if it is instances, else
    /// the value itself with its groups.
    fn shape_of(&mut self, slot: SlotId) -> Prototype {
        if self.slot(slot).ty.prototype.is_some() {
            return self.prototype_of(slot);
        }
        let groups = match self.slot(slot).groups {
            Some(groups) => groups,
            None => self.no_groups(),
        };
        Prototype { elements: slot, groups }
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

    /// Calls `callee`, with an argument for every parameter which is not
    /// hidden. An argument which is `None` uses the default of the
    /// parameter it stands for.
    pub fn call(
        &mut self,
        programs: &ProgramSet,
        callee: ProgramId,
        args: &[Option<SlotId>],
    ) -> Result<Vec<SlotId>, BuildError> {
        let (inputs, types) = self.arguments(programs, callee, args)?;
        let program = programs.get(callee);
        ("call", program.hash, &inputs).hash(&mut self.hasher);
        let slots = self.push_op(OpKind::Call(callee), inputs, types);
        Ok(self.call_outputs(program, &slots, false))
    }

    /// The inputs of a call, hidden parameters included, and the types of
    /// its outputs: the outputs of the callee, and then the offsets of
    /// every grouped one of them.
    fn arguments(
        &mut self,
        programs: &ProgramSet,
        callee: ProgramId,
        args: &[Option<SlotId>],
    ) -> Result<(Vec<SlotId>, Vec<TypeInfo>), BuildError> {
        let program = programs.get(callee);
        let visible = program.visible_params().count();
        if args.len() != visible {
            return Err(BuildError::Arity {
                target: program.name.clone(),
                expected: visible,
                found: args.len(),
            });
        }

        let mut given = args.iter();
        let mut inputs = vec![None; program.params.len()];
        for (index, param) in program.params.iter().enumerate() {
            if param.hidden {
                continue;
            }
            let ty = program.slot(param.slot).ty;
            let arg = given.next().copied().flatten();
            let slot = match arg {
                Some(slot) => {
                    self.check(&program.name, &param.name, ty, slot)?;
                    if !self.slot(slot).card.fits(program.slot(param.slot).card) {
                        return Err(BuildError::Card {
                            target: program.name.clone(),
                            port: param.name.clone(),
                        });
                    }
                    slot
                }
                None => self.constant_words(ty, &param.default),
            };
            inputs[index] = Some(slot);
            if let Some(hidden) = program.groups_of_param(index) {
                let groups = match self.slot(slot).groups {
                    Some(groups) => groups,
                    None => self.no_groups(),
                };
                inputs[hidden] = Some(groups);
            }
            if let Some((elements, groups)) = program.prototype_of_param(index) {
                let prototype = self.prototype_of(slot);
                inputs[elements] = Some(prototype.elements);
                inputs[groups] = Some(prototype.groups);
            }
            if let Some(total) = program.total_of_param(index) {
                inputs[total] = Some(self.total_of(slot));
            }
        }
        let inputs = inputs
            .into_iter()
            .map(|slot| slot.expect("every hidden parameter belongs to one"))
            .collect();

        let types = program
            .outputs
            .iter()
            .map(|output| program.slot(output.slot).ty)
            .chain(
                program
                    .outputs
                    .iter()
                    .filter(|output| program.slot(output.slot).groups.is_some())
                    .map(|_| u32::INFO),
            )
            .chain(program.outputs.iter().flat_map(|output| {
                let prototype = program.slot(output.slot).ty.prototype;
                prototype.map(|ty| [*ty, u32::INFO, u32::INFO]).into_iter().flatten()
            }))
            .collect();
        Ok((inputs, types))
    }

    /// Hands the groups of the outputs of a call to the slots they are in,
    /// and returns the slots which are not offsets. Every output of a call
    /// for each element is grouped, one group per call.
    fn call_outputs(&mut self, program: &Program, slots: &[SlotId], each: bool) -> Vec<SlotId> {
        let visible = program.outputs.len();
        let mut hidden = slots[visible..].iter();
        for (output, slot) in program.outputs.iter().zip(slots) {
            let grouped = each || program.slot(output.slot).groups.is_some();
            let groups = grouped.then(|| *hidden.next().unwrap());
            let card = program.slot(output.slot).card;
            let decl = self.slot_mut(*slot);
            decl.groups = groups;
            decl.card = match groups {
                Some(_) => Card::Grouped,
                None => card,
            };
        }
        // The shapes of instances come back behind all offsets.
        for (output, slot) in program.outputs.iter().zip(slots) {
            if program.slot(output.slot).ty.prototype.is_some() {
                let elements = *hidden.next().unwrap();
                let groups = *hidden.next().unwrap();
                let total = *hidden.next().unwrap();
                let decl = self.slot_mut(*slot);
                decl.prototype = Some(Prototype { elements, groups });
                decl.total = Some(total);
            }
        }
        slots[..visible].to_vec()
    }

    /// Calls `callee` once for every element of the argument at `element`,
    /// or once for every group if it is grouped, see [`OpKind::ForEach`].
    /// The other arguments are the same for every call. An argument which
    /// is `None` uses the default of the parameter it stands for.
    ///
    /// Every output is grouped: one group for what each call gave.
    pub fn for_each(
        &mut self,
        programs: &ProgramSet,
        callee: ProgramId,
        args: &[Option<SlotId>],
        element: usize,
        max: u32,
    ) -> Result<Vec<SlotId>, BuildError> {
        let program = programs.get(callee);
        let Some(source) = args.get(element).copied().flatten() else {
            return Err(BuildError::Element {
                target: program.name.clone(),
            });
        };
        // What every run gives is put one after the other, which instances
        // can not be: each run could refer to a shape of its own.
        let instances = |slot: SlotId| program.slot(slot).ty.prototype.is_some();
        if program.outputs.iter().any(|output| instances(output.slot)) || self.slot(source).ty.prototype.is_some() {
            return Err(BuildError::Instances {
                target: program.name.clone(),
            });
        }
        let param = program
            .params
            .iter()
            .enumerate()
            .filter(|(_, param)| !param.hidden)
            .nth(element)
            .map(|(index, _)| index)
            .unwrap();

        // Only the element goes in, so a single one can be passed for a
        // parameter which takes a single one.
        let card = self.slot(source).card;
        self.slot_mut(source).card = Card::One;
        let arguments = self.arguments(programs, callee, args);
        self.slot_mut(source).card = card;
        let (mut inputs, mut types) = arguments?;

        // Behind the arguments: what the elements are taken in groups of.
        let groups = match self.slot(source).groups {
            Some(groups) => groups,
            None => self.no_groups(),
        };
        inputs.push(groups);
        // Every output is grouped, so it has offsets whether the callee's
        // was grouped or not.
        types.truncate(program.outputs.len());
        types.extend(program.outputs.iter().map(|_| u32::INFO));

        ("for_each", program.hash, &inputs, param, max).hash(&mut self.hasher);
        let kind = OpKind::ForEach {
            callee,
            element: param as u32,
            max,
        };
        let slots = self.push_op(kind, inputs, types);
        Ok(self.call_outputs(program, &slots, true))
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

    pub fn slot(&self, slot: SlotId) -> &SlotDecl {
        &self.slots[slot.0 as usize]
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
