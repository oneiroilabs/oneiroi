//! The content-addressed storage every backend reads from and writes to.
//!
//! All data lives in pages of 32-bit words. A [`Handle`] addresses a run of
//! words inside one page and means the same thing on the CPU (an index into
//! a `Vec<u32>`) and on the GPU (an index into a bound array of buffers).
//!
//! Entries are looked up by [`ContentKey`]. Everything in here can be
//! recomputed from parameters, so the store is free to evict.

use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use bytemuck::{Pod, Zeroable};

use crate::types::{DataType, TypeInfo};

/// Default page size: 1 MiB.
pub const PAGE_WORDS: u32 = 1 << 18;

/// Identifies a value by how it was made instead of where it lives.
///
/// Two values with the same key are interchangeable, which is what lets
/// instances with equal parameters share their results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentKey(u128);

impl ContentKey {
    /// Stands in for a key which is not derived yet.
    pub(crate) const UNSET: Self = Self(0);

    /// The key of a leaf value: it is identified by its type and its bytes.
    pub fn of_data(ty: &TypeInfo, words: &[u32]) -> Self {
        let mut hasher = KeyHasher::new(0);
        hasher.write(ty.name);
        hasher.write(words);
        hasher.finish()
    }

    /// The key of the `output`-th result of running `node` on `inputs`.
    pub fn derive(node: &str, output: u32, inputs: &[ContentKey]) -> Self {
        let mut hasher = KeyHasher::new(1);
        hasher.write(node);
        hasher.write(output);
        hasher.write(inputs);
        hasher.finish()
    }
}

struct KeyHasher(DefaultHasher, DefaultHasher);

impl KeyHasher {
    fn new(domain: u8) -> Self {
        let mut hasher = Self(DefaultHasher::new(), DefaultHasher::new());
        // Decorrelates the two halves of the key.
        hasher.1.write_u8(0xA5);
        hasher.write(domain);
        hasher
    }

    fn write(&mut self, value: impl Hash) {
        value.hash(&mut self.0);
        value.hash(&mut self.1);
    }

    fn finish(self) -> ContentKey {
        ContentKey((self.0.finish() as u128) << 64 | self.1.finish() as u128)
    }
}

/// Where an entry lives: `len` words starting at `offset` in page `page`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Pod, Zeroable)]
pub struct Handle {
    pub page: u32,
    pub offset: u32,
    pub len: u32,
}

struct Page {
    words: Vec<u32>,
    /// Free `(offset, len)` runs, sorted by offset and never adjacent.
    free: Vec<(u32, u32)>,
    /// The runs of words written since somebody last asked, as
    /// `(start, end)`, sorted and never touching each other.
    dirty: Vec<(u32, u32)>,
}

impl Page {
    fn new(words: u32) -> Self {
        Self {
            words: vec![0; words as usize],
            free: vec![(0, words)],
            dirty: Vec::new(),
        }
    }

    fn alloc(&mut self, len: u32) -> Option<u32> {
        let index = self.free.iter().position(|&(_, free)| free >= len)?;
        let (offset, free) = self.free[index];
        if free == len {
            self.free.remove(index);
        } else {
            self.free[index] = (offset + len, free - len);
        }
        Some(offset)
    }

    fn mark_dirty(&mut self, start: u32, end: u32) {
        let (mut start, mut end) = (start, end);
        self.dirty.retain(|&(from, to)| {
            let apart = to < start || end < from;
            if !apart {
                start = start.min(from);
                end = end.max(to);
            }
            apart
        });
        let index = self.dirty.partition_point(|&(from, _)| from < start);
        self.dirty.insert(index, (start, end));
    }

    fn free(&mut self, offset: u32, len: u32) {
        let index = self.free.partition_point(|&(start, _)| start < offset);
        self.free.insert(index, (offset, len));
        if index + 1 < self.free.len() && offset + len == self.free[index + 1].0 {
            self.free[index].1 += self.free[index + 1].1;
            self.free.remove(index + 1);
        }
        if index > 0 && self.free[index - 1].0 + self.free[index - 1].1 == offset {
            self.free[index - 1].1 += self.free[index].1;
            self.free.remove(index);
        }
    }
}

struct Entry {
    handle: Handle,
    last_used: u64,
    /// Whether the words in the page are the value. If not, only the
    /// backend which computed it has them so far.
    resident: bool,
    /// The entry this one is a part of. It owns no memory of its own.
    alias_of: Option<ContentKey>,
}

