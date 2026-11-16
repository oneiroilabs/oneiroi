struct Uniforms {
    view_projection: mat4x4<f32>,
    // Points from the surface towards the light.
    light: vec4<f32>,
    color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
}

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) normal: vec3<f32>) -> VertexOutput {
    return VertexOutput(uniforms.view_projection * vec4<f32>(position, 1.0), normal);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Both sides are lit the same, a preview should never hide a surface
    // just because it faces the wrong way.
    let diffuse = abs(dot(normalize(input.normal), normalize(uniforms.light.xyz)));
    return vec4<f32>(uniforms.color.rgb * (0.2 + 0.8 * diffuse), 1.0);
}
