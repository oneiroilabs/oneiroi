// How the GPU sweeps: which curve a patch belongs to, where it is cut and
// how big the profile is there, around `sweep_pieces` and `sweep_piece` of
// the profile. The same as `sweep_curve` on the CPU.

// How many patches a cap has at each end of a curve.
fn sweep_caps() -> u32 {
    if (in_ends(0u) == 0u) {
        return sweep_pieces();
    }
    return 0u;
}

// Into how many pieces a segment can be cut at most: as many as the
// longest curve of the scale has segments.
fn sweep_slots() -> u32 {
    if (in_scale_len() == 0u) {
        return 1u;
    }
    var most = 1u;
    for (var group = 0u; group < in_scale_groups(); group++) {
        most = max(most, in_scale_end(group) - in_scale_start(group));
    }
    return most;
}

// Where the patches of each curve start, and behind the last one where
// they end: its sides, then its caps.
fn sweep_offset(group: u32) -> u32 {
    return spine_offset(group) * sweep_slots() * sweep_pieces() + group * 2u * sweep_caps();
}

fn sweep_group_of(index: u32) -> u32 {
    var low = 0u;
    var high = in_spine_groups();
    while (high - low > 1u) {
        let middle = (low + high) / 2u;
        if (sweep_offset(middle) <= index) {
            low = middle;
        } else {
            high = middle;
        }
    }
    return low;
}

fn sweep_tangent(segment: u32, u: f32) -> vec3<f32> {
    return curve_tangent(in_spine(segment), u);
}

