//! The content-addressed storage every backend reads from and writes to.
//!
//! All data lives in pages of 32-bit words. A [`Handle`] addresses a run of
//! words inside one page and means the same thing on the CPU (an index into
//! a `Vec<u32>`) and on the GPU (an index into a bound array of buffers).
//!
//! Entries are looked up by [`ContentKey`]. Everything in here can be
//! recomputed from parameters, so the store is free to evict.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    hash::{Hash, Hasher},
    ptr::NonNull,
    sync::{Arc, Weak},
};

use xxhash_rust::xxh3::{Xxh3, xxh3_128};

use bytemuck::{Pod, Zeroable};

use crate::types::{DataType, TypeInfo};

/// Default page size: 1 MiB.
pub const PAGE_WORDS: u32 = 1 << 18;

/// Identifies a value by how it was made instead of where it lives.
///
/// Two values with the same key are interchangeable, which is what lets
/// instances with equal parameters share their results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContentKey(u128);

/// A key is a hash already, evenly spread over all its bits, so a map
/// takes some of them as they are instead of hashing it again.
impl Hash for ContentKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0 as u64);
    }
}

impl nohash_hasher::IsEnabled for ContentKey {}

/// A map by [`ContentKey`], which does not hash the keys again.
pub type KeyMap<V> = HashMap<ContentKey, V, nohash_hasher::BuildNoHashHasher<ContentKey>>;

/// A set of [`ContentKey`]s, which does not hash them again.
pub type KeySet = std::collections::HashSet<ContentKey, nohash_hasher::BuildNoHashHasher<ContentKey>>;

impl ContentKey {
    /// Stands in for a key which is not derived yet.
    pub(crate) const UNSET: Self = Self(0);

    /// The key of a leaf value: it is identified by its type and its bytes.
    pub fn of_data(ty: &TypeInfo, words: &[u32]) -> Self {
        // What the streaming hasher would be given, see `KeyHasher`. XXH3
        // comes to the same over the same bytes either way, and small
        // values, like the elements of most groups, are hashed at once.
        const ON_STACK: usize = 1024;
        let len = 1 + ty.name.len() + 1 + size_of::<usize>() + 4 * words.len();
        if len > ON_STACK {
            let mut hasher = KeyHasher::new(0);
            hasher.write(ty.name);
            hasher.write(words);
            return hasher.finish();
        }
        let mut bytes = [0u8; ON_STACK];
        let mut at = 0;
        let mut put = |part: &[u8]| {
            bytes[at..at + part.len()].copy_from_slice(part);
            at += part.len();
        };
        put(&[0]);
        put(ty.name.as_bytes());
        put(&[0xff]);
        put(&words.len().to_ne_bytes());
        put(bytemuck::cast_slice(words));
        ContentKey(xxh3_128(&bytes[..len]))
    }

    /// [`ContentKey::derive`] for a node: its results change when its
    /// version does.
    pub fn derive_node(node: &str, version: u32, output: u32, inputs: &[ContentKey]) -> Self {
        derived(2, node, &[version as u64, output as u64], inputs)
    }

    /// The key of the `output`-th result of running `node` on `inputs`.
    pub fn derive(node: &str, output: u32, inputs: &[ContentKey]) -> Self {
        derived(1, node, &[output as u64], inputs)
    }

    /// The key of the `output`-th result of running the program with the
    /// hash `program` for each element `element` of `inputs`, at most `max`
    /// times.
    pub fn derive_for_each(program: u64, element: u32, max: u32, output: u32, inputs: &[ContentKey]) -> Self {
        derived(3, "for each", &[program, element as u64, max as u64, output as u64], inputs)
    }
}

