struct NurbsGpuSegmentCache {
    matrix: mat4x4<f32>,
    t_start: f32,
    t_end: f32,
    start_point_idx: u32,
    _pad0: u32,
}

@group(0) @binding(0) var<storage, read> global_points: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read> segments: array<GpuMinimalSegment>;
@group(0) @binding(2) var<uniform> camera_view_proj: mat4x4<f32>;

fn find_segment_index(t: f32, num_segments: u32) -> u32 {
    let last_idx = num_segments - 1u;
    for (var i = 0u; i < num_segments; i = i + 1u) {
        if (t >= segments[i].t_start && t <= segments[i].t_end) {
            return i;
        }
    }
    return last_idx;
}

@mesh @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let sample_count = 64u;
    let thread_idx = global_id.x;
    if (thread_idx >= sample_count) { return; }

    let num_segments = arrayLength(&segments);
    let t_start = segments[0u].t_start;
    let t_end = segments[num_segments - 1u].t_end;
    
    let progress = f32(thread_idx) / f32(sample_count - 1u);
    let t = t_start + progress * (t_end - t_start);

    let seg_idx = find_segment_index(t, num_segments);
    let segment = segments[seg_idx];

    let dt = segment.t_end - segment.t_start;
    var u = 0.0;
    if (dt > 1e-6) { u = (t - segment.t_start) / dt; }

    // Reconstruct our sliding window indices from the single lean base index
    let base_idx = segment.start_point_idx;
    let ph0 = global_points[base_idx];
    let ph1 = global_points[base_idx + 1u];
    let ph2 = global_points[base_idx + 2u];
    let ph3 = global_points[base_idx + 3u];

    // Compute polynomial coefficients via Marsden Matrix
    var c: array<vec4<f32>, 4>;
    for (var j = 0u; j < 4u; j = j + 1u) {
        let row = segment.matrix[j];
        c[j] = ph0 * row.x + ph1 * row.y + ph2 * row.z + ph3 * row.w;
    }

    let horner_eval = c[0u] + u * (c[1u] + u * (c[2u] + u * c[3u]));
    let world_pos = vec3<f32>(horner_eval.xyz / horner_eval.w);

    SetMeshOutputs(64u, 63u);
    output_vertices[thread_idx].position = camera_view_proj * vec4<f32>(world_pos, 1.0);
    output_vertices[thread_idx].color = vec3<f32>(0.2, 0.8, 0.4); // Standard green line

    if (thread_idx < 63u) {
        output_indices[thread_idx] = vec2<u32>(thread_idx, thread_idx + 1u);
    }
}
