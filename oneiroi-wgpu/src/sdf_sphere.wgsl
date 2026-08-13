struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

struct SdfSphereUniforms{
    inv_view_proj: mat4x4f, 
    color: vec4f,          
    origin: vec3f,         
    radius: f32,           
};
@group(0) @binding(0)
var<uniform> uniforms: SdfSphereUniforms;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1 - i32(in_vertex_index & 1u)) * 2.0 - 1.0;
    let y = f32(1 - i32((in_vertex_index >> 1u) & 1u)) * 2.0 - 1.0;
    
    out.position = vec4f(x, y, 1.0, 1.0);
    out.uv = vec2f(x, y);
    return out;
}

fn sdf_sphere(p: vec3f, c: vec3f, r: f32) -> f32 {
    return length(p - c) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let near_target = uniforms.inv_view_proj * vec4f(in.uv.x, in.uv.y, 1.0, 1.0);
    let world_near  = near_target.xyz / near_target.w;

    // 2. Um Richtungsfehler durch die unendliche Reverse-Z-Fernebene (Z=0.0) zu vermeiden,
    // nutzen wir einen Delta-Schritt in NDC-Tiefe (z.B. Z=0.9), um die Richtung exakt zu bestimmen.
    let dir_target  = uniforms.inv_view_proj * vec4f(in.uv.x, in.uv.y, 0.9, 1.0);
    let world_dir_p = dir_target.xyz / dir_target.w;

    // Strahl-Setup
    let ray_origin = world_near;
    let ray_dir    = normalize(world_dir_p - world_near);

    var depth = 0.0;
    let max_depth = 150.0; // Leicht erhöht für große Entfernungen
    var hit = false;

    // 5. Raymarching-Schleife mit angepasster Trefferschwelle
    for (var i = 0; i < 128; i++) {
        let hit_pos = ray_origin + ray_dir * depth;
        let dist = sdf_sphere(hit_pos, uniforms.origin, uniforms.radius);
        
        // Da wir eine riesige Kugel (Radius 20) rendern, erhöhen wir die Toleranz leicht
        if (dist < 0.005) {
            hit = true;
            break;
        }
        
        depth += dist;
        if (depth >= max_depth) {
            break;
        }
    }

    // Wenn nichts getroffen wurde, Pixel komplett verwerfen (damit die Tube sichtbar bleibt)
    if (!hit) {
      discard;
    }

    return uniforms.color;
}