// Turning, scaling and moving curves, the same as
// `CubicNurbsSegmentCache::placed`. Needs `curve.wgsl`.

struct Placement {
    // Its columns are where x, y and z of the shape go.
    rotation: mat3x3<f32>,
    scale: f32,
    offset: vec3<f32>,
}

// The inverse of `curve_decode_normal`.
fn curve_encode_normal(normal: vec3<f32>, tangent: vec3<f32>) -> vec2<f32> {
    let n = curve_reference_normal(tangent);
    let b = curve_normalize_or_zero(cross(tangent, n));
    return vec2<f32>(dot(normal, n), dot(normal, b));
}

fn curve_placed(segment: CubicNurbsSegmentCache, place: Placement) -> CubicNurbsSegmentCache {
    var placed = segment;
    let m = segment.monomial_basis;
    for (var i = 0u; i < 4u; i++) {
        let column = m[i];
        placed.monomial_basis[i] = vec4<f32>(
            place.rotation * column.xyz * place.scale + place.offset * column.w,
            column.w,
        );
    }
    placed.length = segment.length * place.scale;
    placed.cumulative_length = segment.cumulative_length * place.scale;

    let start = curve_tangent(segment, 0.0);
    let end = curve_tangent(segment, 1.0);
    placed.rmf_start_normal = curve_encode_normal(
        place.rotation * curve_decode_normal(segment.rmf_start_normal, start),
        place.rotation * start,
    );
    placed.rmf_end_normal = curve_encode_normal(
        place.rotation * curve_decode_normal(segment.rmf_end_normal, end),
        place.rotation * end,
    );
    return placed;
}
