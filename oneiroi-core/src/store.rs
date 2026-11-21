//! The content-addressed storage every backend reads from and writes to.
//!
//! All data lives in pages of 32-bit words. A [`Handle`] addresses a run of
//! words inside one page and means the same thing on the CPU (an index into
//! a `Vec<u32>`) and on the GPU (an index into a bound array of buffers).
//!
//! Entries are looked up by [`ContentKey`]. Everything in here can be
//! recomputed from parameters, so the store is free to evict.

mod eviction;
mod pages;
#[cfg(test)]
mod tests;

pub use crate::key::{ContentKey, KeyMap, KeySet};
pub use pages::PAGE_WORDS;

use std::{collections::{BTreeSet, VecDeque}, hash::Hasher, sync::Weak};

use bytemuck::{Pod, Zeroable};

use crate::{key::KeyHasher, store::pages::Page, types::{DataType, TypeInfo}};

/// Where an entry lives: `len` words starting at `offset` in page `page`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Pod, Zeroable)]
pub struct Handle {
    pub page: u32,
    pub offset: u32,
    pub len: u32,
}

struct Entry {
    handle: Handle,
    last_used: u64,
    /// Whether the words in the page are the value. If not, only the
    /// backend which computed it has them so far.
    resident: bool,
    /// The entry this one is a part of. It owns no memory of its own.
    alias_of: Option<ContentKey>,
    /// Who needs the value to stay, like an instance whose parameter it
    /// is: it is never evicted while one of them lives.
    holders: Vec<Weak<()>>,
    /// Whether it was found held while evicting, and is out of the line of
    /// eviction until nobody holds it anymore, see [`Store::held`].
    out_of_line: bool,
    /// How many groups of [`Store::pin`] keep it. A pinned entry is out of
    /// the line of eviction altogether, and not even looked at by it.
    pins: u32,
    /// The entries which are a part of this one, and go with it. Some may
    /// be gone already, or be there twice.
    parts: Vec<ContentKey>,
}

pub struct Store {
    pages: Vec<Page>,
    /// Every free run of every page as `(len, page, offset)`: the smallest
    /// which fits is found at once.
    free_runs: BTreeSet<(u32, u32, u32)>,
    entries: KeyMap<Entry>,
    /// When each entry was used, the oldest first: who is evicted next is
    /// found at the front. An entry is in here once for each frame it was
    /// used in, only the last of which counts.
    used: VecDeque<(u64, ContentKey)>,
    page_words: u32,
    used_words: u64,
    frame: u64,
    /// The groups of [`Store::pin`], each with whoever keeps it.
    pinned: Vec<(Weak<()>, Vec<ContentKey>)>,
    /// The entries out of line while they are held, each with a holder.
    /// Evicting only looks at whether the holders live, not at the
    /// entries. Some are outdated, they are cleared out as it grows.
    held: Vec<(Weak<()>, ContentKey)>,
}

impl Default for Store {
    fn default() -> Self {
        Self::with_page_words(PAGE_WORDS)
    }
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    /// An entry larger than `page_words` gets a page of its own size.
    pub fn with_page_words(page_words: u32) -> Self {
        Self {
            pages: Vec::new(),
            free_runs: BTreeSet::new(),
            entries: KeyMap::default(),
            used: VecDeque::new(),
            page_words,
            used_words: 0,
            frame: 0,
            pinned: Vec::new(),
            held: Vec::new(),
        }
    }

    pub fn contains(&self, key: ContentKey) -> bool {
        self.entries.contains_key(&key)
    }

    pub fn get(&self, key: ContentKey) -> Option<Handle> {
        self.entries.get(&key).map(|entry| entry.handle)
    }

    /// Like [`Store::get`] but also protects the entry from eviction for
    /// the current frame.
    pub fn touch(&mut self, key: ContentKey) -> Option<Handle> {
        let entry = self.entries.get_mut(&key)?;
        let (handle, parent) = (entry.handle, entry.alias_of);
        if entry.last_used != self.frame {
            entry.last_used = self.frame;
            self.mark_used(key);
        }
        // A part keeps what it is a part of.
        if let Some(parent) = parent {
            self.touch(parent);
        }
        Some(handle)
    }

