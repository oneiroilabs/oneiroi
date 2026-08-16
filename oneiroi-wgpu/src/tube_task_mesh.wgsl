enable wgpu_mesh_shader;

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

//struct VertexOutput {
 //   @location(0) normal: vec3<f32>,
//    @location(1) uv: vec2<f32>,
//    @location(2) @interpolate(flat) segment_id: u32,
//    @location(3) world_position: vec3<f32>,
//};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
}

struct PrimitiveOutput {
    @builtin(triangle_indices) indices: vec3<u32>,
    @builtin(cull_primitive) cull: bool,
    @per_primitive @location(1) colorMask: vec4<f32>,
}

struct MeshOutput {
    @builtin(vertices) vertices: array<VertexOutput, 3>,
    @builtin(primitives) primitives: array<PrimitiveOutput, 1>,
    @builtin(vertex_count) vertex_count: u32,
    @builtin(primitive_count) primitive_count: u32,
}

var<workgroup> mesh_output: MeshOutput;

const PI: f32 = 3.14159265359;

@mesh(mesh_output)
@workgroup_size(16, 1, 1) 
fn ms_main(
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let inst_idx = workgroup_id.x;
    let total_subdivisions = config.radial_segments;
    let local_vertex_id = local_id.x;

    let num_vertices = total_subdivisions * 2u;
    let num_triangles = total_subdivisions * 2u;
    mesh_output.vertex_count  = num_vertices;
    mesh_output.primitive_count = num_triangles;

    let angle = (f32(local_vertex_id) / f32(total_subdivisions)) * 2.0 * PI;
    let cos_a = cos(angle);
    let sin_a = sin(angle);

    let frame_curr = evaluated_frames[inst_idx];
    let normal_curr = (frame_curr.normal * cos_a) + (frame_curr.binormal * sin_a);
    let pos_curr = frame_curr.position + (normal_curr * config.tube_radius);
    let v_idx_curr = local_vertex_id;

    let frame_next = evaluated_frames[inst_idx + 1u];
    let normal_next = (frame_next.normal * cos_a) + (frame_next.binormal * sin_a);
    let pos_next = frame_next.position + (normal_next * config.tube_radius);
    let v_idx_next = local_vertex_id + total_subdivisions;

    let clip_pos_curr = config.view_projection * vec4<f32>(pos_curr, 1.0);
    //mesh_output.vertices[v_idx_curr].position = clip_pos_curr;
    mesh_output.vertices[v_idx_curr] = VertexOutput(vec4<f32>(pos_curr,1.0),normalize(normal_curr));
    // = VertexOutput(
    //    normalize(normal_curr),
    //    vec2<f32>(f32(local_vertex_id) / f32(total_subdivisions), f32(inst_idx) * 0.1),
    //    inst_idx / 32u,
    //    pos_curr
    //);

    // Write next ring vertex out
    let clip_pos_next = config.view_projection * vec4<f32>(pos_next, 1.0);
    //mesh_output.vertices[v_idx_next].position = clip_pos_next;
    mesh_output.vertices[v_idx_next]= VertexOutput(vec4<f32>(pos_next,1.0),normalize(normal_next));;
    // = VertexOutput(
    //    normalize(normal_next),
    //    vec2<f32>(f32(local_vertex_id) / f32(total_subdivisions), f32(inst_idx + 1u) * 0.1),
    //    inst_idx / 32u,
    //    pos_next
    //);

    let next_local_id = (local_vertex_id + 1u) % total_subdivisions;
    
    let i0 = local_vertex_id;
    let i1 = next_local_id;
    let i2 = local_vertex_id + total_subdivisions;
    let i3 = next_local_id + total_subdivisions;

    let tri_idx_base = local_vertex_id * 2u;
    mesh_output.primitives[tri_idx_base].indices     = vec3<u32>(i0, i2, i1);
    mesh_output.primitives[tri_idx_base + 1u].indices = vec3<u32>(i1, i2, i3);
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

fn generate_triplanar_voronoi(world_pos: vec4<f32>, world_normal: vec3<f32>, params: TriplanarVoronoiParams) -> vec4<f32> { 
    var weights = abs(world_normal);
    weights = pow(weights, vec3<f32>(params.triplanar_sharpness));
    weights = weights / (weights.x + weights.y + weights.z);

    let p = world_pos * params.voronoi_scale;
    let voronoi_x = evaluate_voronoi_2d(p.yz, params.voronoi_jitter);
    let voronoi_y = evaluate_voronoi_2d(p.xz, params.voronoi_jitter);
    let voronoi_z = evaluate_voronoi_2d(p.xy, params.voronoi_jitter);

    let blended_dist = (voronoi_x.x * weights.x) + (voronoi_y.x * weights.y) + (voronoi_z.x * weights.z);
    let blended_id = (voronoi_x.y * weights.x) + (voronoi_y.y * weights.y) + (voronoi_z.y * weights.z);
    let base_color = mix(params.color_a, params.color_b, vec3<f32>(blended_dist));

    return vec4<f32>(base_color * (0.8 + 0.4 * blended_id), blended_dist);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var material_params = TriplanarVoronoiParams();
    material_params.voronoi_scale = 10.0;
    material_params.voronoi_jitter = 1.0;
    material_params.triplanar_sharpness = 4.0;
    material_params.color_a = vec3<f32>(1.0, 0.0, 0.0);
    material_params.color_b = vec3<f32>(0.0, 0.0, 1.0);

    let voronoi_sample = generate_triplanar_voronoi(input.position, normalize(input.normal), material_params);
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let diff = max(dot(input.normal, light_dir), 0.0);
    let ambient = 0.15;
    
    return vec4<f32>(voronoi_sample.xyz * (diff + ambient), 1.0);
}
