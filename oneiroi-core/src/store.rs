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
    /// The words written since somebody last asked, as `start..end`.
    dirty: Option<(u32, u32)>,
}

impl Page {
    fn new(words: u32) -> Self {
        Self {
            words: vec![0; words as usize],
            free: vec![(0, words)],
            dirty: None,
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
        Some(entry.handle)
    }

    /// Stores `words` under `key`. An existing entry is kept as it is,
    /// since equal keys mean equal content.
    pub fn insert(&mut self, key: ContentKey, words: &[u32]) -> Handle {
        if let Some(handle) = self.touch(key) {
            return handle;
        }

        let handle = self.alloc(words.len() as u32);
        let page = &mut self.pages[handle.page as usize];
        page.words[handle.offset as usize..][..words.len()].copy_from_slice(words);
        if !words.is_empty() {
            let (start, end) = (handle.offset, handle.offset + handle.len);
            page.dirty = Some(match page.dirty {
                Some((from, to)) => (from.min(start), to.max(end)),
                None => (start, end),
            });
        }
        self.used_words += words.len() as u64;
        self.entries.insert(
            key,
            Entry {
                handle,
                last_used: self.frame,
            },
        );
        handle
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

    pub fn read<T: DataType>(&self, key: ContentKey) -> Option<&[T]> {
        self.get(key).map(|handle| self.slice(handle))
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
            let handle = self.entries.remove(&key).unwrap().handle;
            self.pages[handle.page as usize].free(handle.offset, handle.len);
            self.used_words -= handle.len as u64;
            evicted += 1;
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
            .filter_map(|(index, page)| {
                let (start, end) = page.dirty.take()?;
                Some((index as u32, start..end))
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