/// The key of whatever comes out of `inputs` by what `name` and `numbers`
/// say. The few bytes this takes are put together on the stack and hashed
/// at once, which costs a fraction of setting up a streaming hasher: keys
/// are derived for every node of every element.
fn derived(domain: u8, name: &str, numbers: &[u64], inputs: &[ContentKey]) -> ContentKey {
    const ON_STACK: usize = 512;
    let len = 1 + 8 + name.len() + 8 * numbers.len() + 8 + 16 * inputs.len();
    let mut stack = [0u8; ON_STACK];
    let mut heap = Vec::new();
    let bytes = match len <= ON_STACK {
        true => &mut stack[..len],
        false => {
            heap.resize(len, 0);
            &mut heap[..]
        }
    };
    let mut at = 0;
    let mut put = |part: &[u8]| {
        bytes[at..at + part.len()].copy_from_slice(part);
        at += part.len();
    };
    put(&[domain]);
    put(&(name.len() as u64).to_le_bytes());
    put(name.as_bytes());
    for number in numbers {
        put(&number.to_le_bytes());
    }
    put(&(inputs.len() as u64).to_le_bytes());
    for input in inputs {
        put(&input.0.to_le_bytes());
    }
    ContentKey(xxh3_128(bytes))
}

/// XXH3 with 128 bits: fast over the megabytes a parameter can have, and
/// with too many bits for two different values to meet by chance. Keys are
/// never shared with anybody, so nothing has to resist an attacker.
struct KeyHasher(Xxh3);

impl KeyHasher {
    fn new(domain: u8) -> Self {
        let mut hasher = Self(Xxh3::new());
        hasher.write(domain);
        hasher
    }

    fn write(&mut self, value: impl Hash) {
        value.hash(&mut self.0);
    }

    fn finish(self) -> ContentKey {
        ContentKey(self.0.digest128())
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

/// The words of a page. They never move while the page lives, so a part of
/// them can be written while other parts are read, see
/// [`Store::output_words`].
struct Words {
    ptr: NonNull<u32>,
    len: usize,
}

// The words are owned like a `Box<[u32]>` and only reached through it.
unsafe impl Send for Words {}
unsafe impl Sync for Words {}

impl Words {
    fn zeroed(len: usize) -> Self {
        let boxed: Box<[u32]> = vec![0; len].into_boxed_slice();
        let ptr = NonNull::new(Box::into_raw(boxed) as *mut u32).expect("a box is never null");
        Self { ptr, len }
    }

    fn all(&self) -> &[u32] {
        // SAFETY: `ptr` owns `len` initialized words for as long as `self`.
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    fn get(&self, offset: usize, len: usize) -> &[u32] {
        &self.all()[offset..][..len]
    }

    fn get_mut(&mut self, offset: usize, len: usize) -> &mut [u32] {
        // SAFETY: `&mut self` makes this the only access.
        unsafe { self.range_mut(offset, len) }
    }

    /// The words `offset..offset + len` to write.
    ///
    /// # Safety
    ///
    /// Nothing else may read or write these words while the slice lives.
    /// Other words of the page may be read and written meanwhile, since the
    /// slice is made from the pointer and not from a borrow of all words.
    unsafe fn range_mut<'a>(&self, offset: usize, len: usize) -> &'a mut [u32] {
        assert!(offset + len <= self.len, "the words are inside the page");
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr().add(offset), len) }
    }
}

impl Drop for Words {
    fn drop(&mut self) {
        // SAFETY: made from this box in `zeroed`, and dropped once.
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(self.ptr.as_ptr(), self.len)));
        }
    }
}

/// How many dirty runs a page keeps before it merges them.
const DIRTY_RUNS: usize = 1024;

struct Page {
    words: Words,
    /// Free runs, `offset → len`, never adjacent. [`Store::free_runs`] has
    /// them by size.
    free: BTreeMap<u32, u32>,
    /// The runs of words written since somebody last asked, as
    /// `(start, end)`, in the order they were written. Without anybody
    /// asking, as when nothing mirrors the pages, they are merged once
    /// there are many, so they never grow without bound.
    dirty: Vec<(u32, u32)>,
}

impl Page {
    fn new(words: u32) -> Self {
        Self {
            words: Words::zeroed(words as usize),
            free: BTreeMap::new(),
            dirty: Vec::new(),
        }
    }

    fn mark_dirty(&mut self, start: u32, end: u32) {
        self.dirty.push((start, end));
        if self.dirty.len() > DIRTY_RUNS {
            self.merge_dirty();
        }
    }

