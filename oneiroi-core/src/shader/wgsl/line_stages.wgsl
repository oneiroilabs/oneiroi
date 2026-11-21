@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> @builtin(position) vec4<f32> {
    if (!piece_present(instance)) {
        return vec4<f32>(0.0);
    }
    // Two vertices per step: where it starts and where it ends.
    let step = vertex / 2u + vertex % 2u;
    let position = line_point(element(instance), f32(step) / f32(args.rings));
    return args.view_projection * vec4<f32>(position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(args.color.rgb, 1.0);
}
