
// The NURBS node: the segments of every curve, made from its control
// points and knots the same way `extract_segments` makes them on the CPU.
// The CPU hands over where the segments of each curve start (`starts`)
// and the knot span of every segment (`spans`). A pass makes the piece of
// every segment on its own (`pieces`), then every segment joins the pieces
// of its curve before it.

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




// De Boor's algorithm with another parameter on every level.
// The first control point of a span, which it is computed relative to,
// as `span_origin`.
fn nurbs_origin(curve: u32, span: u32) -> vec3<f32> {
    let first = nurbs_point(in_points_start(curve) + span - 3u);
    if (first.w != 0.0) {
        return first.xyz / first.w;
    }
    return vec3<f32>(0.0);
}

// A control point in homogeneous coordinates.
fn nurbs_point(index: u32) -> vec4<f32> {
    let point = in_points(index);
    return vec4<f32>(point.position * point.weight, point.weight);
}

fn nurbs_moved(point: vec4<f32>, origin: vec3<f32>) -> vec4<f32> {
    return point - vec4<f32>(origin * point.w, 0.0);
}

fn nurbs_blossom(curve: u32, span: u32, at: vec3<f32>, origin: vec3<f32>) -> vec4<f32> {
    let first = in_points_start(curve) + span - 3u;
    var d = array<vec4<f32>, 4>(
        nurbs_moved(nurbs_point(first), origin),
        nurbs_moved(nurbs_point(first + 1u), origin),
        nurbs_moved(nurbs_point(first + 2u), origin),
        nurbs_moved(nurbs_point(first + 3u), origin),
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

// The basis of a span relative to `origin`.
fn nurbs_basis(curve: u32, span: u32, origin: vec3<f32>) -> mat4x4<f32> {
    let a = nurbs_knot(curve, span);
    let b = nurbs_knot(curve, span + 1u);
    let points = mat4x4<f32>(
        nurbs_blossom(curve, span, vec3<f32>(a, a, a), origin),
        nurbs_blossom(curve, span, vec3<f32>(a, a, b), origin),
        nurbs_blossom(curve, span, vec3<f32>(a, b, b), origin),
        nurbs_blossom(curve, span, vec3<f32>(b, b, b), origin),
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

// The curve a segment belongs to, by a binary search through where the
// segments of each curve start.
fn nurbs_curve_of(index: u32) -> u32 {
    var low = 0u;
    var high = stage_starts_len() - 1u;
    while (high - low > 1u) {
        let middle = (low + high) / 2u;
        if (stage_starts(middle) <= index) {
            low = middle;
        } else {
            high = middle;
        }
    }
    return low;
}

fn nurbs_reflection(v: vec3<f32>, c: f32) -> mat3x3<f32> {
    let k = 2.0 / c;
    return mat3x3<f32>(
        vec3<f32>(1.0, 0.0, 0.0) - (k * v.x) * v,
        vec3<f32>(0.0, 1.0, 0.0) - (k * v.y) * v,
        vec3<f32>(0.0, 0.0, 1.0) - (k * v.z) * v,
    );
}

// One step of the double reflection method as a matrix, as
// `double_reflection`.
fn nurbs_double_reflection(
    position: vec3<f32>,
    tangent: vec3<f32>,
    next_position: vec3<f32>,
    next_tangent: vec3<f32>,
) -> mat3x3<f32> {
    let v1 = next_position - position;
    let c1 = dot(v1, v1);
    if (c1 <= 1e-8) {
        return mat3x3<f32>(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0));
    }
    let first = nurbs_reflection(v1, c1);
    let v2 = next_tangent - first * tangent;
    let c2 = dot(v2, v2);
    if (c2 <= 1e-8) {
        return first;
    }
    return nurbs_reflection(v2, c2) * first;
}

// A segment on its own, as `NurbsPiece::new`: measured relative to its
// origin, where small numbers keep their precision.
fn nurbs_piece(index: u32) -> NurbsPiece {
    let curve = nurbs_curve_of(index);
    let span = stage_spans(index);
    let origin = nurbs_origin(curve, span);
    let local = CubicNurbsSegmentCache(
        nurbs_basis(curve, span, origin),
        0.0,
        0.0,
        vec2<f32>(0.0),
        vec2<f32>(0.0),
    );
    var transport = mat3x3<f32>(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0));
    var position = curve_point(local, 0.0).position;
    var tangent = curve_tangent(local, 0.0);
    let start_tangent = tangent;
    for (var step = 1u; step <= 31u; step++) {
        let u = f32(step) / 31.0;
        let next_position = curve_point(local, u).position;
        let next_tangent = curve_tangent(local, u);
        transport = nurbs_double_reflection(position, tangent, next_position, next_tangent) * transport;
        position = next_position;
        tangent = next_tangent;
    }
    // The shape back where it is, as `translated`.
    let m = local.monomial_basis;
    let basis = mat4x4<f32>(
        m[0] + vec4<f32>(origin * m[0].w, 0.0),
        m[1] + vec4<f32>(origin * m[1].w, 0.0),
        m[2] + vec4<f32>(origin * m[2].w, 0.0),
        m[3] + vec4<f32>(origin * m[3].w, 0.0),
    );
    return NurbsPiece(
        basis,
        start_tangent,
        curve_length_until(local, 1.0),
        tangent,
        transport[0],
        transport[1],
        transport[2],
    );
}

// `normal` turned with the tangent, as `turn`.
fn nurbs_turn(from_: vec3<f32>, to: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let turned = curve_rotate_arc(from_, to, normal);
    let along = turned - to * dot(turned, to);
    let len = length(along);
    if (len > 1e-30) {
        return along / len;
    }
    return curve_reference_normal(to);
}

// A segment joined with the pieces of its curve before it, as `join`.
fn nurbs_segment(index: u32) -> CubicNurbsSegmentCache {
    let first = stage_starts(nurbs_curve_of(index));
    var tangent = stage_pieces(first).start_tangent;
    var normal = curve_reference_normal(tangent);
    var total = 0.0;
    var start_normal = vec2<f32>(0.0);
    for (var at = first; at <= index; at++) {
        let piece = stage_pieces(at);
        normal = nurbs_turn(tangent, piece.start_tangent, normal);
        start_normal = nurbs_encode(normal, piece.start_tangent);
        normal = mat3x3<f32>(piece.transport_x, piece.transport_y, piece.transport_z) * normal;
        tangent = piece.end_tangent;
        total += piece.length;
    }
    let piece = stage_pieces(index);
    return CubicNurbsSegmentCache(
        piece.monomial_basis,
        piece.length,
        total,
        start_normal,
        nurbs_encode(normal, tangent),
    );
}
