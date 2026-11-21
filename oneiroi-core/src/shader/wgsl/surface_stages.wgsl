struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> VertexOutput {
    if (!piece_present(instance)) {
        return VertexOutput(vec4<f32>(0.0), vec3<f32>(0.0));
    }
    // Six vertices per cell of the grid: two triangles which wind the
    // way the mesh of the same surface does.
    let cell = vertex / 6u;
    var corners = array<vec2<u32>, 6>(
        vec2<u32>(0u, 0u), vec2<u32>(0u, 1u), vec2<u32>(1u, 0u),
        vec2<u32>(0u, 1u), vec2<u32>(1u, 1u), vec2<u32>(1u, 0u),
    );
    let corner = corners[vertex % 6u];
    let ring = cell / args.sides + corner.x;
    let side = cell % args.sides + corner.y;
    let uv = vec2<f32>(f32(ring) / f32(args.rings), f32(side) / f32(args.sides));

    let point = surface_point(element(instance), uv);
    return VertexOutput(args.view_projection * vec4<f32>(point.position, 1.0), point.normal);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Both sides are lit the same, a preview should never hide a surface
    // just because it faces the wrong way.
    let diffuse = abs(dot(normalize(input.normal), normalize(args.light.xyz)));
    return vec4<f32>(args.color.rgb * (0.2 + 0.8 * diffuse), 1.0);
}
