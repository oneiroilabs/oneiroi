struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

struct SdfUniforms {
    color: vec4f,       
    origin: vec2f,      
    radius: f32,        
    aspect_ratio: f32, 
};

@group(0) @binding(0)
var<uniform> uniforms: SdfUniforms;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1 - i32(in_vertex_index & 1u)) * 2.0 - 1.0;
    let y = f32(1 - i32((in_vertex_index >> 1u) & 1u)) * 2.0 - 1.0;
    
    out.position = vec4f(x, y, 0.0, 1.0);
    out.uv = vec2f(x, y);
    return out;
}

fn sdf_circle(p: vec2f, r: f32) -> f32 {
    return length(p) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    var corrected_uv = in.uv;
    corrected_uv.x = corrected_uv.x * uniforms.aspect_ratio;
    
    let corrected_origin = vec2f(uniforms.origin.x * uniforms.aspect_ratio, uniforms.origin.y);
    let sample_pos = corrected_uv - corrected_origin;
    
    let distance = sdf_circle(sample_pos, uniforms.radius);
    
    let edge_softness = fwidth(distance);
    let alpha = 1.0 - smoothstep(-edge_softness, edge_softness, distance);
    
    return vec4f(uniforms.color.rgb, uniforms.color.a * alpha);
}