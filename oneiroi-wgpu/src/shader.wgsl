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

// Bindegruppe für die Kurvendaten (Read-Only)
@group(0) @binding(0) var<storage, read> segments: array<CubicNurbsSegmentCache>;
//@group(0) @binding(1) var<uniform> num_segments: u32;

// Bindegruppe für Ein- und Ausgabedaten der GPU-Abfrage
@group(1) @binding(0) var<storage, read> input_t: array<f32>;

@group(1) @binding(1) var<storage, read_write> output_points: array<vec3<f32>>;

// Hilfsfunktion zur Segment-Suche mittels Binärsuche (Äquivalent zu partition_point)
fn find_segment_idx(t: f32) -> u32 {
    var num_segments = arrayLength(&segments);
    if (num_segments == 0u) { return 0u; }
    if (t <= segments[0u].t_start) { return 0u; }
    if (t >= segments[num_segments - 1u].t_end) { return num_segments - 1u; }

    var low: u32 = 0u;
    var high: u32 = num_segments;

    while (low < high) {
        let mid = low + (high - low) / 2u;
        if (t >= segments[mid].t_start) {
            low = mid + 1u;
        } else {
            high = mid;
        }
    }
    return low - 1u;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= arrayLength(&input_t)) { return; }

    let t = input_t[idx];
    let seg_idx = find_segment_idx(t);
    let segment = segments[seg_idx];

    let u = (t - segment.t_start) / (segment.t_end - segment.t_start);
    
    // Explicitly mirror the Horner scheme accumulation chain from Rust
    let p_hom = fma(
        fma(
            fma(segment.coeff_col3, vec4<f32>(u), segment.coeff_col2), 
            vec4<f32>(u), 
            segment.coeff_col1
        ), 
        vec4<f32>(u), 
        segment.coeff_col0
    );

    // Dehomogenization
    output_points[idx] = p_hom.xyz / p_hom.w;
    }
