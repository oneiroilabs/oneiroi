//! What a CPU kernel sees of a call: its inputs, and the outputs it writes.

use crate::{
    node::{NodeDef, PortDecl, hidden_input, hidden_output, hidden_prototype, hidden_total},
    store::{ContentKey, Handle, Store},
    types::{DataType, InstanceTotal},
};

/// The inputs handed to a [`CpuKernel`], in the order of [`NodeDef::inputs`].
///
/// Grouped inputs come with their offsets: every group starts at an index
/// of the elements, and the last offset is where the last group ends.
/// No offsets at all means a single group of everything.
pub struct Inputs<'a> {
    pub(crate) ports: &'static [PortDecl],
    pub(crate) handles: &'a [Handle],
    /// The words of every handle.
    pub(crate) words: Words<'a>,
}

/// The words of the inputs of a call. Inline for as many inputs as nodes
/// have, so that a call allocates nothing for them.
pub(crate) enum Words<'a> {
    Inline { len: usize, words: [&'a [u32]; INLINE_INPUTS] },
    Heap(Vec<&'a [u32]>),
}

/// How many inputs [`Words`] keeps without allocating.
const INLINE_INPUTS: usize = 12;

impl<'a> Words<'a> {
    fn of(words: impl ExactSizeIterator<Item = &'a [u32]>) -> Self {
        if words.len() > INLINE_INPUTS {
            return Words::Heap(words.collect());
        }
        let mut inline: [&'a [u32]; INLINE_INPUTS] = [&[]; INLINE_INPUTS];
        let len = words.len();
        for (slot, words) in inline.iter_mut().zip(words) {
            *slot = words;
        }
        Words::Inline { len, words: inline }
    }

    fn get(&self, index: usize) -> &'a [u32] {
        match self {
            Words::Inline { len, words } => {
                assert!(index < *len, "there is no input {index}");
                words[index]
            }
            Words::Heap(words) => words[index],
        }
    }
}

impl<'a> Inputs<'a> {
    pub fn new(store: &'a Store, node: &NodeDef, handles: &'a [Handle]) -> Self {
        Self {
            ports: node.inputs,
            handles,
            words: Words::of(handles.iter().map(|handle| store.words(*handle))),
        }
    }

    /// [`Inputs::new`] which does not keep the store borrowed, so that the
    /// outputs can be written into it meanwhile.
    ///
    /// # Safety
    ///
    /// The values of `handles` must stay in the store, unchanged, for as
    /// long as the inputs live: see [`Store::words_unbound`].
    pub(crate) unsafe fn unbound(store: &Store, ports: &'static [PortDecl], handles: &'a [Handle]) -> Self {
        Self {
            ports,
            handles,
            words: Words::of(handles.iter().map(|handle| unsafe { store.words_unbound(*handle) })),
        }
    }

    fn slice<T: DataType>(&self, index: usize) -> &'a [T] {
        bytemuck::cast_slice(self.words.get(index))
    }

    /// How many elements an input has. Unlike [`Inputs::get`] this never
    /// looks at the values.
    pub fn len(&self, index: usize) -> usize {
        (self.handles[index].len / self.ports[index].ty.words) as usize
    }

    pub fn get<T: DataType>(&self, index: usize) -> &'a [T] {
        debug_assert!(self.ports[index].ty.stored_as(&T::INFO), "{}", self.ports[index].name);
        self.slice(index)
    }

    /// The first element of an input, for ports which carry a single value.
    pub fn value<T: DataType>(&self, index: usize) -> T {
        self.get::<T>(index)
            .first()
            .copied()
            .unwrap_or_else(T::zeroed)
    }

    /// Which of the [choices](PortDecl::choices) of input `index` is
    /// picked. A number past the last one is the last one.
    pub fn choice(&self, index: usize) -> usize {
        let last = self.ports[index].choices.len().saturating_sub(1);
        (self.value::<u32>(index) as usize).min(last)
    }

    /// The offsets of a grouped input, if it has groups.
    pub fn offsets(&self, index: usize) -> Option<&'a [u32]> {
        let hidden = hidden_input(self.ports, index)?;
        let offsets: &[u32] = self.slice(hidden);
        (!offsets.is_empty()).then_some(offsets)
    }

    /// The groups of an input, as ranges of its elements. An input without
    /// groups is a single one.
    pub fn groups(&self, index: usize) -> Vec<std::ops::Range<usize>> {
        let len = self.len(index);
        match self.offsets(index) {
            Some(offsets) => offsets
                .windows(2)
                .map(|pair| (pair[0] as usize).min(len)..(pair[1] as usize).min(len))
                .collect(),
            None => vec![0..len],
        }
    }

    /// The shape an input of instances refers to.
    pub fn prototype<T: DataType>(&self, index: usize) -> &'a [T] {
        let hidden = hidden_prototype(self.ports, index).expect("only instances have a shape");
        self.slice(hidden)
    }

    /// How many elements the shape of an input of instances has.
    pub fn prototype_len(&self, index: usize) -> usize {
        let hidden = hidden_prototype(self.ports, index).expect("only instances have a shape");
        let prototype = self.ports[index].ty.prototype.expect("only instances have a shape");
        (self.handles[hidden].len / prototype.words) as usize
    }

    /// The groups of the shape an input of instances refers to.
    pub fn prototype_groups(&self, index: usize) -> Vec<std::ops::Range<usize>> {
        let hidden = hidden_prototype(self.ports, index).expect("only instances have a shape");
        let len = self.prototype_len(index);
        let offsets: &[u32] = self.slice(hidden + 1);
        match offsets.is_empty() {
            true => vec![0..len],
            false => offsets
                .windows(2)
                .map(|pair| (pair[0] as usize).min(len)..(pair[1] as usize).min(len))
                .collect(),
        }
    }

    /// How many elements all copies of an input of instances have together.
    /// Its producer says so without looking at any rows, so it is known on
    /// the CPU while the rows may be on the GPU only.
    pub fn total(&self, index: usize) -> InstanceTotal {
        let hidden = hidden_prototype(self.ports, index).expect("only instances have a total");
        InstanceTotal::of_words(self.slice(hidden + 2))
    }

    /// How many groups an input has, without looking at its offsets: they
    /// may be on the GPU only. The same as the length of
    /// [`Inputs::groups`].
    pub fn group_count(&self, index: usize) -> usize {
        let Some(hidden) = hidden_input(self.ports, index) else {
            return 1;
        };
        match self.handles[hidden].len as usize {
            0 => 1,
            offsets => offsets - 1,
        }
    }
}

