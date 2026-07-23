// nurbs.wgsl

struct NurbsSegmentCache {
    coefficients: mat4x4<f32>,
    t_start: f32,
    t_end: f32,
    length: f32,
    cumulative_length: f32,
};

// Eingabepuffer: Alle Segmente der Kurve
@group(0) @binding(0) var<storage, read> segments: array<NurbsSegmentCache>;

// Beispiel-Eingabe für Abfrage-Parameter t (z.B. für viele parallele Threads)
@group(0) @binding(1) var<storage, read> query_times: array<f32>;

// Ausgabepuffer für Ergebnisse
struct EvalResult {
    position: vec3<f32>,
    tangent: vec3<f32>,
};
@group(0) @binding(2) var<storage, read_write> output_results: array<EvalResult>;

// Hilfsfunktion zur Segment-Suche (Binäre Suche für GPU-Performance bevorzugt)
fn find_segment_idx(t: f32, num_segments: u32) -> u32 {
    if (t <= segments[0].t_start) { return 0u; }
    if (t >= segments[num_segments - 1u].t_end) { return num_segments - 1u; }

    var low = 0u;
    var high = num_segments - 1u;
    
    while (low <= high) {
        let mid = (low + high) / 2u;
        if (t >= segments[mid].t_start && t < segments[mid].t_end) {
            return mid;
        } else if (t < segments[mid].t_start) {
            high = mid - 1u;
        } else {
            low = mid + 1u;
        }
    }
    return low;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let query_idx = id.x;
    let num_queries = arrayLength(&query_times);
    if (query_idx >= num_queries) { return; }

    let t = query_times[query_idx];
    let num_segments = arrayLength(&segments);
    let seg_idx = find_segment_idx(t, num_segments);
    let seg = segments[seg_idx];

    let dt = seg.t_end - seg.t_start;
    let u = (t - seg.t_start) / dt;

    // Koeffizienten-Spalten extrahieren (identisch zu glam .col(x))
    let a = seg.coefficients[0];
    let b = seg.coefficients[1];
    let c = seg.coefficients[2];
    let d = seg.coefficients[3];

    // 1. Position via Horner-Schema (4D Homogen)
    // Entspricht: u * (u * (u * d + c) + b) + a
    let p_hom = fma(vec4<f32>(u), fma(vec4<f32>(u), fma(vec4<f32>(u), d, c), b), a);

    // 2. Erste Ableitung nach u: u * (3*d * u + 2*c) + b
    let d3 = d * 3.0;
    let d2 = c * 2.0;
    let dp_du = fma(vec4<f32>(u), fma(vec4<f32>(u), d3, d2), b);

    // Transformation in den 3D-Raum (NURBS Projektion)
    let a_xyz = p_hom.xyz;
    let w = p_hom.w;

    let inv_dt = 1.0 / dt;
    let da = dp_du.xyz * inv_dt;
    let dw = dp_du.w * inv_dt;

    let c_pos = a_xyz / w;
    let c_vel = (da - dw * c_pos) / w; // 3D-Tangentenvektor

    // Ergebnis speichern
    output_results[query_idx].position = c_pos;
    output_results[query_idx].tangent = c_vel;
}