pub struct Store {
    pages: Vec<Page>,
    entries: HashMap<ContentKey, Entry>,
    page_words: u32,
    used_words: u64,
    frame: u64,
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
            entries: HashMap::new(),
            page_words,
            used_words: 0,
            frame: 0,
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
        entry.last_used = self.frame;
        let (handle, parent) = (entry.handle, entry.alias_of);
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
        self.entries.insert(
            key,
            Entry {
                handle,
                last_used: self.frame,
                resident: false,
                alias_of: Some(parent),
            },
        );
        Some(handle)
    }

    /// Stores `words` under `key`. An existing entry is kept as it is,
    /// since equal keys mean equal content.
    pub fn insert(&mut self, key: ContentKey, words: &[u32]) -> Handle {
        if let Some(entry) = self.entries.get(&key) {
            let (handle, resident) = (entry.handle, entry.resident);
            if !resident {
                self.fill(key, words);
            }
            self.touch(key);
            return handle;
        }

        let handle = self.reserve(key, words.len() as u32);
        let page = &mut self.pages[handle.page as usize];
        page.words[handle.offset as usize..][..words.len()].copy_from_slice(words);
        if !words.is_empty() {
            page.mark_dirty(handle.offset, handle.offset + handle.len);
        }
        self.entries.get_mut(&key).unwrap().resident = true;
        handle
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
        self.entries.insert(
            key,
            Entry {
                handle,
                last_used: self.frame,
                // Nothing is as good on one side as on the other.
                resident: len == 0,
                alias_of: None,
            },
        );
        handle
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
        self.pages[handle.page as usize].words[handle.offset as usize..][..words.len()]
            .copy_from_slice(words);
        true
    }

    fn alloc(&mut self, len: u32) -> Handle {
        for (page, candidate) in self.pages.iter_mut().enumerate() {
            if let Some(offset) = candidate.alloc(len) {
                return Handle {
                    page: page as u32,
                    offset,
                    len,
                };
            }
        }

        let mut page = Page::new(self.page_words.max(len));
        let offset = page.alloc(len).unwrap();
        self.pages.push(page);
        Handle {
            page: self.pages.len() as u32 - 1,
            offset,
            len,
        }
    }

    pub fn words(&self, handle: Handle) -> &[u32] {
        &self.pages[handle.page as usize].words[handle.offset as usize..][..handle.len as usize]
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

    /// Frees the least recently used entries until at most `budget_words`
    /// are in use. Entries touched in the current frame are never freed.
    /// Returns how many entries were evicted.
    pub fn evict(&mut self, budget_words: u64) -> usize {
        if self.used_words <= budget_words {
            return 0;
        }

        let mut candidates: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.last_used < self.frame)
            .map(|(key, entry)| (entry.last_used, *key))
            .collect();
        candidates.sort_unstable();

        let mut evicted = 0;
        for (_, key) in candidates {
            if self.used_words <= budget_words {
                break;
            }
            let Some(entry) = self.entries.remove(&key) else {
                // A part which went along with what it is a part of.
                continue;
            };
            evicted += 1;
            if entry.alias_of.is_some() {
                continue;
            }
            let handle = entry.handle;
            self.pages[handle.page as usize].free(handle.offset, handle.len);
            self.used_words -= handle.len as u64;
            // Its parts point at memory which is free now.
            let before = self.entries.len();
            self.entries.retain(|_, entry| entry.alias_of != Some(key));
            evicted += before - self.entries.len();
        }
        evicted
    }

    pub fn end_frame(&mut self) {
        self.frame += 1;
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

    /// The part of every page which was written since the last call, as
    /// `(page, start..end)` in words. A backend that mirrors the pages
    /// uploads exactly this.
    pub fn take_dirty(&mut self) -> Vec<(u32, std::ops::Range<u32>)> {
        self.pages
            .iter_mut()
            .enumerate()
            .flat_map(|(index, page)| {
                std::mem::take(&mut page.dirty)
                    .into_iter()
                    .map(move |(start, end)| (index as u32, start..end))
            })
            .collect()
    }

    /// The raw pages, in the order a GPU backend binds them.
    pub fn pages(&self) -> impl ExactSizeIterator<Item = &[u32]> {
        self.pages.iter().map(|page| page.words.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(n: u32) -> ContentKey {
        ContentKey::of_data(&u32::INFO, &[n])
    }

    #[test]
    fn keys_separate_type_content_and_derivation() {
        assert_eq!(key(1), key(1));
        assert_ne!(key(1), key(2));
        assert_ne!(
            ContentKey::of_data(&u32::INFO, &[0]),
            ContentKey::of_data(&f32::INFO, &[0])
        );
        assert_ne!(
            ContentKey::derive("a", 0, &[key(1)]),
            ContentKey::derive("a", 1, &[key(1)])
        );
        assert_ne!(
            ContentKey::derive("a", 0, &[key(1)]),
            ContentKey::derive("b", 0, &[key(1)])
        );
    }

    #[test]
    fn insert_is_idempotent_and_readable() {
        let mut store = Store::with_page_words(8);
        let a = store.insert(key(1), &[1, 2, 3]);
        let b = store.insert(key(1), &[9, 9, 9]);
        assert_eq!(a, b);
        assert_eq!(store.read::<u32>(key(1)), Some(&[1, 2, 3][..]));
        assert_eq!(store.used_words(), 3);
    }

    #[test]
    fn large_entries_get_their_own_page() {
        let mut store = Store::with_page_words(4);
        store.insert(key(1), &[0; 3]);
        let big = store.insert(key(2), &[7; 10]);
        assert_eq!(big.page, 1);
        assert_eq!(store.words(big), &[7; 10]);
        assert_eq!(store.pages().len(), 2);
    }

    #[test]
    fn eviction_is_lru_and_spares_the_current_frame() {
        let mut store = Store::with_page_words(8);
        store.insert(key(1), &[1; 4]);
        store.end_frame();
        store.insert(key(2), &[2; 4]);
        store.end_frame();
        store.touch(key(1));

        // key(1) was touched this frame, so only key(2) may go.
        assert_eq!(store.evict(0), 1);
        assert!(store.contains(key(1)) && !store.contains(key(2)));

        // The freed run is reused instead of growing a new page.
        let reused = store.insert(key(3), &[3; 4]);
        assert_eq!(reused.page, 0);
        assert_eq!(store.pages().len(), 1);
    }

    #[test]
    fn writes_are_reported_once() {
        let mut store = Store::with_page_words(8);
        store.insert(key(1), &[1; 2]);
        store.insert(key(2), &[2; 3]);
        store.insert(key(3), &[]);
        assert_eq!(store.take_dirty(), [(0, 0..5)]);
        assert_eq!(store.take_dirty(), []);

        // Storing what is there already writes nothing.
        store.insert(key(1), &[1; 2]);
        store.insert(key(4), &[4; 10]);
        assert_eq!(store.take_dirty(), [(1, 0..10)]);
    }

    #[test]
    fn values_can_live_on_the_backend_only() {
        let mut store = Store::with_page_words(16);
        store.insert(key(1), &[1; 2]);
        let remote = store.reserve(key(2), 3);
        store.insert(key(3), &[3; 2]);

        // It has its place, but nothing to read and nothing to upload:
        // the words around it must not overwrite what the backend put there.
        assert_eq!((remote.offset, remote.len), (2, 3));
        assert!(store.contains(key(2)) && !store.is_resident(key(2)));
        assert_eq!(store.read::<u32>(key(2)), None);
        assert_eq!(store.take_dirty(), [(0, 0..2), (0, 5..7)]);

        assert!(store.fill(key(2), &[2; 3]));
        assert_eq!(store.read::<u32>(key(2)), Some(&[2; 3][..]));
        assert_eq!(store.take_dirty(), []);

        // Only what was reserved can be filled, and only all of it.
        assert!(!store.fill(key(9), &[0]));
        assert!(!store.fill(key(1), &[0; 5]));
        // An empty value is the same everywhere.
        store.reserve(key(4), 0);
        assert!(store.is_resident(key(4)));
    }

    #[test]
    fn a_part_of_a_value_is_a_value_of_its_own() {
        let mut store = Store::with_page_words(16);
        store.insert(key(1), &[1, 2, 3, 4, 5, 6]);
        let part = store.alias(key(2), key(1), 2, 2).unwrap();
        assert_eq!((part.offset, part.len), (2, 2));
        assert_eq!(store.read::<u32>(key(2)), Some(&[3, 4][..]));
        // It costs nothing and is not written anywhere.
        assert_eq!(store.used_words(), 6);
        assert_eq!(store.take_dirty(), [(0, 0..6)]);

        // Beyond the end of the value there is nothing to be a part of.
        assert!(store.alias(key(3), key(1), 5, 2).is_none());
        assert!(store.alias(key(3), key(9), 0, 1).is_none());

        // A part of what only the backend has is only on the backend too,
        // until the whole is brought over.
        store.reserve(key(4), 4);
        store.alias(key(5), key(4), 1, 1).unwrap();
        assert!(!store.is_resident(key(5)));
        store.fill(key(4), &[7, 8, 9, 10]);
        assert_eq!(store.read::<u32>(key(5)), Some(&[8][..]));

        // Using a part keeps the whole, and the parts go with the whole.
        store.end_frame();
        store.touch(key(2));
        store.evict(0);
        assert!(store.contains(key(1)) && store.contains(key(2)));
        assert!(!store.contains(key(4)) && !store.contains(key(5)));
        store.end_frame();
        store.evict(0);
        assert!(store.is_empty());
    }

    #[test]
    fn neighbouring_writes_are_reported_as_one() {
        let mut page = Page::new(32);
        page.mark_dirty(10, 12);
        page.mark_dirty(0, 2);
        page.mark_dirty(20, 22);
        page.mark_dirty(2, 10);
        assert_eq!(page.dirty, vec![(0, 12), (20, 22)]);
        page.mark_dirty(11, 21);
        assert_eq!(page.dirty, vec![(0, 22)]);
    }

    #[test]
    fn freed_runs_coalesce() {
        let mut page = Page::new(12);
        let a = page.alloc(4).unwrap();
        let b = page.alloc(4).unwrap();
        let c = page.alloc(4).unwrap();
        page.free(a, 4);
        page.free(c, 4);
        page.free(b, 4);
        assert_eq!(page.free, vec![(0, 12)]);
    }
}
