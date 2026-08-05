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
    @location(3) world_position: vec3<f32>,
    //@location(4) world_normal: vec3<f32>,
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
    out.segment_id = inst_idx / 32u;
    out.world_position = world_position;
    //out.world_normal

    return out;
}

fn hash_color(id: u32) -> vec3<f32> {
    let x = (id * 1664525u + 1013904223u) & 0xFFFFFFu;
    let r = f32((x >> 16u) & 0xFFu) / 255.0;
    let g = f32((x >> 8u) & 0xFFu) / 255.0;
    let b = f32(x & 0xFFu) / 255.0;
    return vec3<f32>(r, g, b);
}

struct TriplanarVoronoiParams {
    voronoi_scale: f32,
    voronoi_jitter: f32,
    triplanar_sharpness: f32,
    color_a: vec3<f32>,
    color_b: vec3<f32>,
}

fn hash3(p: vec3<f32>) -> vec3<f32> {
    var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xxy + p3.yzz) * p3.zyx);
}

fn evaluate_voronoi_2d(p: vec2<f32>, jitter: f32) -> vec2<f32> {
    let p_cell = floor(p);
    let p_fract = fract(p);

    var min_dist = 8.0;
    var cell_id = 0.0;

    for (var y: i32 = -1; y <= 1; y++) {
        for (var x: i32 = -1; x <= 1; x++) {
            let neighbor = vec2<f32>(vec2<i32>(x, y));
            
            let cell_world_pos = p_cell + neighbor;
            let rand_val = hash3(vec3<f32>(cell_world_pos, 0.0)); 
            
            let cell_offset = neighbor + rand_val.xy * jitter;
            let dist = distance(cell_offset, p_fract);

            if (dist < min_dist) {
                min_dist = dist;
                cell_id = rand_val.z;
            }
        }
    }
    return vec2<f32>(min_dist, cell_id);
}

fn generate_triplanar_voronoi(
    world_pos: vec3<f32>, 
    world_normal: vec3<f32>, 
    params: TriplanarVoronoiParams
) -> vec4<f32> { 
    var weights = abs(world_normal);
    weights = pow(weights, vec3<f32>(params.triplanar_sharpness));
    weights = weights / (weights.x + weights.y + weights.z);

    let p = world_pos * params.voronoi_scale;


    let voronoi_x = evaluate_voronoi_2d(p.yz, params.voronoi_jitter);
    let voronoi_y = evaluate_voronoi_2d(p.xz, params.voronoi_jitter);
    let voronoi_z = evaluate_voronoi_2d(p.xy, params.voronoi_jitter);

    let blended_dist = (voronoi_x.x * weights.x) + 
                       (voronoi_y.x * weights.y) + 
                       (voronoi_z.x * weights.z);

    let blended_id = (voronoi_x.y * weights.x) + 
                     (voronoi_y.y * weights.y) + 
                     (voronoi_z.y * weights.z);

    let base_color = mix(params.color_a, params.color_b, vec3<f32>(blended_dist));
    
    let final_color = base_color * (0.8 + 0.4 * blended_id);

    return vec4<f32>(final_color, blended_dist);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Should be fetched form bindless things.
    var material_params=TriplanarVoronoiParams();
    material_params.voronoi_scale=10;
    material_params.voronoi_jitter=1.0;
    material_params.triplanar_sharpness=4.0;
    material_params.color_a=vec3<f32>(1.0,0.0,0.0);
    material_params.color_b=vec3<f32>(0.0,0.0,1.0);


    let voronoi_sample = generate_triplanar_voronoi(
        input.world_position, 
        normalize(input.normal), 
        material_params
    );

    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let diff = max(dot(input.normal, light_dir), 0.0);
    let ambient = 0.15;
    
    let base_color = voronoi_sample.xyz; 
    let final_color = base_color * (diff + ambient);
    
    return vec4<f32>(final_color, 1.0);

}

//@fragment
//fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
//    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
//    let diff = max(dot(in.normal, light_dir), 0.0);
//    let ambient = 0.15;
//    
//    let base_color = hash_color(in.segment_id); 
//    let final_color = base_color * (diff + ambient);
//    
//    return vec4<f32>(final_color, 1.0);
//}