    /// The dirty runs sorted and merged where they touch. Should there
    /// still be many, the smallest gaps between them are taken in too: a
    /// little more is uploaded, in far fewer pieces.
    fn merge_dirty(&mut self) {
        let dirty = &mut self.dirty;
        dirty.sort_unstable();
        let mut kept = 0;
        for index in 0..dirty.len() {
            let (start, end) = dirty[index];
            match kept > 0 && start <= dirty[kept - 1].1 {
                true => dirty[kept - 1].1 = dirty[kept - 1].1.max(end),
                false => {
                    dirty[kept] = (start, end);
                    kept += 1;
                }
            }
        }
        dirty.truncate(kept);

        if dirty.len() > DIRTY_RUNS / 2 {
            let mut gaps: Vec<u32> = dirty.windows(2).map(|pair| pair[1].0 - pair[0].1).collect();
            let bridged = dirty.len() - DIRTY_RUNS / 4;
            let widest = *gaps.select_nth_unstable(bridged - 1).1;
            let mut kept = 0;
            for index in 0..dirty.len() {
                let (start, end) = dirty[index];
                match kept > 0 && start - dirty[kept - 1].1 <= widest {
                    true => dirty[kept - 1].1 = end,
                    false => {
                        dirty[kept] = (start, end);
                        kept += 1;
                    }
                }
            }
            dirty.truncate(kept);
        }
    }

    fn take_dirty(&mut self) -> Vec<(u32, u32)> {
        self.merge_dirty();
        std::mem::take(&mut self.dirty)
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

    /// Keeps the value of `key` from being evicted for as long as `holder`
    /// lives, or until it is [released](Store::release).
    pub fn hold(&mut self, key: ContentKey, holder: &Arc<()>) {
        if let Some(entry) = self.entries.get_mut(&key)
            && !entry.holders.iter().any(|held| held.as_ptr() == Arc::as_ptr(holder))
        {
            entry.holders.push(Arc::downgrade(holder));
        }
    }

    /// Keeps the values of `keys` for as long as `holder` lives, like
    /// [`Store::hold`], but out of the line of eviction: however many there
    /// are, evicting never looks at them, only at whether `holder` still
    /// lives. For the many values which never change while a program is in
    /// use, like the edge of its constant part. Keys not in the store are
    /// left out.
    pub fn pin(&mut self, keys: impl IntoIterator<Item = ContentKey>, holder: &Arc<()>) {
        let keys: Vec<ContentKey> = keys
            .into_iter()
            .filter(|key| match self.entries.get_mut(key) {
                Some(entry) => {
                    entry.pins += 1;
                    true
                }
                None => false,
            })
            .collect();
        self.pinned.push((Arc::downgrade(holder), keys));
    }

    /// Puts what nobody pins anymore back in line, as the oldest there is.
    fn unpin_dead(&mut self) {
        let mut index = 0;
        while index < self.pinned.len() {
            if self.pinned[index].0.strong_count() > 0 {
                index += 1;
                continue;
            }
            let (_, keys) = self.pinned.swap_remove(index);
            for key in keys {
                if let Some(entry) = self.entries.get_mut(&key) {
                    entry.pins -= 1;
                    if entry.pins == 0 {
                        self.used.push_front((entry.last_used, key));
                    }
                }
            }
        }
    }

    /// Lets the value of `key` be evicted again, as far as `holder` is
    /// concerned.
    pub fn release(&mut self, key: ContentKey, holder: &Arc<()>) {
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.holders.retain(|held| held.as_ptr() != Arc::as_ptr(holder));
            if entry.holders.is_empty() && entry.out_of_line {
                entry.out_of_line = false;
                self.used.push_front((entry.last_used, key));
            }
        }
    }