    /// Whether the value of `key` can be read from the store right now.
    /// A value a backend keeps to itself has to be downloaded first.
    pub fn is_resident(&self, key: ContentKey) -> bool {
        self.entries.get(&key).is_some_and(|entry| self.resident(entry))
    }

    fn resident(&self, entry: &Entry) -> bool {
        entry.resident || entry.alias_of.is_some_and(|parent| self.is_resident(parent))
    }

    /// Stores the words `start..start + len` of the value of `parent` under
    /// `key` as well, without copying them. This is how a single element
    /// of a value becomes a value of its own, on every backend at once.
    ///
    /// Returns `None` if `parent` is not here or too short.
    pub fn alias(&mut self, key: ContentKey, parent: ContentKey, start: u32, len: u32) -> Option<Handle> {
        if let Some(handle) = self.touch(key) {
            return Some(handle);
        }
        let whole = self.touch(parent)?;
        if start.checked_add(len)? > whole.len {
            return None;
        }
        let handle = Handle {
            page: whole.page,
            offset: whole.offset + start,
            len,
        };
        self.add(
            key,
            Entry {
                handle,
                last_used: self.frame,
                resident: false,
                alias_of: Some(parent),
                holders: Vec::new(),
                out_of_line: false,
                pins: 0,
                parts: Vec::new(),
            },
        );
        self.add_part(parent, key);
        Some(handle)
    }

    /// Stores `words` under `key`. An existing entry is kept as it is,
    /// since equal keys mean equal content.
    pub fn insert(&mut self, key: ContentKey, words: &[u32]) -> Handle {
        if let Some(entry) = self.entries.get(&key) {
            let (handle, resident) = (entry.handle, entry.resident);
            // A place the backend reserved and then left to somebody else:
            // the backend does not have these words, so they are uploaded.
            if !resident && self.fill(key, words) && !words.is_empty() {
                self.pages[handle.page as usize].mark_dirty(handle.offset, handle.offset + handle.len);
            }
            self.touch(key);
            return handle;
        }

        let handle = self.reserve(key, words.len() as u32);
        let page = &mut self.pages[handle.page as usize];
        page.words.get_mut(handle.offset as usize, words.len()).copy_from_slice(words);
        if !words.is_empty() {
            page.mark_dirty(handle.offset, handle.offset + handle.len);
        }
        self.entries.get_mut(&key).unwrap().resident = true;
        handle
    }

    /// Stores `words` as a value of type `ty` under its
    /// [`ContentKey::of_data`], which is found while the words are copied:
    /// they are read once for both. An existing entry is kept as it is.
    pub fn insert_data(&mut self, ty: &TypeInfo, words: &[u32]) -> (ContentKey, Handle) {
        /// Small enough for what was just copied to be hashed from cache.
        const CHUNK: usize = 1 << 12;

        let handle = self.alloc(words.len() as u32);
        let mut hasher = KeyHasher::new(0);
        hasher.write(ty.name);
        // What hashing a slice writes: its length, and then its bytes.
        hasher.0.write_usize(words.len());
        let target = self.pages[handle.page as usize]
            .words
            .get_mut(handle.offset as usize, words.len());
        for (target, source) in target.chunks_mut(CHUNK).zip(words.chunks(CHUNK)) {
            target.copy_from_slice(source);
            hasher.0.write(bytemuck::cast_slice(source));
        }
        let key = hasher.finish();

        if self.entries.contains_key(&key) {
            // The words were here already, the copy is not needed.
            self.free(handle);
            return (key, self.insert(key, words));
        }
        self.used_words += handle.len as u64;
        if !words.is_empty() {
            self.pages[handle.page as usize].mark_dirty(handle.offset, handle.offset + handle.len);
        }
        self.add(
            key,
            Entry {
                handle,
                last_used: self.frame,
                resident: true,
                alias_of: None,
                holders: Vec::new(),
                out_of_line: false,
                pins: 0,
                parts: Vec::new(),
            },
        );
        (key, handle)
    }

