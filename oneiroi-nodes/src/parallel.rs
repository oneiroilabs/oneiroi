//! Work of a node which is split up by group, to run on every core.

use std::ops::Range;

use rayon::prelude::*;

/// What `make` makes of every group, all of it in the order of the groups,
/// and the offsets where each group starts in it. The groups are made in
/// parallel, but each the same way as on its own, so the result does not
/// depend on how many threads there are.
pub(crate) fn per_group<T: Send>(
    groups: Vec<Range<usize>>,
    make: impl Fn(usize, Range<usize>) -> Vec<T> + Sync,
) -> (Vec<T>, Vec<u32>) {
    let parts: Vec<Vec<T>> = groups
        .into_par_iter()
        .enumerate()
        .map(|(group, range)| make(group, range))
        .collect();
    let mut offsets = Vec::with_capacity(parts.len() + 1);
    offsets.push(0);
    let mut all = Vec::with_capacity(parts.iter().map(Vec::len).sum());
    for part in parts {
        all.extend(part);
        offsets.push(all.len() as u32);
    }
    (all, offsets)
}
