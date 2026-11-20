fn surface_point(piece: TubePatch, uv: vec2<f32>) -> SurfacePoint {
    let angle = uv.y * 6.2831855;
    if (piece.kind != 0u) {
        let cap = sweep_cap(piece.segment, piece.kind, uv.x);
        let out = cap.frame.normal * cos(angle) + cap.frame.binormal * sin(angle);
        return SurfacePoint(cap.frame.position + out * piece.radius * cap.scale * piece.scale.x, cap.facing);
    }
    let t = mix(piece.span.x, piece.span.y, uv.x);
    let frame = curve_frame(piece.segment, t);
    let speed = curve_speed(piece.segment, t) * (piece.span.y - piece.span.x);
    let scale = sweep_scale(piece.scale, uv.x);
    let outwards = frame.normal * cos(angle) + frame.binormal * sin(angle);
    let offset = outwards * piece.radius * scale.x;
    let along = mix(
        sweep_cut(offset, frame.tangent, piece.miter_start),
        sweep_cut(offset, frame.tangent, piece.miter_end),
        uv.x,
    );
    let normal = sweep_leaning(outwards, outwards * piece.radius, frame.tangent, speed, scale.y);
    return SurfacePoint(frame.position + offset + frame.tangent * along, normal);
}