    /// Puts what nobody holds anymore back in line, as the oldest there
    /// is. Only the holders which died are looked at.
    fn return_released(&mut self) {
        let mut index = 0;
        while index < self.held.len() {
            if self.held[index].0.strong_count() > 0 {
                index += 1;
                continue;
            }
            let (_, key) = self.held.swap_remove(index);
            if let Some(entry) = self.entries.get_mut(&key) {
                entry.holders.retain(|holder| holder.strong_count() > 0);
                if entry.holders.is_empty() && entry.out_of_line {
                    entry.out_of_line = false;
                    self.used.push_front((entry.last_used, key));
                }
            }
        }
        // Holders which live and let go of what they held leave their
        // records behind. Clearing them out costs as much as they grew.
        if self.held.len() > 2 * self.entries.len() + 64 {
            let entries = &self.entries;
            self.held.retain(|(holder, key)| {
                entries.get(key).is_some_and(|entry| {
                    entry.out_of_line && entry.holders.iter().any(|held| held.ptr_eq(holder))
                })
            });
        }
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

    /// Puts `key` last in line, as used in this frame.
    fn mark_used(&mut self, key: ContentKey) {
        self.used.push_back((self.frame, key));
        // Most of what is in line is outdated once it is far longer than
        // there are entries. Clearing it out then costs as much as it grew.
        if self.used.len() > 2 * self.entries.len() + 64 {
            let entries = &self.entries;
            self.used
                .retain(|(frame, key)| entries.get(key).is_some_and(|entry| entry.last_used == *frame));
        }
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

    /// Room for `len` words: the smallest free run which fits, or a new
    /// page if none does.
    fn alloc(&mut self, len: u32) -> Handle {
        if len == 0 {
            if self.pages.is_empty() {
                self.pages.push(Page::new(self.page_words));
                self.add_free_run(0, 0, self.page_words);
            }
            return Handle { page: 0, offset: 0, len };
        }
        if let Some(&(run, page, offset)) = self.free_runs.range((len, 0, 0)..).next() {
            self.free_runs.remove(&(run, page, offset));
            self.pages[page as usize].free.remove(&offset);
            if run > len {
                self.add_free_run(page, offset + len, run - len);
            }
            return Handle { page, offset, len };
        }

        let page = self.pages.len() as u32;
        let words = self.page_words.max(len);
        self.pages.push(Page::new(words));
        if words > len {
            self.add_free_run(page, len, words - len);
        }
        Handle { page, offset: 0, len }
    }

    /// Gives the room of `handle` back, merged with the free runs next to
    /// it.
    fn free(&mut self, handle: Handle) {
        if handle.len == 0 {
            return;
        }
        let Handle { page, mut offset, mut len } = handle;
        let runs = &mut self.pages[page as usize].free;
        if let Some((&before, &before_len)) = runs.range(..offset).next_back()
            && before + before_len == offset
        {
            runs.remove(&before);
            self.free_runs.remove(&(before_len, page, before));
            offset = before;
            len += before_len;
        }
        if let Some(after_len) = runs.remove(&(handle.offset + handle.len)) {
            self.free_runs.remove(&(after_len, page, handle.offset + handle.len));
            len += after_len;
        }
        self.add_free_run(page, offset, len);
    }

    fn add_free_run(&mut self, page: u32, offset: u32, len: u32) {
        self.pages[page as usize].free.insert(offset, len);
        self.free_runs.insert((len, page, offset));
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

    /// Frees the least recently used entries until at most `budget_words`
    /// are in use. Entries touched in the current frame are never freed.
    /// Returns how many entries were evicted.
    pub fn evict(&mut self, budget_words: u64) -> usize {
        self.unpin_dead();
        self.return_released();
        let mut evicted = 0;
        while self.used_words > budget_words {
            let Some(&(frame, key)) = self.used.front() else {
                break;
            };
            // Everything from here on was used in this frame.
            if frame >= self.frame {
                break;
            }
            self.used.pop_front();
            let Some(entry) = self.entries.get_mut(&key) else {
                continue;
            };
            // It was used again since, or is out of line while pinned.
            if entry.last_used != frame || entry.pins > 0 {
                continue;
            }
            entry.holders.retain(|holder| holder.strong_count() > 0);
            if !entry.holders.is_empty() {
                // Out of line until nobody holds it anymore.
                if !entry.out_of_line {
                    entry.out_of_line = true;
                    self.held.extend(entry.holders.iter().map(|holder| (holder.clone(), key)));
                }
                continue;
            }
            evicted += self.remove_entry(key);
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
                page.take_dirty()
                    .into_iter()
                    .map(move |(start, end)| (index as u32, start..end))
            })
            .collect()
    }

    /// The raw pages, in the order a GPU backend binds them.
    pub fn pages(&self) -> impl ExactSizeIterator<Item = &[u32]> {
        self.pages.iter().map(|page| page.words.all())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(n: u32) -> ContentKey {
        ContentKey::of_data(&u32::INFO, &[n])
    }

    #[test]
    fn what_is_held_is_out_of_line_until_nobody_holds_it() {
        let mut store = Store::new();
        let (a, b) = (Arc::new(()), Arc::new(()));
        store.insert(key(1), &[1]);
        store.insert(key(2), &[2]);
        store.hold(key(1), &a);
        store.hold(key(2), &b);
        store.end_frame();
        assert_eq!(store.evict(0), 0);
        // Out of line, evicting looks at nothing of them.
        assert!(store.used.is_empty());
        assert_eq!(store.held.len(), 2);

        // Let go of, or with its holder gone, it is first in line.
        store.release(key(1), &a);
        drop(b);
        assert_eq!(store.evict(0), 2);
        assert!(store.is_empty());
        // A record of a holder which lives is cleared once it is gone.
        drop(a);
        store.evict(0);
        assert!(store.held.is_empty());
    }

    #[test]
    fn many_values_are_reserved_next_to_each_other_and_freed_on_their_own() {
        let mut store = Store::with_page_words(16);
        let there = store.reserve(key(0), 3);
        let wanted = [(key(1), 4), (key(0), 3), (key(2), 6), (key(3), 10), (key(1), 4)];
        let mut handles = Vec::new();
        store.reserve_many(&wanted, &mut handles);

        assert_eq!(handles[1], there, "what is there keeps its place");
        assert_eq!(handles[4], handles[0], "what is wanted twice is one value");
        assert_eq!(handles[2].page, handles[0].page);
        assert_eq!(handles[2].offset, handles[0].offset + 4, "one after the other");
        assert_ne!(handles[3].page, handles[0].page, "what does not fit goes on");
        assert_eq!(store.used_words(), 3 + 4 + 6 + 10);
        for (key, handle) in wanted.iter().map(|(key, _)| *key).zip(&handles) {
            assert_eq!(store.get(key), Some(*handle));
        }

        // Every piece goes back on its own, and the room is found again.
        store.evict(0);
        store.end_frame();
        assert_eq!(store.evict(0), 4);
        assert_eq!(store.used_words(), 0);
        let again = store.reserve(key(4), 16);
        assert_eq!(again.len, 16);
        assert!(again.page <= handles[3].page);
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
    fn derived_keys_depend_on_every_bit_of_their_inputs() {
        let low = ContentKey(1);
        let high = ContentKey(1 | 1 << 100);
        assert_ne!(ContentKey::derive("a", 0, &[low]), ContentKey::derive("a", 0, &[high]));
        // However long the inputs are.
        let many = |last| [vec![ContentKey(7); 40], vec![last]].concat();
        assert_ne!(ContentKey::derive("a", 0, &many(low)), ContentKey::derive("a", 0, &many(high)));
        // Names and numbers do not run into each other.
        assert_ne!(ContentKey::derive("a", 0, &[]), ContentKey::derive("a\0", 0, &[]));
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
    fn data_is_keyed_while_it_is_copied() {
        let mut store = Store::with_page_words(8);
        // More than one chunk of hashing, few enough to be hashed at once,
        // and nothing at all.
        let words: Vec<u32> = (0..10_000).collect();
        for words in [&words[..], &words[..5], &words[..250], &words[..260], &[]] {
            let (key, handle) = store.insert_data(&u32::INFO, words);
            assert_eq!(key, ContentKey::of_data(&u32::INFO, words));
            assert_eq!(store.words(handle), words);
        }
        // What is there already is neither copied nor stored twice.
        let used = store.used_words();
        store.insert_data(&u32::INFO, &words);
        assert_eq!(store.used_words(), used);
    }

    #[test]
    fn held_values_stay_until_their_holder_is_gone() {
        let mut store = Store::with_page_words(8);
        let holder = Arc::new(());
        let (held, _) = store.insert_data(&u32::INFO, &[1; 4]);
        store.hold(held, &holder);
        store.hold(held, &holder);
        store.insert(key(2), &[2; 4]);
        store.end_frame();

        assert_eq!(store.evict(0), 1);
        assert!(store.contains(held) && !store.contains(key(2)));

        store.release(held, &holder);
        store.hold(held, &holder);
        drop(holder);
        assert_eq!(store.evict(0), 1);
        assert!(!store.contains(held));
    }

    #[test]
    fn outputs_are_written_in_place() {
        let mut store = Store::with_page_words(8);
        let handle = store.reserve_output(key(1), 3);
        assert!(store.contains(key(1)) && store.read::<u32>(key(1)).is_none());
        // SAFETY: nothing else reads or writes the reserved words.
        unsafe { store.output_words(handle) }.copy_from_slice(&[4, 5, 6]);
        store.commit(key(1));
        assert_eq!(store.read::<u32>(key(1)), Some(&[4, 5, 6][..]));
        assert_eq!(store.take_dirty(), [(0, 0..3)]);
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
        assert_eq!(page.take_dirty(), vec![(0, 12), (20, 22)]);
        page.mark_dirty(0, 12);
        page.mark_dirty(20, 22);
        page.mark_dirty(11, 21);
        assert_eq!(page.take_dirty(), vec![(0, 22)]);
    }

    #[test]
    fn many_writes_stay_few_runs() {
        let mut page = Page::new(1 << 20);
        let writes = (0..100_000u32).map(|index| (index * 7919) % 100_000 * 8);
        for start in writes.clone() {
            page.mark_dirty(start, start + 2);
        }
        let dirty = page.take_dirty();
        assert!(dirty.len() <= DIRTY_RUNS / 2);
        // Nothing which was written is left out.
        assert!(dirty.windows(2).all(|pair| pair[0].1 < pair[1].0));
        for start in writes {
            assert!(dirty.iter().any(|&(from, to)| from <= start && start + 2 <= to));
        }
    }

    #[test]
    fn freed_runs_coalesce() {
        let mut store = Store::with_page_words(12);
        let a = store.alloc(4);
        let b = store.alloc(4);
        let c = store.alloc(4);
        store.free(a);
        store.free(c);
        store.free(b);
        assert_eq!(store.pages[0].free, BTreeMap::from([(0, 12)]));
        assert_eq!(store.free_runs, BTreeSet::from([(12, 0, 0)]));
    }

    #[test]
    fn the_smallest_run_which_fits_is_taken() {
        let mut store = Store::with_page_words(16);
        let handles: Vec<_> = [3, 1, 5, 1, 2].into_iter().map(|len| store.alloc(len)).collect();
        store.free(handles[0]);
        store.free(handles[2]);
        // Into the run of 3 rather than the one of 5.
        assert_eq!(store.alloc(3).offset, 0);
        // Into what is left at the end, which fits exactly.
        assert_eq!(store.alloc(4).offset, 12);
        assert_eq!(store.alloc(5).offset, 4);
        assert_eq!(store.alloc(1).page, 1);
    }

    #[test]
    fn parts_go_with_what_they_are_part_of() {
        let mut store = Store::with_page_words(64);
        store.insert(key(1), &[1, 2, 3, 4]);
        for part in 0..100 {
            store.alias(key(1000 + part), key(1), part % 4, 1);
        }
        // A part of a part, and parts which went on their own already.
        store.alias(key(2000), key(1000), 0, 1);
        store.end_frame();
        store.touch(key(1));
        store.evict(0);
        assert_eq!(store.len(), 1);
        assert!(store.entries[&key(1)].parts.len() <= 100);

        store.alias(key(1000), key(1), 0, 1);
        store.alias(key(2000), key(1000), 0, 1);
        store.end_frame();
        assert_eq!(store.evict(0), 3);
        assert!(store.is_empty());
        assert_eq!(store.used_words(), 0);
    }

    #[test]
    fn eviction_leaves_held_values_first_in_line() {
        let mut store = Store::with_page_words(8);
        let holder = Arc::new(());
        store.insert(key(1), &[1]);
        store.hold(key(1), &holder);
        store.end_frame();
        store.insert(key(2), &[2]);
        store.end_frame();
        assert_eq!(store.evict(0), 1);
        assert!(store.contains(key(1)));

        drop(holder);
        store.end_frame();
        assert_eq!(store.evict(0), 1);
        assert!(store.is_empty());
    }

    #[test]
    fn what_was_used_stays_bounded_without_eviction() {
        let mut store = Store::with_page_words(64);
        for frame in 0..1000u32 {
            store.insert(key(frame % 10), &[frame]);
            for n in 0..10 {
                store.touch(key(n));
            }
            store.end_frame();
        }
        assert!(store.used.len() <= 2 * store.len() + 64 + 10);
    }
}
