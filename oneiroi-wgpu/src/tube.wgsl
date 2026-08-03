struct EvaluatedFrame {
    position: vec3<f32>,
    tangent: vec3<f32>,
    normal: vec3<f32>,
    binormal: vec3<f32>,
}

@group(0) @binding(0)
var<storage, read> evaluated_frames: array<EvaluatedFrame>;

struct Uniforms {
    view_projection: mat4x4<f32>,
    tube_radius: f32,
    radial_segments: u32,
    _pad0: u32,
    _pad1: u32,
}
@group(0) @binding(1)
var<uniform> config: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) segment_id: u32,
};

const PI: f32 = 3.14159265359;

@vertex
fn vs_main(
    @builtin(vertex_index) vert_idx: u32,
    @builtin(instance_index) inst_idx: u32
) -> VertexOutput {
    var out: VertexOutput;
    let total_subdivisions = config.radial_segments;
    let ring_quad_vertex = vert_idx % 6u;
    let segment_pixel_idx = vert_idx / 6u;

    var local_vertex_id = segment_pixel_idx;
    var use_next_frame = 0u;

    if (ring_quad_vertex == 1u || ring_quad_vertex == 4u || ring_quad_vertex == 5u) {
        use_next_frame = 1u;
    }
    if (ring_quad_vertex == 2u || ring_quad_vertex == 3u || ring_quad_vertex == 5u) {
        local_vertex_id = segment_pixel_idx + 1u;
    }

    let target_frame_idx = inst_idx + use_next_frame;
    let frame = evaluated_frames[target_frame_idx];

    let angle = (f32(local_vertex_id) / f32(total_subdivisions)) * 2.0 * PI;
    let cos_a = cos(angle);
    let sin_a = sin(angle);


    let local_normal = (frame.normal * cos_a) + (frame.binormal * sin_a);
    let world_position = frame.position + (local_normal * config.tube_radius);
    
    
    out.clip_position = config.view_projection * vec4<f32>(world_position, 1.0);
    out.normal = normalize(local_normal);
    out.uv = vec2<f32>(f32(local_vertex_id) / f32(total_subdivisions), f32(target_frame_idx) * 0.1);
    out.segment_id = inst_idx / 31u;

    return out;
}

fn hash_color(id: u32) -> vec3<f32> {
    let x = (id * 1664525u + 1013904223u) & 0xFFFFFFu;
    let r = f32((x >> 16u) & 0xFFu) / 255.0;
    let g = f32((x >> 8u) & 0xFFu) / 255.0;
    let b = f32(x & 0xFFu) / 255.0;
    return vec3<f32>(r, g, b);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let diff = max(dot(in.normal, light_dir), 0.0);
    let ambient = 0.15;
    
    let base_color = hash_color(in.segment_id); 
    let final_color = base_color * (diff + ambient);
    
    return vec4<f32>(final_color, 1.0);
}