enable wgpu_mesh_shader;

struct CubicNurbsSegmentCache {
    coeffs: mat4x4<f32>,
    t_start: f32,
    t_end: f32,
    length: f32,
    cumulative_length: f32,
    rmf_start_normal: vec3<f32>,
    _pad0: u32,
}

struct EvaluatedFrame {
    position: vec3<f32>,
    tangent: vec3<f32>,
    normal: vec3<f32>,
    binormal: vec3<f32>,
}

struct Uniforms {
    view_projection: mat4x4<f32>,
    tube_radius: f32,
    radial_segments: u32,
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0) var<storage, read> segments: array<CubicNurbsSegmentCache>;
@group(0) @binding(1) var<uniform> config: Uniforms;

// --- DEDIZIERTER TASK PAYLOAD ---
struct TubePayload {
    frames: array<EvaluatedFrame, 32>,
    segment_id: u32,
}

var<task_payload> taskPayload: TubePayload;

const PI: f32 = 3.14159265359;

fn get_double_reflection_matrix(pos_a: vec3<f32>, pos_b: vec3<f32>, tang_a: vec3<f32>, tang_b: vec3<f32>) -> mat3x3<f32> {
    let v1 = pos_b - pos_a;
    let v1_len_sq = dot(v1, v1);
    if (v1_len_sq < 1e-6) { return mat3x3<f32>(vec3<f32>(1.0,0.0,0.0), vec3<f32>(0.0,1.0,0.0), vec3<f32>(0.0,0.0,1.0)); }
    let c1 = 2.0 / v1_len_sq;
    let t_l = tang_a - (c1 * dot(v1, tang_a)) * v1;
    let v2 = tang_b - t_l;
    let v2_len_sq = dot(v2, v2);
    if (v2_len_sq < 1e-6) { 
        return mat3x3<f32>(vec3<f32>(1.0,0.0,0.0)-(c1*v1.x)*v1, vec3<f32>(0.0,1.0,0.0)-(c1*v1.y)*v1, vec3<f32>(0.0,0.0,1.0)-(c1*v1.z)*v1); 
    }
    let c2 = 2.0 / v2_len_sq;
    let r1 = vec3<f32>(1.0,0.0,0.0) - (c1 * v1.x) * v1;
    let r2 = vec3<f32>(0.0,1.0,0.0) - (c1 * v1.y) * v1;
    let r3 = vec3<f32>(0.0,0.0,1.0) - (c1 * v1.z) * v1;
    return mat3x3<f32>(r1 - (c2 * dot(v2, r1)) * v2, r2 - (c2 * dot(v2, r2)) * v2, r3 - (c2 * dot(v2, r3)) * v2);
}

// ==========================================
// 1. TASK SHADER (STUFE 1)
// ==========================================
@task
@payload(taskPayload)
@workgroup_size(32, 1, 1)
fn ts_main(
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) wg_id: vec3<u32>
) -> @builtin(mesh_task_size) vec3<u32> {
    let segment_idx = wg_id.x;
    let lane_id = local_id.x;
    let segment = segments[segment_idx];
    
    // Kurve parallel berechnen
    let u = f32(lane_id) / 31.0;
    let u_splat = vec4<f32>(u);
    
     let p_hom = fma(segment.coeffs[3], u_splat, segment.coeffs[2]);
    let p_hom2 = fma(p_hom, u_splat, segment.coeffs[1]);
    let position_hom = fma(p_hom2, u_splat, segment.coeffs[0]);
    let w = position_hom.w;
    let position = position_hom.xyz / w;
    
    let d3 = segment.coeffs[3] * 3.0;
    let d2 = segment.coeffs[2] * 2.0;
    let dp_du = fma(d3, u_splat, d2);
    let derivative_hom = fma(dp_du, u_splat, segment.coeffs[1]);
    
    let velocity = (derivative_hom.xyz - derivative_hom.w * position) / w;
    let tangent = normalize(velocity);

    // Spalten-Vektoren für RMF-Kette via Subgroups aufbauen
    var local_R = mat3x3<f32>(vec3<f32>(1.0,0.0,0.0), vec3<f32>(0.0,1.0,0.0), vec3<f32>(0.0,0.0,1.0));
    let next_pos = subgroupShuffleDown(position, 1u);
    let next_tangent = subgroupShuffleDown(tangent, 1u);
    if (lane_id < 31u) {
        local_R = get_double_reflection_matrix(position, next_pos, tangent, next_tangent);
    }

    for (var offset = 1u; offset < 32u; offset *= 2u) {
        let spawned_col0 = subgroupShuffleUp(local_R[0], offset);
        let spawned_col1 = subgroupShuffleUp(local_R[1], offset);
        let spawned_col2 = subgroupShuffleUp(local_R[2], offset);
        
        let spawned_R = mat3x3<f32>(vec3<f32>(spawned_col0), vec3<f32>(spawned_col1), vec3<f32>(spawned_col2));
        let accumulated_R = spawned_R * local_R;
        if (lane_id >= offset) { local_R = accumulated_R; }
    }

    let final_col0 = subgroupShuffleUp(local_R[0], 1u);
    let final_col1 = subgroupShuffleUp(local_R[1], 1u);
    let final_col2 = subgroupShuffleUp(local_R[2], 1u);
    
    let final_chain_matrix = mat3x3<f32>(vec3<f32>(final_col0), vec3<f32>(final_col1), vec3<f32>(final_col2));
    let normal = select(segment.rmf_start_normal, normalize(final_chain_matrix * segment.rmf_start_normal), lane_id > 0u);    
    let binormal = normalize(cross(tangent, normal));
    let final_normal = cross(binormal, tangent);

    // In den globalen Task-Payload sichern
    taskPayload.frames[lane_id] = EvaluatedFrame(position, tangent, final_normal, binormal);
    if (lane_id == 0u) {
        taskPayload.segment_id = segment_idx;
        // Schicke exakt 31 Mesh-Shader-Workgroups auf die Reise
        return vec3<u32>(31u, 1u, 1u);
    }
    return vec3<u32>(0u, 0u, 0u);
}

