//! Parameters of one instance which follow an output of another, without
//! the value being copied.

use std::{fmt, sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}}};

use crate::{
    engine::{Engine, Executor, Instance, keys::Keys},
    program::{ProgramId, SlotId},
    store::ContentKey,
};

/// What parameter `param` follows, if one of `links` is for it and gave it
/// `key`: the program of the instance it follows, the keys and the links
/// that one was last planned with, and the slot it follows of it.
pub(super) fn followed(links: &[Link], param: usize, key: ContentKey) -> Option<(ProgramId, Arc<Keys>, Vec<Link>, SlotId)> {
    let link = links.iter().find(|link| link.param == param)?;
    let snapshot = link.source.snapshot.lock().unwrap();
    let (program, keys) = (snapshot.program?, snapshot.keys.clone()?);
    (keys.slots[link.slot.0 as usize] == key).then(|| (program, keys, snapshot.links.clone(), link.slot))
}

/// Whether `instance` is `other`, or follows it, directly or through others.
fn follows(instance: &Arc<Published>, other: &Arc<Published>) -> bool {
    let mut next = vec![instance.clone()];
    let mut seen: Vec<*const Published> = Vec::new();
    while let Some(at) = next.pop() {
        if Arc::ptr_eq(&at, other) {
            return true;
        }
        if seen.contains(&Arc::as_ptr(&at)) {
            continue;
        }
        seen.push(Arc::as_ptr(&at));
        next.extend(at.snapshot.lock().unwrap().links.iter().map(|link| link.source.clone()));
    }
    false
}

/// Why a parameter can not follow an output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkError {
    UnknownParam(String),
    UnknownOutput(String),
    Type {
        param: String,
        expected: &'static str,
        found: &'static str,
    },
    /// What comes along with the parameter, like the shape instances refer
    /// to, does not come along with the output.
    Along { param: String, output: String },
    /// The output depends on the parameter, through other links.
    Cycle,
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownParam(param) => write!(f, "there is no parameter `{param}`"),
            Self::UnknownOutput(output) => write!(f, "there is no output `{output}`"),
            Self::Type {
                param,
                expected,
                found,
            } => write!(f, "`{param}` expects `{expected}` but the output is `{found}`"),
            Self::Along { param, output } => {
                write!(f, "what `{param}` needs along with it does not come with `{output}`")
            }
            Self::Cycle => write!(f, "the output depends on the parameter"),
        }
    }
}

impl std::error::Error for LinkError {}

/// A parameter of an instance which follows a slot of another instance.
#[derive(Clone)]
pub(super) struct Link {
    pub(super) param: usize,
    /// What the instance it follows was last planned with.
    pub(super) source: Arc<Published>,
    /// The slot it follows, in the program of that instance.
    slot: SlotId,
    /// The version of `source` it was set from last.
    seen: u64,
}

/// What an instance was last planned with, shared with whoever follows it:
/// enough to plan what they follow of it even when it is not planned
/// itself, or gone. Whoever follows it only looks inside when its version
/// changed.
#[derive(Default)]
pub(super) struct Published {
    /// Counts up every time it is planned with other keys.
    version: AtomicU64,
    /// The last plan it was given to, see [`Engine::plans`].
    pub(super) listed: AtomicU64,
    /// The last plan it was planned in.
    pub(super) planned: AtomicU64,
    snapshot: Mutex<Snapshot>,
}

#[derive(Default)]
struct Snapshot {
    /// `None` until it is planned for the first time.
    program: Option<ProgramId>,
    keys: Option<Arc<Keys>>,
    /// The parameters it follows itself: a chain of them is followed to
    /// its start, and never goes round in a circle.
    links: Vec<Link>,
    /// Holds its parameters, for as long as somebody follows it.
    holder: Arc<()>,
    /// The parameters it holds.
    params: Vec<ContentKey>,
}

impl<E: Executor> Engine<E> {
    /// Sets the parameters of `instance` which follow others to what they
    /// follow is now. Only looks at those which were planned anew since.
    pub(super) fn follow(&mut self, instance: &mut Instance) {
        for link in &mut instance.links {
            let version = link.source.version.load(Ordering::Acquire);
            if version == link.seen {
                continue;
            }
            let snapshot = link.source.snapshot.lock().unwrap();
            let Some(keys) = &snapshot.keys else {
                continue;
            };
            link.seen = version;
            let key = keys.slots[link.slot.0 as usize];
            if instance.param_keys[link.param] != key {
                self.store.release(instance.param_keys[link.param], &instance.alive);
                instance.param_keys[link.param] = key;
                if !instance.changed.contains(&link.param) {
                    instance.changed.push(link.param);
                }
            }
        }
    }

