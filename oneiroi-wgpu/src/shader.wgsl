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

// 5-Point Gauss–Legendre constants
const GAUSS_NODES = array<f32, 5>(0.0, -0.5384693, 0.5384693, -0.90617985, 0.90617985);
const GAUSS_WEIGHTS = array<f32, 5>(0.5688889, 0.47862867, 0.47862867, 0.23692689, 0.23692689);

fn get_double_reflection_matrix(pos_a: vec3<f32>, pos_b: vec3<f32>, tang_a: vec3<f32>, tang_b: vec3<f32>) -> mat3x3<f32> {
    let v1 = pos_b - pos_a;
    let v1_len_sq = dot(v1, v1);
    if (v1_len_sq < 1e-6) { 
        return mat3x3<f32>(vec3<f32>(1.0,0.0,0.0), vec3<f32>(0.0,1.0,0.0), vec3<f32>(0.0,0.0,1.0)); 
    }
    
    let t_l = tang_a - (2.0 / v1_len_sq) * dot(v1, tang_a) * v1;
    let v2 = tang_b - t_l;
    let v2_len_sq = dot(v2, v2);
    if (v2_len_sq < 1e-6) { 
        return mat3x3<f32>(vec3<f32>(1.0,0.0,0.0), vec3<f32>(0.0,1.0,0.0), vec3<f32>(0.0,0.0,1.0)); 
    }

    let r1 = vec3<f32>(1.0, 0.0, 0.0) - (2.0 / v1_len_sq) * v1.x * v1;
    let r2 = vec3<f32>(0.0, 1.0, 0.0) - (2.0 / v1_len_sq) * v1.y * v1;
    let r3 = vec3<f32>(0.0, 0.0, 1.0) - (2.0 / v1_len_sq) * v1.z * v1;
    let M1 = mat3x3<f32>(r1, r2, r3);

    let u1 = vec3<f32>(1.0, 0.0, 0.0) - (2.0 / v2_len_sq) * v2.x * v2;
    let u2 = vec3<f32>(0.0, 1.0, 0.0) - (2.0 / v2_len_sq) * v2.y * v2;
    let u3 = vec3<f32>(0.0, 0.0, 1.0) - (2.0 / v2_len_sq) * v2.z * v2;
    let M2 = mat3x3<f32>(u1, u2, u3);

    return M2 * M1;
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
    
    // 1. Gleichmäßige Unterteilung des Segments in 32 diskrete Punkte (u von 0.0 bis 1.0)
    let u = f32(lane_id) / 31.0;
    let u_splat = vec4<f32>(u);
    
    // Analytische Positionsauswertung im Monom-Raum (O(3) Horner)
    let p_hom = fma(segment.coeffs[3], u_splat, segment.coeffs[2]);
    let p_hom2 = fma(p_hom, u_splat, segment.coeffs[1]);
    let position_hom = fma(p_hom2, u_splat, segment.coeffs[0]);
    let w = position_hom.w;
    let position = position_hom.xyz / w;
    
    // Analytische Ableitungs- und Tangentenauswertung
    let d3 = segment.coeffs[3] * 3.0;
    let d2 = segment.coeffs[2] * 2.0;
    let dp_du = fma(d3, u_splat, d2);
    let derivative_hom = fma(dp_du, u_splat, segment.coeffs[1]);
    
    let da = derivative_hom.xyz / dt;
    let dw = derivative_hom.w / dt;
    let velocity = (da - dw * position) / w;
    let tangent = normalize(velocity);

    // 2. Lokale Brücken-Rotationsmatrix zur nächsten Lane ermitteln
    // Jede Lane holt sich die Geometriedaten ihres direkten rechten Nachbars
    let next_pos = subgroupShuffleDown(position, 1u);
    let next_tangent = subgroupShuffleDown(tangent, 1u);
    
    var local_R = mat3x3<f32>(vec3<f32>(1.0,0.0,0.0), vec3<f32>(0.0,1.0,0.0), vec3<f32>(0.0,0.0,1.0));
    if (lane_id < 31u) {
        local_R = get_double_reflection_matrix(position, next_pos, tangent, next_tangent);
    }

    // 3. Bleeding-Edge Parallel Prefix Scan via Warp-Shuffles
    // Akkumuliert alle lokalen Segment-Transformationen in log2(32) = 5 Taktzyklen rein in Registern!
    var cumulative_R = local_R;
    for (var offset = 1u; offset < 32u; offset *= 2u) {
        // Wir extrahieren die 3 Spaltenvektoren der Matrix einzeln
        let col0 = cumulative_R[0];
        let col1 = cumulative_R[1];
        let col2 = cumulative_R[2];

        // Shuffeln der Vektoren (Das ist hardwareseitig voll erlaubt!)
        let spawned_col0 = subgroupShuffleUp(col0, offset);
        let spawned_col1 = subgroupShuffleUp(col1, offset);
        let spawned_col2 = subgroupShuffleUp(col2, offset);

        if (lane_id >= offset) {
            // Matrix auf der Empfängerseite wieder zusammenbauen
            let spawned_R = mat3x3<f32>(spawned_col0, spawned_col1, spawned_col2);
            
            // Matrix-Multiplikation durchführen
            cumulative_R = spawned_R * cumulative_R; 
        }
    }

    // 4. Start-Normale laden
    let start_normal_cpu = segment.rmf_start_normal;

    // 5. Transformation der Startnormale
    var normal = start_normal_cpu;
    if (lane_id > 0u) {
        // Auch hier: Den Matrix-Shuffle des linken Nachbars spaltenweise auflösen
        let final_col0 = subgroupShuffleUp(cumulative_R[0], 1u);
        let final_col1 = subgroupShuffleUp(cumulative_R[1], 1u);
        let final_col2 = subgroupShuffleUp(cumulative_R[2], 1u);
        let final_chain_matrix = mat3x3<f32>(final_col0, final_col1, final_col2);
        
        normal = normalize(final_chain_matrix * start_normal_cpu);
    }


    // 6. Strategie 2 Rekonstruktion: Binormale driftfrei über Kreuzprodukt erzeugen
    let binormal = normalize(cross(tangent, normal));
    // Letzter orthogonaler Feinschliff für die endgültige Normale
    let final_normal = cross(binormal, tangent);

    // 7. Sequentiellen Ausgabe-Index berechnen und in den VRAM streamen
    let global_write_idx = (segment_idx * 32u) + lane_id;
    output_samples[global_write_idx] = EvaluatedFrame(position, tangent, final_normal, binormal);
}