// ==========================================
// 2. MESH SHADER (STUFE 2)
// ==========================================
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @interpolate(flat) @location(2) segment_id: u32,
}

struct PrimitiveOutput {
    @builtin(triangle_indices) indices: vec3<u32>,
}

struct MeshOutput {
    @builtin(vertices) vertices: array<VertexOutput, 32>,
    @builtin(primitives) primitives: array<PrimitiveOutput, 32>,
    @builtin(vertex_count) vertex_count: u32,
    @builtin(primitive_count) primitive_count: u32,
}

var<workgroup> mesh_output: MeshOutput;

@mesh(mesh_output)
@payload(taskPayload)
@workgroup_size(32, 1, 1)
fn ms_main(
    @builtin(local_invocation_id) thread_id: vec3<u32>,
    @builtin(workgroup_id) wg_id: vec3<u32>
) {
    let sub_slice_idx = wg_id.x; // Wert von 0 bis 30
    let lane_id = thread_id.x;
    let radial_segments = config.radial_segments;

    let total_vertices = radial_segments * 2u;
    let total_triangles = radial_segments * 2u;

    if (lane_id == 0u) {
        mesh_output.vertex_count = total_vertices;
        mesh_output.primitive_count = total_triangles;
    }

    // Vertices emittieren
    if (lane_id < total_vertices) {
        let ring = lane_id / radial_segments; // 0 (Startring) oder 1 (Endring)
        let rad_idx = lane_id % radial_segments;

        // Direktzugriff auf die vorberechneten Kurven-Frames aus dem Task-Payload
        let frame = taskPayload.frames[sub_slice_idx + ring];
        
        let angle = (f32(rad_idx) / f32(radial_segments)) * 2.0 * PI;
        let local_normal = (frame.normal * cos(angle)) + (frame.binormal * sin(angle));
        let world_pos = frame.position + (local_normal * config.tube_radius);

        var out: VertexOutput;
        out.position = config.view_projection * vec4<f32>(world_pos, 1.0);
        out.normal = normalize(local_normal);
        out.uv = vec2<f32>(f32(rad_idx) / f32(radial_segments), f32(sub_slice_idx + ring) * 0.1);
        out.segment_id = taskPayload.segment_id;

        mesh_output.vertices[lane_id] = out;
    }

    // Topologie-Indizes emittieren
    if (lane_id < radial_segments) {
        let i0 = lane_id;
        let i1 = (lane_id + 1u) % radial_segments;
        let i2 = i0 + radial_segments;
        let i3 = i1 + radial_segments;

        let tri_idx = lane_id * 2u;

        mesh_output.primitives[tri_idx].indices = vec3<u32>(i0, i1, i2);
        mesh_output.primitives[tri_idx + 1u].indices = vec3<u32>(i2, i1, i3);
    }
}

// ==========================================
// 3. FRAGMENT SHADER
// ==========================================
@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let diff = max(dot(vertex.normal, light_dir), 0.0);
    
    let x = (vertex.segment_id * 1664525u + 1013904223u) & 0xFFFFFFu;
    let base_color = vec3<f32>(
        f32((x >> 16u) & 0xFFu) / 255.0, 
        f32((x >> 8u) & 0xFFu) / 255.0, 
        f32(x & 0xFFu) / 255.0
    ); 
    
    return vec4<f32>(base_color * (diff + 0.15), 1.0);
}
