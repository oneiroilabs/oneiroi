//! The pages the words of values live in, and how room is found in them.

use std::{collections::BTreeMap, ptr::NonNull};

use crate::store::{Handle, Store};

/// Default page size: 1 MiB.
pub const PAGE_WORDS: u32 = 1 << 18;

/// The words of a page. They never move while the page lives, so a part of
/// them can be written while other parts are read, see
/// [`Store::output_words`].
pub(super) struct Words {
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

    pub(super) fn get(&self, offset: usize, len: usize) -> &[u32] {
        &self.all()[offset..][..len]
    }

    pub(super) fn get_mut(&mut self, offset: usize, len: usize) -> &mut [u32] {
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
    pub(super) unsafe fn range_mut<'a>(&self, offset: usize, len: usize) -> &'a mut [u32] {
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
pub(super) const DIRTY_RUNS: usize = 1024;

pub(super) struct Page {
    pub(super) words: Words,
    /// Free runs, `offset → len`, never adjacent. [`Store::free_runs`] has
    /// them by size.
    pub(super) free: BTreeMap<u32, u32>,
    /// The runs of words written since somebody last asked, as
    /// `(start, end)`, in the order they were written. Without anybody
    /// asking, as when nothing mirrors the pages, they are merged once
    /// there are many, so they never grow without bound.
    dirty: Vec<(u32, u32)>,
}

impl Page {
    pub(super) fn new(words: u32) -> Self {
        Self {
            words: Words::zeroed(words as usize),
            free: BTreeMap::new(),
            dirty: Vec::new(),
        }
    }

    pub(super) fn mark_dirty(&mut self, start: u32, end: u32) {
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

    pub(super) fn take_dirty(&mut self) -> Vec<(u32, u32)> {
        self.merge_dirty();
        std::mem::take(&mut self.dirty)
    }
}

impl Store {
    /// Room for `len` words: the smallest free run which fits, or a new
    /// page if none does.
    pub(super) fn alloc(&mut self, len: u32) -> Handle {
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
    pub(super) fn free(&mut self, handle: Handle) {
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
