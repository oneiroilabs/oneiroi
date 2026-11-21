use std::{collections::BTreeMap, sync::Arc};

use super::*;
use crate::store::pages::{DIRTY_RUNS, Page};

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
