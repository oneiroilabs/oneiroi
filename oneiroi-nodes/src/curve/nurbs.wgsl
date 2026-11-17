
// The NURBS node: the segments of every curve, made from its control
// points and knots the same way `extract_segments` makes them on the CPU.

fn nurbs_points(curve: u32) -> u32 {
    return in_points_end(curve) - in_points_start(curve);
}

// Knot `k` of a curve: of the group of knots with the same index, or the
// last group, or clamped and uniform without any knots.
fn nurbs_knot(curve: u32, k: u32) -> f32 {
    if (in_knots_len() == 0u) {
        let segments = max(nurbs_points(curve), 4u) - 3u;
        return f32(min(max(k, 3u) - 3u, segments)) / f32(segments);
    }
    let group = min(curve, in_knots_groups() - 1u);
    return in_knots(in_knots_start(group) + k);
}

fn nurbs_valid(curve: u32) -> bool {
    let points = nurbs_points(curve);
    if (points < 4u) {
        return false;
    }
    if (in_knots_len() == 0u) {
        return true;
    }
    let group = min(curve, in_knots_groups() - 1u);
    return in_knots_end(group) - in_knots_start(group) == points + 4u;
}

fn nurbs_is_span(curve: u32, span: u32) -> bool {
    return nurbs_knot(curve, span) < nurbs_knot(curve, span + 1u);
}

fn nurbs_segments(curve: u32) -> u32 {
    if (!nurbs_valid(curve)) {
        return 0u;
    }
    var count = 0u;
    for (var span = 3u; span < nurbs_points(curve); span++) {
        if (nurbs_is_span(curve, span)) {
            count++;
        }
    }
    return count;
}

// De Boor's algorithm with another parameter on every level.
fn nurbs_blossom(curve: u32, span: u32, at: vec3<f32>) -> vec4<f32> {
    let first = in_points_start(curve) + span - 3u;
    var d = array<vec4<f32>, 4>(
        in_points(first),
        in_points(first + 1u),
        in_points(first + 2u),
        in_points(first + 3u),
    );
    for (var level = 1u; level <= 3u; level++) {
        for (var j = 3u; j >= level; j--) {
            let i = span - 3u + j;
            let width = nurbs_knot(curve, i + 4u - level) - nurbs_knot(curve, i);
            var alpha = 0.0;
            if (width > 0.0) {
                alpha = (at[level - 1u] - nurbs_knot(curve, i)) / width;
            }
            d[j] = d[j - 1u] + (d[j] - d[j - 1u]) * alpha;
        }
    }
    return d[3];
}

fn nurbs_basis(curve: u32, span: u32) -> mat4x4<f32> {
    let a = nurbs_knot(curve, span);
    let b = nurbs_knot(curve, span + 1u);
    let points = mat4x4<f32>(
        nurbs_blossom(curve, span, vec3<f32>(a, a, a)),
        nurbs_blossom(curve, span, vec3<f32>(a, a, b)),
        nurbs_blossom(curve, span, vec3<f32>(a, b, b)),
        nurbs_blossom(curve, span, vec3<f32>(b, b, b)),
    );
    let bezier = mat4x4<f32>(
        vec4<f32>(-1.0, 3.0, -3.0, 1.0),
        vec4<f32>(3.0, -6.0, 3.0, 0.0),
        vec4<f32>(-3.0, 3.0, 0.0, 0.0),
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
    );
    return points * bezier;
}

fn nurbs_encode(normal: vec3<f32>, tangent: vec3<f32>) -> vec2<f32> {
    let n = curve_reference_normal(tangent);
    let b = curve_normalize_or_zero(cross(tangent, n));
    return vec2<f32>(dot(normal, n), dot(normal, b));
}

// One step of the double reflection method, as `double_reflect`.
fn nurbs_double_reflect(
    position: vec3<f32>,
    tangent: vec3<f32>,
    normal: vec3<f32>,
    next_position: vec3<f32>,
    next_tangent: vec3<f32>,
) -> vec3<f32> {
    let v1 = next_position - position;
    let c1 = dot(v1, v1);
    if (c1 <= 1e-8) {
        return normal;
    }
    let n_reflected = normal - (2.0 / c1) * dot(v1, normal) * v1;
    let t_reflected = tangent - (2.0 / c1) * dot(v1, tangent) * v1;

    let v2 = next_tangent - t_reflected;
    let c2 = dot(v2, v2);
    var next_normal = n_reflected;
    if (c2 > 1e-8) {
        next_normal = n_reflected - (2.0 / c2) * dot(v2, n_reflected) * v2;
    }
    let binormal = curve_normalize_or_zero(cross(next_tangent, next_normal));
    return curve_normalize_or_zero(cross(binormal, next_tangent));
}

// A segment needs the length of and the frame at the end of the ones
// before it, so every invocation walks its curve from the start.
fn nurbs_segment(index: u32) -> CubicNurbsSegmentCache {
    var curve = 0u;
    var local = index;
    loop {
        let count = nurbs_segments(curve);
        if (local < count) {
            break;
        }
        local -= count;
        curve++;
    }

    var tangent = vec3<f32>(0.0);
    var normal = vec3<f32>(0.0);
    var total = 0.0;
    var seen = 0u;
    var segment = CubicNurbsSegmentCache(mat4x4<f32>(), 0.0, 0.0, vec2<f32>(0.0), vec2<f32>(0.0));
    for (var span = 3u; span < nurbs_points(curve); span++) {
        if (!nurbs_is_span(curve, span)) {
            continue;
        }
        segment = CubicNurbsSegmentCache(nurbs_basis(curve, span), 0.0, 0.0, vec2<f32>(0.0), vec2<f32>(0.0));
        segment.length = curve_length_until(segment, 1.0);
        total += segment.length;
        segment.cumulative_length = total;

        if (seen == 0u) {
            tangent = curve_tangent(segment, 0.0);
            normal = curve_reference_normal(tangent);
        }
        var position = curve_point(segment, 0.0).position;
        let start_tangent = curve_tangent(segment, 0.0);
        normal = curve_rotate_arc(tangent, start_tangent, normal);
        tangent = start_tangent;
        segment.rmf_start_normal = nurbs_encode(normal, tangent);
        for (var step = 1u; step <= 31u; step++) {
            let u = f32(step) / 31.0;
            let next_position = curve_point(segment, u).position;
            let next_tangent = curve_tangent(segment, u);
            normal = nurbs_double_reflect(position, tangent, normal, next_position, next_tangent);
            position = next_position;
            tangent = next_tangent;
        }
        segment.rmf_end_normal = nurbs_encode(normal, tangent);
        if (seen == local) {
            break;
        }
        seen++;
    }
    return segment;
}

fn nurbs_offset(curve: u32) -> u32 {
    var total = 0u;
    for (var before = 0u; before < curve; before++) {
        total += nurbs_segments(before);
    }
    return total;
}
