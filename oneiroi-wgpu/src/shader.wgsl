struct CubicNurbsSegmentCache {
    coeffs: mat4x4<f32>,
    
    t_start: f32,
    t_end: f32,
    length: f32,
    cumulative_length: f32,

    rmf_start_normal: vec3<f32>,
    _pad0: u32,
}

@group(0) @binding(0)
var<storage, read> segments: array<CubicNurbsSegmentCache>;

struct EvaluatedFrame {
    position: vec3<f32>,
    tangent: vec3<f32>,
    normal: vec3<f32>,
    binormal: vec3<f32>,
}

@group(1) @binding(0)
var<storage, read_write> output_samples: array<EvaluatedFrame>;

fn get_double_reflection_matrix(pos_a: vec3<f32>, pos_b: vec3<f32>, tang_a: vec3<f32>, tang_b: vec3<f32>) -> mat3x3<f32> {
    let v1 = pos_b - pos_a;
    let v1_len_sq = dot(v1, v1);
    if (v1_len_sq < 1e-6) { 
        return mat3x3<f32>(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0)); 
    }
    
    let c1 = 2.0 / v1_len_sq;
    let t_l = tang_a - (c1 * dot(v1, tang_a)) * v1;
    
    let v2 = tang_b - t_l;
    let v2_len_sq = dot(v2, v2);
    if (v2_len_sq < 1e-6) { 
        let r1 = vec3<f32>(1.0, 0.0, 0.0) - (c1 * v1.x) * v1;
        let r2 = vec3<f32>(0.0, 1.0, 0.0) - (c1 * v1.y) * v1;
        let r3 = vec3<f32>(0.0, 0.0, 1.0) - (c1 * v1.z) * v1;
        return mat3x3<f32>(r1, r2, r3); 
    }

    let c2 = 2.0 / v2_len_sq;

    let r1 = vec3<f32>(1.0, 0.0, 0.0) - (c1 * v1.x) * v1;
    let r2 = vec3<f32>(0.0, 1.0, 0.0) - (c1 * v1.y) * v1;
    let r3 = vec3<f32>(0.0, 0.0, 1.0) - (c1 * v1.z) * v1;

    let col0 = r1 - (c2 * dot(v2, r1)) * v2;
    let col1 = r2 - (c2 * dot(v2, r2)) * v2;
    let col2 = r3 - (c2 * dot(v2, r3)) * v2;

    return mat3x3<f32>(col0, col1, col2);
}




@compute @workgroup_size(32,1,1)
fn main(
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) wg_id: vec3<u32>
    ) {
    let segment_idx = wg_id.x;
    let lane_id = local_id.x;
    
    let segment = segments[segment_idx];
    let dt = segment.t_end - segment.t_start;
    
    // The Lanes Are going from 0.0 to 1.0
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

    //Grab neighbouring position
    var local_R = mat3x3<f32>(vec3<f32>(1.0,0.0,0.0), vec3<f32>(0.0,1.0,0.0), vec3<f32>(0.0,0.0,1.0));
    let next_pos = subgroupShuffleDown(position, 1u);
    let next_tangent = subgroupShuffleDown(tangent, 1u);
    if (lane_id < 31u) {
        local_R = get_double_reflection_matrix(position, next_pos, tangent, next_tangent);
    }

    // 5 Iterations in the 32 Worgroup (1,2,4,8,16)
    for (var offset = 1u; offset < 32u; offset *= 2u) {
        // WGSL does not allow mat3x3 subgroup ops so we sadly need to split.
        let spawned_col0 = subgroupShuffleUp(local_R[0], offset);
        let spawned_col1 = subgroupShuffleUp(local_R[1], offset);
        let spawned_col2 = subgroupShuffleUp(local_R[2], offset);

        let cond= (lane_id >= offset);
        local_R[0]=select(local_R[0], local_R[0] * spawned_col0, cond);
        local_R[1]=select(local_R[1], local_R[1] * spawned_col1, cond);
        local_R[2]=select(local_R[2], local_R[2] * spawned_col2, cond);
    }

    // WGSL does not allow mat3x3 subgroup ops so we sadly need to split.
    let final_col0 = subgroupShuffleUp(local_R[0], 1u);
    let final_col1 = subgroupShuffleUp(local_R[1], 1u);
    let final_col2 = subgroupShuffleUp(local_R[2], 1u);
    let final_chain_matrix = mat3x3<f32>(final_col0, final_col1, final_col2);
    let normal=select(segment.rmf_start_normal,normalize(final_chain_matrix * segment.rmf_start_normal),lane_id > 0u);    

    let binormal = normalize(cross(tangent, normal));
    let final_normal = cross(binormal, tangent);

    let global_write_idx = (segment_idx * 32u) + lane_id;
    output_samples[global_write_idx] = EvaluatedFrame(position, tangent, final_normal, binormal);
}
