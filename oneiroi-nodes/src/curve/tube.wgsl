fn surface_point(piece: TubePatch, uv: vec2<f32>) -> SurfacePoint {
    let frame = curve_frame(piece.segment, uv.x);
    let angle = uv.y * 6.2831855;
    let normal = frame.normal * cos(angle) + frame.binormal * sin(angle);
    return SurfacePoint(frame.position + normal * piece.radius, normal);
}