    /// Room for the value of `key`, to be written in place: a CPU kernel
    /// writes its outputs right where they are kept. Nothing else may be
    /// stored under `key` yet. [`Store::commit`] it once it is written.
    pub fn reserve_output(&mut self, key: ContentKey, len: u32) -> Handle {
        self.remove(key);
        let handle = self.alloc(len);
        self.used_words += len as u64;
        self.add(
            key,
            Entry {
                handle,
                last_used: self.frame,
                resident: false,
                alias_of: None,
                holders: Vec::new(),
                out_of_line: false,
                pins: 0,
                parts: Vec::new(),
            },
        );
        handle
    }

    /// The words of a value reserved with [`Store::reserve_output`], to
    /// write it.
    ///
    /// # Safety
    ///
    /// Nothing else may read or write these words while the slice lives,
    /// and nothing may be evicted or removed meanwhile. Other values may
    /// be read meanwhile, see [`Store::words_unbound`].
    pub unsafe fn output_words<'a>(&self, handle: Handle) -> &'a mut [u32] {
        unsafe { self.pages[handle.page as usize].words.range_mut(handle.offset as usize, handle.len as usize) }
    }

    /// The words of a value, not bound to a borrow of the store.
    ///
    /// # Safety
    ///
    /// The value must not be removed, evicted or written while the slice
    /// lives. Other values may be reserved and written meanwhile.
    pub unsafe fn words_unbound<'a>(&self, handle: Handle) -> &'a [u32] {
        let words = self.pages[handle.page as usize].words.get(handle.offset as usize, handle.len as usize);
        unsafe { std::slice::from_raw_parts(words.as_ptr(), words.len()) }
    }