    /// What `instance` was planned with, for whoever follows it. Its
    /// parameters are held for them, so that what they follow can be made
    /// again even once it is gone.
    pub(super) fn publish(&mut self, instance: &Instance) {
        // Nobody follows it.
        let Some(published) = instance.published.get().filter(|published| Arc::strong_count(published) > 1) else {
            return;
        };
        let mut snapshot = published.snapshot.lock().unwrap();
        let same = match (&snapshot.keys, &instance.keys) {
            (Some(published), Some(keys)) => Arc::ptr_eq(published, keys),
            _ => false,
        };
        if same {
            return;
        }
        snapshot.program = Some(instance.program);
        snapshot.keys = instance.keys.clone();
        if snapshot.params != instance.param_keys {
            for key in &instance.param_keys {
                self.store.hold(*key, &snapshot.holder);
            }
            for key in &snapshot.params {
                if !instance.param_keys.contains(key) {
                    self.store.release(*key, &snapshot.holder);
                }
            }
            snapshot.params.clone_from(&instance.param_keys);
        }
        published.version.fetch_add(1, Ordering::Release);
    }

    /// Lets parameter `param` of `target` follow output `output` of
    /// `source`: whenever that changes, so does the parameter, without the
    /// value being copied. Planned in the same [`Engine::plan`], `source`
    /// goes first. Planned on its own, `target` follows what `source` was
    /// planned with last, even once it is gone.
    ///
    /// What comes along with the parameter, like its groups, follows what
    /// comes along with the output. An output without groups is a single
    /// group for a parameter which has them. Setting the parameter, or
    /// [`Engine::unlink`], ends it.
    pub fn link(
        &mut self,
        target: &mut Instance,
        param: &str,
        source: &Instance,
        output: &str,
    ) -> Result<(), LinkError> {
        let program = self.programs.get(target.program);
        let index = program
            .param_index(param)
            .ok_or_else(|| LinkError::UnknownParam(param.into()))?;
        let from = self.programs.get(source.program);
        let output_index = from
            .output_index(output)
            .ok_or_else(|| LinkError::UnknownOutput(output.into()))?;
        let slot = from.outputs[output_index].slot;
        let (expected, found) = (program.slot(program.params[index].slot).ty, from.slot(slot).ty);
        if expected != found {
            return Err(LinkError::Type {
                param: param.into(),
                expected: expected.name,
                found: found.name,
            });
        }
        let (followed, following) = (
            source.published.get_or_init(Arc::default).clone(),
            target.published.get_or_init(Arc::default).clone(),
        );
        if follows(&followed, &following) {
            return Err(LinkError::Cycle);
        }

        // The parameter and what comes along with it.
        let along = LinkError::Along {
            param: param.into(),
            output: output.into(),
        };
        let decl = from.slot(slot);
        let mut links = vec![(index, Some(slot))];
        if let Some(groups) = program.groups_of_param(index) {
            // No groups is a single group of everything.
            links.push((groups, decl.groups));
        }
        if let Some((elements, groups)) = program.prototype_of_param(index) {
            let prototype = decl.prototype.ok_or(along.clone())?;
            links.push((elements, Some(prototype.elements)));
            links.push((groups, Some(prototype.groups)));
        }
        if let Some(total) = program.total_of_param(index) {
            links.push((total, Some(decl.total.ok_or(along)?)));
        }

        let params: Vec<usize> = links.iter().map(|(param, _)| *param).collect();
        self.unlink_params(target, &params);
        for (param, slot) in links {
            match slot {
                Some(slot) => target.links.push(Link {
                    param,
                    source: followed.clone(),
                    slot,
                    // Seen nothing of it yet.
                    seen: u64::MAX,
                }),
                None => {
                    let name = self.programs.get(target.program).params[param].name.clone();
                    self.set_param_words(target, &name, &[]).expect("no groups are whole words");
                }
            }
        }
        following.snapshot.lock().unwrap().links.clone_from(&target.links);
        if source.keys.is_some() {
            self.publish(source);
        }
        self.follow(target);
        Ok(())
    }

    /// Ends what [`Engine::link`] began for `param`: it keeps the value it
    /// has now, until it is set.
    pub fn unlink(&mut self, target: &mut Instance, param: &str) {
        let program = self.programs.get(target.program);
        let Some(index) = program.param_index(param) else {
            return;
        };
        let prototype = program.prototype_of_param(index).map(|(elements, groups)| [elements, groups]);
        let params: Vec<usize> = [index]
            .into_iter()
            .chain(program.groups_of_param(index))
            .chain(prototype.into_iter().flatten())
            .chain(program.total_of_param(index))
            .collect();
        self.unlink_params(target, &params);
    }

    pub(super) fn unlink_params(&mut self, target: &mut Instance, params: &[usize]) {
        if !target.links.iter().any(|link| params.contains(&link.param)) {
            return;
        }
        for link in target.links.iter().filter(|link| params.contains(&link.param)) {
            // The value stays, so it is held like one which was set.
            self.store.hold(target.param_keys[link.param], &target.alive);
        }
        target.links.retain(|link| !params.contains(&link.param));
        if let Some(published) = target.published.get() {
            published.snapshot.lock().unwrap().links.clone_from(&target.links);
        }
    }
}
