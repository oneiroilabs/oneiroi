
// Where along its whole curve a segment starts and ends, as fractions of
// the length, the same as `fractions` on the CPU.
fn curve_fractions(segment: CubicNurbsSegmentCache, last: CubicNurbsSegmentCache) -> vec2<f32> {
    let total = max(last.cumulative_length, 1.1754944e-38);
    let end = segment.cumulative_length;
    return vec2<f32>((end - segment.length) / total, end / total);
}

// Where each curve of the spine starts among its segments, and behind the
// last one where it ends.
fn spine_offset(group: u32) -> u32 {
    let groups = in_spine_groups();
    if (group < groups) {
        return in_spine_start(group);
    }
    if (groups == 0u) {
        return 0u;
    }
    return in_spine_end(groups - 1u);
}