// The plane between two pieces of a curve which meet at an angle, the same
// as `miters` on the CPU.
fn sweep_between(from_: vec3<f32>, to: vec3<f32>) -> vec4<f32> {
    let turn = dot(from_, to);
    if (turn > 1.0 - 1e-4 || turn < -1.0 + 1e-4) {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(normalize(from_ + to), 0.0);
}

fn sweep_closed(curve: u32) -> bool {
    let first = in_spine_start(curve);
    let end = in_spine_end(curve);
    if (end <= first + 1u) {
        return false;
    }
    let last = in_spine(end - 1u);
    let gap = distance(curve_point(in_spine(first), 0.0).position, curve_point(last, 1.0).position);
    return gap <= 1e-5 * (1.0 + last.cumulative_length);
}

// The segments of the scale of `curve`, as where they start and end.
fn sweep_scale_range(curve: u32) -> vec2<u32> {
    if (in_scale_len() == 0u) {
        return vec2<u32>(0u);
    }
    let group = min(curve, in_scale_groups() - 1u);
    return vec2<u32>(in_scale_start(group), in_scale_end(group));
}

fn sweep_scale_x(segment: u32, u: f32) -> f32 {
    return curve_point(in_scale(segment), u).position.x;
}

// The scale `along` the spine of `curve`, and how fast it changes there:
// where two segments of it meet, as it goes on `after` it or as it came.
// The same as `Scaling::at` on the CPU.
fn sweep_scale_at(curve: u32, along: f32, after: bool) -> vec2<f32> {
    let range = sweep_scale_range(curve);
    if (range.x == range.y) {
        return vec2<f32>(1.0, 0.0);
    }
    let first = range.x;
    let last = range.y - 1u;
    if (along < sweep_scale_x(first, 0.0)) {
        return vec2<f32>(curve_point(in_scale(first), 0.0).position.y, 0.0);
    }
    if (along > sweep_scale_x(last, 1.0)) {
        return vec2<f32>(curve_point(in_scale(last), 1.0).position.y, 0.0);
    }
    var segment = last;
    if (after) {
        segment = first;
        for (var index = range.y; index > range.x; index--) {
            if (sweep_scale_x(index - 1u, 0.0) <= along) {
                segment = index - 1u;
                break;
            }
        }
    } else {
        for (var index = range.x; index < range.y; index++) {
            if (along <= sweep_scale_x(index, 1.0)) {
                segment = index;
                break;
            }
        }
    }
    // Right at an end of the segment, or else halving.
    var u = 0.0;
    if (along == sweep_scale_x(segment, 0.0)) {
        u = 0.0;
    } else if (along == sweep_scale_x(segment, 1.0)) {
        u = 1.0;
    } else {
        var low = 0.0;
        var high = 1.0;
        for (var step = 0u; step < 32u; step++) {
            let middle = (low + high) * 0.5;
            if (sweep_scale_x(segment, middle) < along) {
                low = middle;
            } else {
                high = middle;
            }
        }
        u = (low + high) * 0.5;
    }
    let point = curve_point(in_scale(segment), u);
    var slope = 0.0;
    if (abs(point.velocity.x) > 1e-12) {
        slope = point.velocity.y / point.velocity.x;
    }
    return vec2<f32>(point.position.y, slope);
}

// The scale along `span` of `segment`, which lies `along` of a spine
// `total` long, as `a + b u + c u² + d u³`, the same as `Scaling::cubic`.
fn sweep_scale_cubic(curve: u32, segment: CubicNurbsSegmentCache, span: vec2<f32>, along: vec2<f32>, total: f32) -> vec4<f32> {
    let range = sweep_scale_range(curve);
    if (range.x == range.y) {
        return vec4<f32>(1.0, 0.0, 0.0, 0.0);
    }
    let start = sweep_scale_at(curve, along.x, true);
    let end = sweep_scale_at(curve, along.y, false);
    let m0 = start.y * curve_speed(segment, span.x) * (span.y - span.x) / total;
    let m1 = end.y * curve_speed(segment, span.y) * (span.y - span.x) / total;
    return vec4<f32>(start.x, m0, 3.0 * (end.x - start.x) - 2.0 * m0 - m1, 2.0 * (start.x - end.x) + m0 + m1);
}

// Where the `nth` joint of the scale of `curve` is which lies inside
// `along`, as a fraction of the spine, if there is one.
fn sweep_joint(curve: u32, along: vec2<f32>, nth: u32) -> vec2<f32> {
    let range = sweep_scale_range(curve);
    var found = 0u;
    for (var index = range.x + 1u; index < range.y; index++) {
        let joint = sweep_scale_x(index, 0.0);
        if (joint > along.x + 1e-6 && joint < along.y - 1e-6) {
            if (found == nth) {
                return vec2<f32>(joint, 1.0);
            }
            found++;
        }
    }
    return vec2<f32>(0.0, 0.0);
}

fn sweep_patch(index: u32) -> Patch {
    let curve = sweep_group_of(index);
    let first = in_spine_start(curve);
    let end = in_spine_end(curve);
    let pieces = sweep_pieces();
    let slots = sweep_slots();
    let local = index - sweep_offset(curve);
    let sides = (end - first) * slots * pieces;
    let last = in_spine(end - 1u);
    let total = max(last.cumulative_length, 1.1754944e-38);
    if (local < sides) {
        let at = first + local / (slots * pieces);
        let slot = (local / pieces) % slots;
        let piece = local % pieces;
        let segment = in_spine(at);
        let along = curve_fractions(segment, last);
        let before = segment.cumulative_length - segment.length;

        // The piece of the segment between two cuts, if it has as many.
        var span = vec2<f32>(0.0, 1.0);
        var part = along;
        if (slot > 0u) {
            let joint = sweep_joint(curve, along, slot - 1u);
            if (joint.y == 0.0) {
                // Cut into fewer pieces: this one is not there.
                return sweep_piece(segment, curve, piece, vec2<f32>(along.y), 3u, vec4<f32>(0.0), vec4<f32>(0.0), vec2<f32>(0.0, 1.0), vec4<f32>(0.0));
            }
            span.x = curve_u_at_distance(segment, joint.x * total - before);
            part.x = joint.x;
        }
        let next = sweep_joint(curve, along, slot);
        let ends_here = next.y == 0.0;
        if (!ends_here) {
            span.y = curve_u_at_distance(segment, next.x * total - before);
            part.y = next.x;
        }

        // Only where the segment itself starts and ends can it be at a
        // corner.
        let closed = sweep_closed(curve);
        var miters = array<vec4<f32>, 2>(vec4<f32>(0.0), vec4<f32>(0.0));
        if (slot == 0u) {
            if (at > first) {
                miters[0] = sweep_between(sweep_tangent(at - 1u, 1.0), sweep_tangent(at, 0.0));
            } else if (closed) {
                miters[0] = sweep_between(sweep_tangent(end - 1u, 1.0), sweep_tangent(at, 0.0));
            }
        }
        if (ends_here) {
            if (at + 1u < end) {
                miters[1] = sweep_between(sweep_tangent(at, 1.0), sweep_tangent(at + 1u, 0.0));
            } else if (closed) {
                miters[1] = sweep_between(sweep_tangent(at, 1.0), sweep_tangent(first, 0.0));
            }
        }
        let scale = sweep_scale_cubic(curve, segment, span, part, total);
        return sweep_piece(segment, curve, piece, part, 0u, miters[0], miters[1], span, scale);
    }
    // A cap: at the start, then at the end, as big as the profile is there.
    let cap = local - sides;
    let at_end = cap >= pieces;
    var kind = select(1u, 2u, at_end);
    var segment = in_spine(first);
    var along = vec2<f32>(0.0);
    if (at_end) {
        segment = last;
        along = vec2<f32>(1.0);
    }
    let size = sweep_scale_at(curve, along.x, !at_end).x;
    if (sweep_closed(curve) || size == 0.0) {
        kind = 3u;
    }
    return sweep_piece(segment, curve, cap % pieces, along, kind, vec4<f32>(0.0), vec4<f32>(0.0), vec2<f32>(0.0, 1.0), vec4<f32>(size, 0.0, 0.0, 0.0));
}