    /// Says that a value written in place is complete: it can be read,
    /// and a backend which mirrors the pages takes it over.
    pub fn commit(&mut self, key: ContentKey) {
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.resident = true;
            let handle = entry.handle;
            if handle.len > 0 {
                self.pages[handle.page as usize].mark_dirty(handle.offset, handle.offset + handle.len);
            }
        }
    }

    /// Takes the value of `key` out, if it is here and nothing is a part of
    /// it.
    fn remove(&mut self, key: ContentKey) {
        self.remove_entry(key);
    }

    /// Takes the entry of `key` out, together with every part of it, and
    /// frees what it owns. Returns how many entries that was.
    fn remove_entry(&mut self, key: ContentKey) -> usize {
        let Some(entry) = self.entries.remove(&key) else {
            return 0;
        };
        if entry.alias_of.is_none() {
            self.free(entry.handle);
            self.used_words -= entry.handle.len as u64;
        }
        let mut removed = 1;
        for part in entry.parts {
            // Only what is still a part of it: a key can come back as
            // something else.
            if self.entries.get(&part).is_some_and(|entry| entry.alias_of == Some(key)) {
                removed += self.remove_entry(part);
            }
        }
        removed
    }

    /// A new entry, used in this frame.
    fn add(&mut self, key: ContentKey, entry: Entry) {
        self.entries.insert(key, entry);
        self.mark_used(key);
    }

    /// Lets `part` go along with `parent`.
    fn add_part(&mut self, parent: ContentKey, part: ContentKey) {
        let Some(entry) = self.entries.get_mut(&parent) else {
            return;
        };
        // Parts which are gone, or there twice, are only cleared out when
        // the list would grow anyway, so it costs as much as it grew.
        if entry.parts.len() == entry.parts.capacity() && !entry.parts.is_empty() {
            let mut parts = std::mem::take(&mut entry.parts);
            parts.sort_unstable();
            parts.dedup();
            parts.retain(|part| self.entries.get(part).is_some_and(|entry| entry.alias_of == Some(parent)));
            self.entries.get_mut(&parent).unwrap().parts = parts;
        }
        self.entries.get_mut(&parent).unwrap().parts.push(part);
    }

    /// Makes room for a value of `len` words which a backend computes and
    /// keeps on its side. The store knows where the value belongs, but not
    /// what it is until it gets [filled in](Store::fill).
    pub fn reserve(&mut self, key: ContentKey, len: u32) -> Handle {
        if let Some(handle) = self.touch(key) {
            return handle;
        }

        let handle = self.alloc(len);
        self.used_words += len as u64;
        self.add(
            key,
            Entry {
                handle,
                last_used: self.frame,
                // Nothing is as good on one side as on the other.
                resident: len == 0,
                alias_of: None,
                holders: Vec::new(),
                out_of_line: false,
                pins: 0,
                parts: Vec::new(),
            },
        );
        handle
    }

    /// [`Store::reserve`] for many values at once, which go one after the
    /// other into as few runs as fit into a page: a run is found once for
    /// all of them instead of once for each. Every piece is freed on its own
    /// later, like any other. Pushes the handle of each onto `handles`, in
    /// their order.
    pub fn reserve_many(&mut self, wanted: &[(ContentKey, u32)], handles: &mut Vec<Handle>) {
        let start = handles.len();
        // What is here already keeps its place, the rest is unset so far.
        let mut missing = 0u64;
        for (key, len) in wanted {
            match self.touch(*key) {
                Some(handle) => handles.push(handle),
                None => {
                    handles.push(Handle { page: u32::MAX, offset: 0, len: *len });
                    missing += *len as u64;
                }
            }
        }
        if missing == 0 {
            // Values of no words, or all of them here.
            for (index, (key, len)) in wanted.iter().enumerate() {
                if handles[start + index].page == u32::MAX {
                    handles[start + index] = self.reserve(*key, *len);
                }
            }
            return;
        }

        let mut index = 0;
        while index < wanted.len() {
            // As many of the missing ones from here as fit into a page; one
            // longer than a page goes on its own.
            let mut end = index;
            let mut run = 0u32;
            while end < wanted.len() {
                let handle = handles[start + end];
                let len = if handle.page == u32::MAX { handle.len } else { 0 };
                if run > 0 && run as u64 + len as u64 > self.page_words as u64 {
                    break;
                }
                run += len;
                end += 1;
            }
            let room = self.alloc(run);
            self.used_words += run as u64;
            let mut offset = room.offset;
            for at in index..end {
                if handles[start + at].page != u32::MAX {
                    continue;
                }
                let (key, len) = wanted[at];
                let handle = Handle { page: room.page, offset, len };
                offset += len;
                // Wanted twice: the first one has it.
                if let Some(entry) = self.entries.get(&key) {
                    handles[start + at] = entry.handle;
                    self.free(handle);
                    self.used_words -= len as u64;
                    continue;
                }
                handles[start + at] = handle;
                self.add(
                    key,
                    Entry {
                        handle,
                        last_used: self.frame,
                        resident: len == 0,
                        alias_of: None,
                        holders: Vec::new(),
                        out_of_line: false,
                        pins: 0,
                        parts: Vec::new(),
                    },
                );
            }
            index = end;
        }
    }

    /// Writes the value of a reserved entry which was brought over from
    /// the backend. Returns whether there was such an entry.
    pub fn fill(&mut self, key: ContentKey, words: &[u32]) -> bool {
        let Some(entry) = self.entries.get_mut(&key) else {
            return false;
        };
        if entry.handle.len as usize != words.len() {
            return false;
        }
        entry.resident = true;
        let handle = entry.handle;
        // The backend has these words already, so they are not dirty.
        self.pages[handle.page as usize]
            .words
            .get_mut(handle.offset as usize, words.len())
            .copy_from_slice(words);
        true
    }

    pub fn words(&self, handle: Handle) -> &[u32] {
        self.pages[handle.page as usize].words.get(handle.offset as usize, handle.len as usize)
    }

    pub fn slice<T: DataType>(&self, handle: Handle) -> &[T] {
        debug_assert_eq!(handle.len % T::INFO.words, 0);
        bytemuck::cast_slice(self.words(handle))
    }

    /// The value of `key`, if it is here and not only on the backend.
    pub fn read<T: DataType>(&self, key: ContentKey) -> Option<&[T]> {
        let entry = self.entries.get(&key).filter(|entry| self.resident(entry))?;
        Some(self.slice(entry.handle))
    }

    pub fn used_words(&self) -> u64 {
        self.used_words
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
