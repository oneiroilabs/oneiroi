fn surface_point(piece: SweptPatch, uv: vec2<f32>) -> SurfacePoint {
    let frame = curve_frame(piece.spine, uv.x);
    let outline = curve_point(piece.profile, uv.y).position;
    let tangent = curve_tangent(piece.profile, uv.y).xy;

    // To the right of the direction the profile runs in.
    var outwards = vec2<f32>(0.0);
    let len = length(tangent);
    if (len > 1e-30) {
        outwards = vec2<f32>(tangent.y, -tangent.x) / len;
    }
    return SurfacePoint(
        frame.position + frame.normal * outline.x + frame.binormal * outline.y,
        frame.normal * outwards.x + frame.binormal * outwards.y,
    );
}
