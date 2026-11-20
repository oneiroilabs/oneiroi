// How the GPU sweeps: which curve a patch belongs to and where it is cut,
// around `sweep_pieces` and `sweep_piece` of the profile.
// How many patches a cap has at each end of a curve.
fn sweep_caps() -> u32 {
    if (in_ends(0u) == 0u) {
        return sweep_pieces();
    }
    return 0u;
}

// Where the patches of each curve start, and behind the last one where
// they end: its sides, then its caps.
fn sweep_offset(group: u32) -> u32 {
    return spine_offset(group) * sweep_pieces() + group * 2u * sweep_caps();
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

fn sweep_patch(index: u32) -> Patch {
    let curve = sweep_group_of(index);
    let first = in_spine_start(curve);
    let end = in_spine_end(curve);
    let pieces = sweep_pieces();
    let local = index - sweep_offset(curve);
    let sides = (end - first) * pieces;
    let last = in_spine(end - 1u);
    if (local < sides) {
        let at = first + local / pieces;
        let closed = sweep_closed(curve);
        var miters = array<vec4<f32>, 2>(vec4<f32>(0.0), vec4<f32>(0.0));
        if (at > first) {
            miters[0] = sweep_between(sweep_tangent(at - 1u, 1.0), sweep_tangent(at, 0.0));
        } else if (closed) {
            miters[0] = sweep_between(sweep_tangent(end - 1u, 1.0), sweep_tangent(at, 0.0));
        }
        if (at + 1u < end) {
            miters[1] = sweep_between(sweep_tangent(at, 1.0), sweep_tangent(at + 1u, 0.0));
        } else if (closed) {
            miters[1] = sweep_between(sweep_tangent(at, 1.0), sweep_tangent(first, 0.0));
        }
        let along = curve_fractions(in_spine(at), last);
        return sweep_piece(in_spine(at), curve, local % pieces, along, 0u, miters[0], miters[1]);
    }
    // A cap: at the start, then at the end.
    let cap = local - sides;
    var kind = 1u + cap / pieces;
    if (sweep_closed(curve)) {
        kind = 3u;
    }
    var segment = in_spine(first);
    var along = vec2<f32>(0.0);
    if (cap >= pieces) {
        segment = last;
        along = vec2<f32>(1.0);
    }
    return sweep_piece(segment, curve, cap % pieces, along, kind, vec4<f32>(0.0), vec4<f32>(0.0));
}
