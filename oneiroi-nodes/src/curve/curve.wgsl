// Evaluates the segments of a curve. This is the same computation as the
// methods of `CubicNurbsSegmentCache` and has to stay that way.

struct CurvePoint {
    position: vec3<f32>,
    velocity: vec3<f32>,
}

struct CurveFrame {
    position: vec3<f32>,
    tangent: vec3<f32>,
    normal: vec3<f32>,
    binormal: vec3<f32>,
}

fn curve_normalize_or_zero(v: vec3<f32>) -> vec3<f32> {
    let len = length(v);
    if (len > 1e-30) {
        return v / len;
    }
    return vec3<f32>(0.0);
}

fn curve_point(segment: CubicNurbsSegmentCache, u: f32) -> CurvePoint {
    let m = segment.monomial_basis;
    let us = vec4<f32>(u);

    let hom = fma(fma(fma(m[0], us, m[1]), us, m[2]), us, m[3]);
    let position = hom.xyz / hom.w;

    let derivative = fma(fma(m[0], vec4<f32>(u * 3.0), m[1] * 2.0), us, m[2]);
    let velocity = (derivative.xyz - derivative.w * position) / hom.w;
    return CurvePoint(position, velocity);
}

fn curve_tangent(segment: CubicNurbsSegmentCache, u: f32) -> vec3<f32> {
    let velocity = curve_point(segment, u).velocity;
    let len = length(velocity);
    if (len > 1e-30) {
        return velocity / len;
    }

    // The curve stands still here, e.g. at a clamped end. Look a bit ahead.
    var nearby = u - 0.001;
    if (u + 0.001 <= 1.0) {
        nearby = u + 0.001;
    }
    return curve_normalize_or_zero(curve_point(segment, nearby).velocity);
}

// The first of the two vectors a normal is stored relative to.
fn curve_reference_normal(tangent: vec3<f32>) -> vec3<f32> {
    let a = abs(tangent);
    var reference = vec3<f32>(0.0, 0.0, 1.0);
    if (a.x < a.y && a.x < a.z) {
        reference = vec3<f32>(1.0, 0.0, 0.0);
    } else if (a.y < a.z) {
        reference = vec3<f32>(0.0, 1.0, 0.0);
    }
    return curve_normalize_or_zero(cross(reference, tangent));
}

fn curve_decode_normal(normal: vec2<f32>, tangent: vec3<f32>) -> vec3<f32> {
    let n = curve_reference_normal(tangent);
    let b = curve_normalize_or_zero(cross(tangent, n));
    return n * normal.x + b * normal.y;
}

// Turns `v` by the shortest rotation which takes `from` to `to`.
fn curve_rotate_arc(from_: vec3<f32>, to: vec3<f32>, v: vec3<f32>) -> vec3<f32> {
    let limit = 1.0 - 2.0 * 1.1920929e-7;
    let d = dot(from_, to);
    if (d > limit) {
        return v;
    }

    var q: vec4<f32>;
    if (d < -limit) {
        // Half a turn around anything which is orthogonal.
        let s = select(-1.0, 1.0, from_.z >= 0.0);
        let a = -1.0 / (s + from_.z);
        let b = from_.x * from_.y * a;
        q = vec4<f32>(b, s + from_.y * from_.y * a, -from_.y, 0.0);
    } else {
        q = normalize(vec4<f32>(cross(from_, to), 1.0 + d));
    }

    let axis = q.xyz;
    return v * (q.w * q.w - dot(axis, axis))
        + axis * (dot(v, axis) * 2.0)
        + cross(axis, v) * (q.w * 2.0);
}

fn curve_frame(segment: CubicNurbsSegmentCache, u: f32) -> CurveFrame {
    let position = curve_point(segment, u).position;
    let tangent = curve_tangent(segment, u);

    let start_tangent = curve_tangent(segment, 0.0);
    let end_tangent = curve_tangent(segment, 1.0);
    let start = curve_rotate_arc(
        start_tangent,
        tangent,
        curve_decode_normal(segment.rmf_start_normal, start_tangent),
    );
    let end = curve_rotate_arc(
        end_tangent,
        tangent,
        curve_decode_normal(segment.rmf_end_normal, end_tangent),
    );
    let blended = mix(start, end, u);

    var normal = blended - tangent * dot(blended, tangent);
    let len = length(normal);
    if (len > 1e-30) {
        normal = normal / len;
    } else {
        normal = curve_reference_normal(tangent);
    }
    let binormal = curve_normalize_or_zero(cross(tangent, normal));

    return CurveFrame(position, tangent, normal, binormal);
}

fn curve_speed(segment: CubicNurbsSegmentCache, u: f32) -> f32 {
    return length(curve_point(segment, u).velocity);
}

// The length of the segment between 0 and `u`, by a Gauss-Legendre rule of
// five points. In the same order as on the CPU, to get the same sum.
fn curve_length_until(segment: CubicNurbsSegmentCache, u: f32) -> f32 {
    if (u <= 0.0) {
        return 0.0;
    }
    let half = 0.5 * u;
    var integral = 0.0;
    integral += 0.5688889 * curve_speed(segment, half * (0.0 + 1.0));
    integral += 0.47862867 * curve_speed(segment, half * (-0.5384693 + 1.0));
    integral += 0.47862867 * curve_speed(segment, half * (0.5384693 + 1.0));
    integral += 0.23692689 * curve_speed(segment, half * (-0.90617985 + 1.0));
    integral += 0.23692689 * curve_speed(segment, half * (0.90617985 + 1.0));
    return integral * 0.5 * u;
}

// The `u` at which the segment is `distance` long, by at most eight Newton
// steps, as many as `NEWTON_STEPS`.
fn curve_u_at_distance(segment: CubicNurbsSegmentCache, distance: f32) -> f32 {
    if (segment.length <= 0.0) {
        return 0.0;
    }
    var u = clamp(distance / segment.length, 0.0, 1.0);
    for (var step = 0u; step < 8u; step++) {
        let speed = curve_speed(segment, u);
        if (speed < 1e-5) {
            break;
        }
        let delta = (curve_length_until(segment, u) - distance) / speed;
        u = clamp(u - delta, 0.0, 1.0);
        if (abs(delta) < 1e-5) {
            break;
        }
    }
    return u;
}
