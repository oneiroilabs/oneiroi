fn surface_point(piece: TubePatch, uv: vec2<f32>) -> SurfacePoint {
    let angle = uv.y * 6.2831855;
    if (piece.kind != 0u) {
        let cap = sweep_cap(piece.segment, piece.kind, uv.x);
        let out = cap.frame.normal * cos(angle) + cap.frame.binormal * sin(angle);
        return SurfacePoint(cap.frame.position + out * piece.radius * cap.scale, cap.facing);
    }
    let frame = curve_frame(piece.segment, uv.x);
    let normal = frame.normal * cos(angle) + frame.binormal * sin(angle);
    let offset = normal * piece.radius;
    let along = mix(
        sweep_cut(offset, frame.tangent, piece.miter_start),
        sweep_cut(offset, frame.tangent, piece.miter_end),
        uv.x,
    );
    return SurfacePoint(frame.position + offset + frame.tangent * along, normal);
}
