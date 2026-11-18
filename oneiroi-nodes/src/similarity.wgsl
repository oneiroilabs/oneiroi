// Similarities and the quaternions they turn with, the same as
// `Similarity` and `quat_from_basis` on the CPU. Needs `Similarity`.

fn quat_mul(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(a.w * b.xyz + b.w * a.xyz + cross(a.xyz, b.xyz), a.w * b.w - dot(a.xyz, b.xyz));
}

fn quat_rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

// `inner` first and then `outer`.
fn similarity_then(outer: Similarity, inner: Similarity) -> Similarity {
    return Similarity(
        normalize(quat_mul(outer.rotation, inner.rotation)),
        quat_rotate(outer.rotation, inner.offset) * outer.scale + outer.offset,
        outer.scale * inner.scale,
    );
}

fn similarity_point(t: Similarity, p: vec3<f32>) -> vec3<f32> {
    return quat_rotate(t.rotation, p) * t.scale + t.offset;
}

// The rotation which takes x, y and z to the columns `x`, `y` and `z`.
fn quat_from_basis(x: vec3<f32>, y: vec3<f32>, z: vec3<f32>) -> vec4<f32> {
    let trace = x.x + y.y + z.z;
    var q: vec4<f32>;
    if (trace > 0.0) {
        let s = 0.5 / sqrt(trace + 1.0);
        q = vec4<f32>((y.z - z.y) * s, (z.x - x.z) * s, (x.y - y.x) * s, 0.25 / s);
    } else if (x.x > y.y && x.x > z.z) {
        let s = 2.0 * sqrt(max(1.0 + x.x - y.y - z.z, 0.0));
        q = vec4<f32>(0.25 * s, (y.x + x.y) / s, (z.x + x.z) / s, (y.z - z.y) / s);
    } else if (y.y > z.z) {
        let s = 2.0 * sqrt(max(1.0 + y.y - x.x - z.z, 0.0));
        q = vec4<f32>((y.x + x.y) / s, 0.25 * s, (z.y + y.z) / s, (z.x - x.z) / s);
    } else {
        let s = 2.0 * sqrt(max(1.0 + z.z - x.x - y.y, 0.0));
        q = vec4<f32>((z.x + x.z) / s, (z.y + y.z) / s, 0.25 * s, (x.y - y.x) / s);
    }
    return normalize(q);
}

// Turning by `degrees` around x, then y, then z, seen from the turned
// axes: the same as `quat_from_degrees` on the CPU.
fn quat_from_degrees(degrees: vec3<f32>) -> vec4<f32> {
    let half = radians(degrees) * 0.5;
    let qx = vec4<f32>(sin(half.x), 0.0, 0.0, cos(half.x));
    let qy = vec4<f32>(0.0, sin(half.y), 0.0, cos(half.y));
    let qz = vec4<f32>(0.0, 0.0, sin(half.z), cos(half.z));
    return normalize(quat_mul(quat_mul(qx, qy), qz));
}
