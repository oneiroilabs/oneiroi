struct CubicNurbsSegmentCache {
    coefficients: mat4x4<f32>,
    t_start: f32,
    t_end: f32,
    length: f32,
    cumulative_length: f32,
}

// Bindegruppe für die Kurvendaten (Read-Only)
@group(0) @binding(0) var<storage, read> segments: array<CubicNurbsSegmentCache>;
@group(0) @binding(1) var<uniform> num_segments: u32;

// Bindegruppe für Ein- und Ausgabedaten der GPU-Abfrage
@group(1) @binding(0) var<storage, read> input_t: array<f32>;
@group(1) @binding(1) var<storage, read_write> output_points: array<vec3<f32>>;

// Hilfsfunktion zur Segment-Suche mittels Binärsuche (Äquivalent zu partition_point)
fn find_segment_idx(t: f32) -> u32 {
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
    
    // Array-Grenzen absichern gegen ungerade Workgroup-Größen
    if (idx >= arrayLength(&input_t)) { return; }

    let t = input_t[idx];
    let seg_idx = find_segment_idx(t);
    let segment = segments[seg_idx];

    // Normalisierung des t-Parameters auf das lokale Segment [0.0, 1.0]
    let u = (t - segment.t_start) / (segment.t_end - segment.t_start);

    let mat = segment.coefficients;
    
    // Horner-Schema Auswertung (Entspricht exakt der mul_add Kette im Rust Code)
    // WGSL mat4x4 Spalten-Indizierung erfolgt über mat[col_idx]
    let horner_eval = mat[3u] * u + mat[2u];
    let horner_eval2 = horner_eval * u + mat[1u];
    let p_hom = horner_eval2 * u + mat[0u];

    // Dehomogenisierung (Perspektivische Division der W-Komponente)
    output_points[idx] = vec3<f32>(p_hom.x / p_hom.w, p_hom.y / p_hom.w, p_hom.z / p_hom.w);
}
