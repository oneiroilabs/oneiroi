fn surface_point(piece: SweptPatch, uv: vec2<f32>) -> SurfacePoint {
    let outline = curve_point(piece.profile, uv.y).position;
    if (piece.kind != 0u) {
        let cap = sweep_cap(piece.spine, piece.kind, uv.x);
        let across = cap.frame.normal * outline.x + cap.frame.binormal * outline.y;
        return SurfacePoint(cap.frame.position + across * cap.scale, cap.facing);
    }
    let frame = curve_frame(piece.spine, uv.x);
    let tangent = curve_tangent(piece.profile, uv.y).xy;

    // To the right of the direction the profile runs in.
    var outwards = vec2<f32>(0.0);
    let len = length(tangent);
    if (len > 1e-30) {
        outwards = vec2<f32>(tangent.y, -tangent.x) / len;
    }
    let offset = frame.normal * outline.x + frame.binormal * outline.y;
    let along = mix(
        sweep_cut(offset, frame.tangent, piece.miter_start),
        sweep_cut(offset, frame.tangent, piece.miter_end),
        uv.x,
    );
    return SurfacePoint(
        frame.position + offset + frame.tangent * along,
        frame.normal * outwards.x + frame.binormal * outwards.y,
    );
}
