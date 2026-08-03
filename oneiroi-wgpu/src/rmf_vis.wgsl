struct Uniforms {
    view_proj: mat4x4<f32>,
    vector_scale: f32, // Length of the diagnostic vectors (e.g., 0.2)
}
@group(0) @binding(0) var<uniform> config: Uniforms;

struct EvaluatedFrame {
    position: vec3<f32>,
    normal: vec3<f32>,
    binormal: vec3<f32>,
    tangent: vec3<f32>,
}
@group(0) @binding(1) var<storage, read> evaluated_frames: array<EvaluatedFrame>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vert_idx: u32,
    @builtin(instance_index) inst_idx: u32
) -> VertexOutput {
    var out: VertexOutput;

    // Fetch the target frame base data
    let frame = evaluated_frames[inst_idx];
    
    // Each instance draws 3 independent lines (6 vertices total)
    // Vertices: 0-1 (Tangent), 2-3 (Normal), 4-5 (Binormal)
    let line_type = vert_idx / 2u;
    let is_tip = vert_idx % 2u; // 0 = base at curve position, 1 = tip of vector

    var world_pos = frame.position;
    var line_color = vec4<f32>(0.0, 0.0, 0.0, 1.0);

    if (line_type == 0u) {
        // Tangent Vector (Red)
        line_color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
        if (is_tip == 1u) { world_pos += frame.tangent * config.vector_scale; }
    } else if (line_type == 1u) {
        // Normal Vector (Green)
        line_color = vec4<f32>(0.0, 1.0, 0.0, 1.0);
        if (is_tip == 1u) { world_pos += frame.normal * config.vector_scale; }
    } else {
        // Binormal Vector (Blue)
        line_color = vec4<f32>(0.0, 0.0, 1.0, 1.0);
        if (is_tip == 1u) { world_pos += frame.binormal * config.vector_scale; }
    }

    out.clip_position = config.view_proj * vec4<f32>(world_pos, 1.0);
    out.color = line_color;
    return out;
}

@fragment
fn fr_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
