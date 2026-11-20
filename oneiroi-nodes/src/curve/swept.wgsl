fn surface_point(piece: SweptPatch, uv: vec2<f32>) -> SurfacePoint {
    let outline = curve_point(piece.profile, uv.y).position;
    if (piece.kind != 0u) {
        let cap = sweep_cap(piece.spine, piece.kind, uv.x);
        let across = cap.frame.normal * outline.x + cap.frame.binormal * outline.y;
        return SurfacePoint(cap.frame.position + across * cap.scale * piece.scale.x, cap.facing);
    }
    let t = mix(piece.span.x, piece.span.y, uv.x);
    let frame = curve_frame(piece.spine, t);
    let speed = curve_speed(piece.spine, t) * (piece.span.y - piece.span.x);
    let scale = sweep_scale(piece.scale, uv.x);
    let tangent = curve_tangent(piece.profile, uv.y).xy;

    // To the right of the direction the profile runs in.
    var flat_out = vec2<f32>(0.0);
    let len = length(tangent);
    if (len > 1e-30) {
        flat_out = vec2<f32>(tangent.y, -tangent.x) / len;
    }
    let outwards = frame.normal * flat_out.x + frame.binormal * flat_out.y;
    let across = frame.normal * outline.x + frame.binormal * outline.y;
    let offset = across * scale.x;
    let along = mix(
        sweep_cut(offset, frame.tangent, piece.miter_start),
        sweep_cut(offset, frame.tangent, piece.miter_end),
        uv.x,
    );
    return SurfacePoint(
        frame.position + offset + frame.tangent * along,
        sweep_leaning(outwards, across, frame.tangent, speed, scale.y),
    );
}
