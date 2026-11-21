@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> VertexOutput {
    // The finest grid there is, six vertices per cell. Cells the patch
    // does not need end up with all of their corners in one place.
    let cells = 1u << args.max_level;
    let cell = vertex / 6u;
    var corners = array<vec2<u32>, 6>(
        vec2<u32>(0u, 0u), vec2<u32>(0u, 1u), vec2<u32>(1u, 0u),
        vec2<u32>(0u, 1u), vec2<u32>(1u, 1u), vec2<u32>(1u, 0u),
    );
    let index = vec2<u32>(cell / cells, cell % cells) + corners[vertex % 6u];

    if (!piece_present(instance)) {
        return VertexOutput(vec4<f32>(0.0), vec3<f32>(0.0), 0.0, 0u);
    }
    let piece = element(instance);
    let lod = lod_of(piece);
    let uv = lod_uv(lod, vec2<u32>(args.max_level), index);
    let point = surface_point(piece, uv);
    // Without mesh shaders there are no meshlets: a patch is drawn on its
    // own, and is what is coloured.
    return VertexOutput(
        args.view_projection * vec4<f32>(point.position, 1.0),
        point.normal,
        lod_detail(lod),
        instance,
    );
}
