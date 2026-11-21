struct DrawArgs {
    view_projection: mat4x4<f32>,
    light: vec4<f32>,
    color: vec4<f32>,
    offset: u32,
    rings: u32,
    sides: u32,
}

@group(0) @binding(0) var<storage, read> page: array<u32>;
@group(0) @binding(1) var<uniform> args: DrawArgs;