/// The outputs a [`CpuKernel`] fills, in the order of [`NodeDef::outputs`].
/// An output which is not written stays empty.
///
/// They are written right into the store, where they are kept: nothing is
/// copied once the kernel is done.
pub struct Outputs<'a> {
    pub(crate) ports: &'static [PortDecl],
    store: &'a mut Store,
    /// Under which key each output and everything behind it goes.
    keys: &'a [ContentKey],
    written: Vec<bool>,
}

impl<'a> Outputs<'a> {
    pub(crate) fn new(store: &'a mut Store, ports: &'static [PortDecl], keys: &'a [ContentKey]) -> Self {
        Self {
            ports,
            store,
            keys,
            written: vec![false; keys.len()],
        }
    }

    /// Room for `words` words of output `index` (or of what comes along
    /// with the outputs), as it is: not zeroed.
    fn words(&mut self, index: usize, words: usize) -> &'a mut [u32] {
        assert!(!self.written[index], "`{}` is written once", self.ports.get(index).map_or("a hidden output", |port| port.name));
        let handle = self.store.reserve_output(self.keys[index], words as u32);
        self.written[index] = true;
        // SAFETY: the room is new and only this slice reaches it, since
        // every output is reserved once. Nothing is evicted or removed
        // while a kernel runs, and a kernel can not keep the slice past
        // its run, so it is gone before the output is committed.
        unsafe { self.store.output_words(handle) }
    }

    pub fn set<T: DataType>(&mut self, index: usize, values: &[T]) {
        debug_assert!(self.ports[index].ty.stored_as(&T::INFO), "{}", self.ports[index].name);
        let words: &[u32] = bytemuck::cast_slice(values);
        self.words(index, words.len()).copy_from_slice(words);
    }

    /// Zeroed space for `len` elements to write the output in place.
    /// Every output can be had at the same time.
    pub fn alloc<T: DataType>(&mut self, index: usize, len: usize) -> &'a mut [T] {
        let values = self.alloc_unset::<T>(index, len);
        bytemuck::cast_slice_mut::<T, u32>(values).fill(0);
        values
    }

    /// [`Outputs::alloc`] without the zeroing, for a kernel which writes
    /// every element: what is in it at first is left over from before.
    pub fn alloc_unset<T: DataType>(&mut self, index: usize, len: usize) -> &'a mut [T] {
        debug_assert!(self.ports[index].ty.stored_as(&T::INFO), "{}", self.ports[index].name);
        bytemuck::cast_slice_mut(self.words(index, len * T::INFO.words as usize))
    }

    /// How many elements all copies of an output of instances have
    /// together, see [`Inputs::total`].
    pub fn set_total(&mut self, index: usize, total: InstanceTotal) {
        let hidden = hidden_total(self.ports, index).expect("only instances have a total");
        self.words(hidden, 3).copy_from_slice(&total.words());
    }

    /// Where the groups of a grouped output start, and where the last one
    /// ends. Leaving them out makes a single group of everything.
    ///
    /// An output which is not grouped is a single shape, and has no offsets
    /// to set: they are left out. One kernel can serve a node which makes
    /// one and a node which makes many that way.
    pub fn set_groups(&mut self, index: usize, offsets: &[u32]) {
        if let Some(hidden) = hidden_output(self.ports, index) {
            self.words(hidden, offsets.len()).copy_from_slice(offsets);
        }
    }

    /// Makes everything written readable, and leaves what was not written
    /// empty.
    pub(crate) fn finish(self) {
        for (key, written) in self.keys.iter().zip(self.written) {
            match written {
                true => self.store.commit(*key),
                false => {
                    self.store.insert(*key, &[]);
                }
            }
        }
    }
}
