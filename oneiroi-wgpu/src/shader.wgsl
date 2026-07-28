struct CubicNurbsSegmentCache {
    coeff_col0: vec4<f32>,
    coeff_col1: vec4<f32>,
    coeff_col2: vec4<f32>,
    coeff_col3: vec4<f32>,
    
    t_start: f32,
    t_end: f32,
    length: f32,
    cumulative_length: f32,
}

@group(0) @binding(0)
var<storage, read> segments: array<CubicNurbsSegmentCache>;

// Output: Array of positions and tangents bundled as vec4s
struct OutputSample {
    position: vec4<f32>,
    tangent: vec4<f32>,
}                   

@group(1) @binding(0)
var<storage, read_write> output_samples: array<OutputSample>;

// 5-Point Gauss–Legendre constants
const GAUSS_NODES = array<f32, 5>(0.0, -0.5384693, 0.5384693, -0.90617985, 0.90617985);
const GAUSS_WEIGHTS = array<f32, 5>(0.5688889, 0.47862867, 0.47862867, 0.23692689, 0.23692689);

fn evaluate_tangent(seg_idx: u32, t: f32) -> OutputSample {
    let segment = segments[seg_idx];
    let dt = segment.t_end - segment.t_start;
    let u = (t - segment.t_start) / dt;
    


    let p_hom = fma(
        fma(fma(segment.coeff_col3, vec4<f32>(u), segment.coeff_col2), vec4<f32>(u), segment.coeff_col1),
        vec4<f32>(u),
        segment.coeff_col0
    );

    let dp_du = fma(
        fma(segment.coeff_col3 * 3.0, vec4<f32>(u), segment.coeff_col2 * 2.0),
        vec4<f32>(u),
        segment.coeff_col1
    );

    let inv_dt = 1.0 / dt;
    let a_xyz = p_hom.xyz;
    let w = p_hom.w;
    let da = dp_du.xyz * inv_dt;
    let dw = dp_du.w * inv_dt;

    let c_pos = a_xyz / w;
    let c_vel = (da - dw * c_pos) / w;

    var out: OutputSample;
    out.position = vec4<f32>(c_pos, 1.0);
    out.tangent = vec4<f32>(c_vel, 0.0);
    return out;
}

fn length_inside_segment(seg_idx: u32, t_cutoff: f32) -> f32 {
    let segment = segments[seg_idx];
    let dt = t_cutoff - segment.t_start;
    if (dt <= 1e-6) { return 0.0; }

    var span_integral = 0.0;
    for (var i = 0u; i < 5u; i++) {
        let t = 0.5 * (dt * GAUSS_NODES[i] + (t_cutoff + segment.t_start));
        let sample_eval = evaluate_tangent(seg_idx, t);
        span_integral += GAUSS_WEIGHTS[i] * length(sample_eval.tangent.xyz);
    }
    return span_integral * 0.5 * dt;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let count = arrayLength(&output_samples);
    if (idx >= count) { return; }

    let num_segments = arrayLength(&segments);
    let total_length = segments[num_segments - 1u].cumulative_length;

    // Handle edges cleanly
    if (idx == 0u) {
        output_samples[idx] = evaluate_tangent(0u, segments[0u].t_start);
        return;
    }
    if (idx == count - 1u) {
        output_samples[idx] = evaluate_tangent(num_segments - 1u, segments[num_segments - 1u].t_end);
        return;
    }

    let step = total_length / f32(count - 1u);
    let target_s = f32(idx) * step;

    // Binary search for segment matching target_s (analogous to partition_point)
    var low = 0u;
    var high = num_segments;
    while (low < high) {
        let mid = low + (high - low) / 2u;
        if (segments[mid].cumulative_length < target_s) {
            low = mid + 1u;
        } else {
            high = mid;
        }
    }
    let current_seg_idx = clamp(low, 0u, num_segments - 1u);
    let segment = segments[current_seg_idx];

    var seg_start_len = 0.0;
    if (current_seg_idx > 0u) {
        seg_start_len = segments[current_seg_idx - 1u].cumulative_length;
    }
    let s_local = target_s - seg_start_len;

    // Initial root guess
    var t = segment.t_start + (s_local / segment.length) * (segment.t_end - segment.t_start);

    // Newton-Raphson iteration loop
    for (var iter = 0; iter < 2; iter++) {
        let current_s_local = length_inside_segment(current_seg_idx, t);
        let sample_eval = evaluate_tangent(current_seg_idx, t);
        let speed = length(sample_eval.tangent.xyz);

        if (speed < 1e-5) { break; }

        let delta_t = (current_s_local - s_local) / speed;
        t -= delta_t;
        t = clamp(t, segment.t_start, segment.t_end);

        if (abs(delta_t) < 1e-5) { break; }
    }

    output_samples[idx] = evaluate_tangent(current_seg_idx, t);
}
