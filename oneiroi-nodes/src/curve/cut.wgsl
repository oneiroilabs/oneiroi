
// How far a point `offset` across the curve, where it heads `tangent`, is
// moved along it to lie on the plane through the curve with the normal
// `miter`: where it meets its neighbour at a corner. Nothing without one.
fn sweep_cut(offset: vec3<f32>, tangent: vec3<f32>, miter: vec4<f32>) -> f32 {
    let facing = dot(tangent, miter.xyz);
    if (abs(facing) < 1e-3) {
        return 0.0;
    }
    return -dot(offset, miter.xyz) / facing;
}

struct SweepCap {
    frame: CurveFrame,
    scale: f32,
    facing: vec3<f32>,
}

// Where a cap is and how far out its point at `u` is, the same as `cap` on
// the CPU.
fn sweep_cap(segment: CubicNurbsSegmentCache, kind: u32, u: f32) -> SweepCap {
    let at_end = kind == 2u;
    let frame = curve_frame(segment, select(0.0, 1.0, at_end));
    var scale = 0.0;
    if (kind == 1u) {
        scale = u;
    } else if (kind == 2u) {
        scale = 1.0 - u;
    }
    return SweepCap(frame, scale, select(-frame.tangent, frame.tangent, at_end));
}
