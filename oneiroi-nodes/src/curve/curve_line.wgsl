fn line_point(piece: CubicNurbsSegmentCache, u: f32) -> vec3<f32> {
    return curve_point(piece, u).position;
}
