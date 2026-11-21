enable wgpu_binding_array;

struct Page {
    words: array<u32>,
}

@group(0) @binding(0) var<storage, read_write> pages: binding_array<Page>;
@group(1) @binding(0) var<storage, read> calls: array<u32>;

@compute @workgroup_size(64)
fn main(
    @builtin(global_invocation_id) id: vec3<u32>,
    @builtin(workgroup_id) group: vec3<u32>,
) {
    let call = group.y * 5u;
    if (id.x >= calls[call + 2u]) {
        return;
    }
    let from_page = calls[call];
    let to_page = calls[call + 3u];
    pages[to_page].words[calls[call + 4u] + id.x] = pages[from_page].words[calls[call + 1u] + id.x];
}
