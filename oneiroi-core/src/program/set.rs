//! Every program of an application, which can only call those added before it.

use crate::program::{OpKind, Program, ProgramId};

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
