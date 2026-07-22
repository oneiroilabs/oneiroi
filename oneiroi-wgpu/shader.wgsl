// Aktivierung der Mesh-Shader Erweiterung (wgpu-spezifisch)
enable mesh_shading; 

struct RenderArgs {
    view_proj: mat4x4<f32>,
    subdivisions: u32,       // Wie viele Vertex-Schritte pro Segment (z.B. 32)
    ribbon_width: f32,       // Breite des erzeugten Kurven-Bandes
    padding: f32,
};

struct NurbsSegmentCache {
    marsden_identity: mat4x4<f32>,
    t_start: f32,
    t_end: f32,
    length: f32,
    cumulative_length: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> args: RenderArgs;
@group(0) @binding(1) var<storage, read> segments: array<NurbsSegmentCache>;
// Jedes Segment greift auf seine 4 Kontrollpunkte zu (segment_idx .. segment_idx + 4)
@group(0) @binding(2) var<storage, read> control_points: array<vec4<f32>>; 

// Maximale Ausgabegröße definieren (Beispiel für 32 Unterteilungen)
// 32 Quad-Schritte bedeuten 66 Vertices (links/rechts pro Schritt) und 64 Triangles
@mesh
@workgroup_size(64) 
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    output: mesh_output<VertexOutput, vec3<u32>, 66, 64> 
) {
    let segment_idx = workgroup_id.x;
    let num_subdivisions = args.subdivisions;
    
    // 1. Mesh-Ausgabe-Größe dynamisch setzen
    let total_vertices = (num_subdivisions + 1u) * 2u;
    let total_triangles = num_subdivisions * 2u;
    set_mesh_output_counts(total_vertices, total_triangles);

    let segment = segments[segment_idx];
    let thread_id = local_id.x;

    // 2. Vertices generieren (Jeder Thread berechnet einen "Schnitt" der Kurve)
    if (thread_id <= num_subdivisions) {
        let u = f32(thread_id) / f32(num_subdivisions);
        
        // Laden der 4 homogenen Kontrollpunkte für dieses Segment
        let p0 = control_points[segment_idx + 0u];
        let p1 = control_points[segment_idx + 1u];
        let p2 = control_points[segment_idx + 2u];
        let p3 = control_points[segment_idx + 3u];
        
        let p_mat = mat4x4<f32>(p0, p1, p2, p3);
        let monom = p_mat * segment.marsden_identity;
        
        // Horner-Auswertung (Homogene Koordinaten)
        let p_hom = monom[0] + u * (monom[1] + u * (monom[2] + u * monom[3]));
        let pos = p_hom.xyz / p_hom.w;
        
        // Ableitung nach u für Tangentenberechnung (für das Band-Extrusion)
        let dp_du = monom[1] + u * (2.0 * monom[2] + 3.0 * u * monom[3]);
        let tangent = normalize(dp_du.xyz);
        
        // Einfacher Up-Vektor zur Generierung einer Senkrechten (Ribbon-Effekt)
        let up = vec3<f32>(0.0, 1.0, 0.0);
        let binormal = normalize(cross(tangent, up));
        
        // Links- und Rechts-Erweiterung des Bandes
        let offset = binormal * (args.ribbon_width * 0.5);
        let pos_left = pos - offset;
        let pos_right = pos + offset;
        
        // Indizes im Ausgabearray berechnen
        let v_left_idx = thread_id * 2u;
        let v_right_idx = thread_id * 2u + 1u;
        
        // Vertex-Ausgabe schreiben
        output.vertices[v_left_idx].position = args.view_proj * vec4<f32>(pos_left, 1.0);
        output.vertices[v_left_idx].color = vec4<f32>(u, 0.5, 1.0 - u, 1.0);
        
        output.vertices[v_right_idx].position = args.view_proj * vec4<f32>(pos_right, 1.0);
        output.vertices[v_right_idx].color = vec4<f32>(u, 1.0, 0.5, 1.0);
    }

    // 3. Topologie / Index-Buffer schreiben (Triangle Strip via Triangles simuliert)
    if (thread_id < num_subdivisions) {
        let v_idx = thread_id * 2u;
        let tri_idx = thread_id * 2u;
        
        // Erstes Dreieck des Quads
        output.primitives[tri_idx] = vec3<u32>(v_idx, v_idx + 1u, v_idx + 2u);
        // Zweites Dreieck des Quads
        output.primitives[tri_idx + 1u] = vec3<u32>(v_idx + 1u, v_idx + 3u, v_idx + 2u);
    }
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}