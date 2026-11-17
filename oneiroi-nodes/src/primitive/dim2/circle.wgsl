fn line_point(piece: Circle, u: f32) -> vec3<f32> {
    let angle = u * 6.2831855;
    return vec3<f32>(cos(angle), sin(angle), 0.0) * piece.radius;
}
