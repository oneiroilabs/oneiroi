//! Instances of programs, the values of their parameters, and what can be
//! read of their outputs.

use std::{fmt, sync::{Arc, OnceLock}};

use crate::{
    engine::{Engine, Executor, keys::Keys, link::{Link, Published}},
    program::ProgramId,
    store::ContentKey,
    types::DataType,
};

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
    pub(super) program: ProgramId,
    /// What it was last planned with, for whoever follows it. Only made
    /// once something does, so that the others never look at it.
    pub(super) published: OnceLock<Arc<Published>>,
    /// The values of the parameters are in the store, held there for as
    /// long as this lives.
    pub(super) param_keys: Vec<ContentKey>,
    pub(super) alive: std::sync::Arc<()>,
    /// Derived when it is first planned, and from then on again only where
    /// a parameter changed. Shared with whoever follows it.
    pub(super) keys: Option<Arc<Keys>>,
    /// The parameters which follow an output of another instance.
    pub(super) links: Vec<Link>,
    /// The parameters which changed since the keys were derived.
    pub(super) changed: Vec<usize>,
    /// The outputs it holds in the store, once all of them were there:
    /// until a parameter changes they stay, and planning it again is
    /// skipped altogether. Empty while it is not settled.
    pub(super) held: Vec<ContentKey>,
}

impl Instance {
    pub fn program(&self) -> ProgramId {
        self.program
    }
}

impl<E: Executor> Engine<E> {
    /// A new instance with every parameter at its default.
    pub fn instantiate(&self, program: ProgramId) -> Instance {
        let decl = self.programs.get(program);
        let param_keys = decl
            .params
            .iter()
            .map(|param| ContentKey::of_data(&decl.slot(param.slot).ty, &param.default))
            .collect();
        Instance {
            program,
            published: OnceLock::new(),
            param_keys,
            alive: std::sync::Arc::new(()),
            keys: None,
            links: Vec::new(),
            changed: Vec::new(),
            held: Vec::new(),
        }
    }

    pub fn set_param<T: DataType>(
        &mut self,
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
        &mut self,
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

        // A value of its own instead of what it followed.
        if instance.links.iter().any(|link| link.param == index) {
            self.unlink_params(instance, &[index]);
        }

        // Straight into the store, hashed while it is copied.
        let (key, _) = self.store.insert_data(&ty, words);
        let old = instance.param_keys[index];
        if key != old {
            self.store.release(old, &instance.alive);
            instance.param_keys[index] = key;
            if !instance.changed.contains(&index) {
                instance.changed.push(index);
            }
        }
        self.store.hold(key, &instance.alive);
        Ok(())
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

    /// The keys of the shape an output of instances refers to, and of its
    /// offsets.
    pub fn prototype_keys(&self, instance: &Instance, name: &str) -> Option<(ContentKey, ContentKey)> {
        let program = self.programs.get(instance.program);
        let slot = program.outputs[program.output_index(name)?].slot;
        let prototype = program.slot(slot).prototype?;
        let keys = &instance.keys.as_ref()?.slots;
        Some((keys[prototype.elements.0 as usize], keys[prototype.groups.0 as usize]))
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
}
