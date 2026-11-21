//! What stays in the store and what goes: the line of eviction, and the
//! values which are held or pinned out of it.

use std::sync::Arc;

use crate::{key::ContentKey, store::Store};

impl Store {
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

    /// Puts `key` last in line, as used in this frame.
    pub(super) fn mark_used(&mut self, key: ContentKey) {
        self.used.push_back((self.frame, key));
        // Most of what is in line is outdated once it is far longer than
        // there are entries. Clearing it out then costs as much as it grew.
        if self.used.len() > 2 * self.entries.len() + 64 {
            let entries = &self.entries;
            self.used
                .retain(|(frame, key)| entries.get(key).is_some_and(|entry| entry.last_used == *frame));
        }
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
}
