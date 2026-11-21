// A large patch is cut into blocks of this many cells along each side,
// one mesh shader each. Small ones are bundled: several of them share one.
const BLOCK: u32 = 8u;
// What one mesh shader can make at most.
const VERTICES: u32 = 81u;
const TRIANGLES: u32 = 128u;
// How many patches one task shader takes care of.
const PATCHES: u32 = 16u;

struct Payload {
    first: u32,
    // How fine each of the patches is, by its place among the 16.
    // Only arrays of scalars: DXC crashes on a payload with an array of
    // vectors, as naga hands the payload to a function by value. Both
    // levels are in one word, along and then across from bit 16 on.
    levels: array<u32, 16>,
    along_from: array<f32, 16>,
    along_to: array<f32, 16>,
    across_from: array<f32, 16>,
    across_to: array<f32, 16>,
    // What each mesh shader makes. The lowest 4 bits are the place of a
    // patch. Above them is how many patches from there on it makes whole,
    // or 0 if it makes one block of that patch, which is at the bits from
    // 8 on (along) and from 16 on (across).
    groups: array<u32, 1024>,
}

var<task_payload> payload: Payload;

// How many invocations each task and mesh shader has. They share the
// work, as the hardware likes it, instead of one doing all of it.
const LANES: u32 = 32u;

// The bundle being collected while the task shader goes over its patches.
struct Bundle {
    start: u32,
    count: u32,
    vertices: u32,
    triangles: u32,
}

fn cells_of(levels: vec2<u32>) -> vec2<u32> {
    return vec2<u32>(1u << levels.x, 1u << levels.y);
}

// Both levels of a patch out of the word of the payload they are kept in.
fn levels_of(word: u32) -> vec2<u32> {
    return vec2<u32>(word & 0xffffu, word >> 16u);
}

// Which of the 16 patches are on screen, found by one invocation each.
var<workgroup> task_shown: array<u32, 16>;
// How many mesh shaders the task shader starts.
var<workgroup> task_groups: u32;

@task @payload(payload) @workgroup_size(32)
fn ts_main(
    @builtin(workgroup_id) group: vec3<u32>,
    // Not `local_invocation_index`: naga declares that one for DX12 itself,
    // and then twice when the shader does as well.
    @builtin(local_invocation_id) local: vec3<u32>,
) -> @builtin(mesh_task_size) vec3<u32> {
    let lane = local.x;
    let first = (group.x + group.y * FOLD) * PATCHES;
    // Every patch on its own: whether it is there and how fine it gets.
    if (lane < PATCHES) {
        var shown = 0u;
        let index = first + lane;
        if (index < piece_count() && piece_present(index)) {
            let piece = element(index);
            if (!lod_hidden(piece)) {
                let lod = lod_of(piece);
                let inside = lod_inside(lod);
                payload.levels[lane] = lod_level(inside.x) | (lod_level(inside.y) << 16u);
                payload.along_from[lane] = lod.along.x;
                payload.along_to[lane] = lod.along.y;
                payload.across_from[lane] = lod.across.x;
                payload.across_to[lane] = lod.across.y;
                shown = 1u;
            }
        }
        task_shown[lane] = shown;
    }
    workgroupBarrier();

    // Then what the mesh shaders make, in order, by a single invocation.
    if (lane == 0u) {
        payload.first = first;
        var groups = 0u;
        var bundle = Bundle(0u, 0u, 0u, 0u);
        for (var place = 0u; place < PATCHES; place++) {
            if (task_shown[place] == 0u) {
                // A bundle is a run of patches which are all there.
                if (bundle.count > 0u) {
                    payload.groups[groups] = bundle.start | (bundle.count << 4u);
                    groups++;
                    bundle = Bundle(0u, 0u, 0u, 0u);
                }
                continue;
            }

            let cells = cells_of(levels_of(payload.levels[place]));
            let vertices = (cells.x + 1u) * (cells.y + 1u);
            let triangles = cells.x * cells.y * 2u;
            let fits = vertices <= VERTICES && triangles <= TRIANGLES;
            let full = bundle.count == 15u
                || bundle.vertices + vertices > VERTICES
                || bundle.triangles + triangles > TRIANGLES;
            if (bundle.count > 0u && (!fits || full)) {
                payload.groups[groups] = bundle.start | (bundle.count << 4u);
                groups++;
                bundle = Bundle(0u, 0u, 0u, 0u);
            }

            if (fits) {
                if (bundle.count == 0u) {
                    bundle.start = place;
                }
                bundle.count++;
                bundle.vertices += vertices;
                bundle.triangles += triangles;
            } else {
                let blocks = (cells + BLOCK - 1u) / BLOCK;
                for (var x = 0u; x < blocks.x; x++) {
                    for (var y = 0u; y < blocks.y; y++) {
                        payload.groups[groups] = place | (x << 8u) | (y << 16u);
                        groups++;
                    }
                }
            }
        }
        if (bundle.count > 0u) {
            payload.groups[groups] = bundle.start | (bundle.count << 4u);
            groups++;
        }
        task_groups = groups;
    }
    return vec3<u32>(workgroupUniformLoad(&task_groups), 1u, 1u);
}

struct PrimitiveOutput {
    @builtin(triangle_indices) indices: vec3<u32>,
}

struct MeshOutput {
    @builtin(vertices) vertices: array<VertexOutput, 81>,
    @builtin(primitives) primitives: array<PrimitiveOutput, 128>,
    @builtin(vertex_count) vertex_count: u32,
    @builtin(primitive_count) primitive_count: u32,
}

var<workgroup> mesh_output: MeshOutput;

// A mesh shader makes either one block of a large patch, or several small
// patches each whole.
@mesh(mesh_output) @payload(payload) @workgroup_size(32)
fn ms_main(
    @builtin(workgroup_id) group: vec3<u32>,
    // See `ts_main`.
    @builtin(local_invocation_id) local: vec3<u32>,
) {
    let lane = local.x;
    let word = payload.groups[group.x];
    let start = word & 15u;
    let whole = ((word >> 4u) & 15u) != 0u;
    var count = (word >> 4u) & 15u;
    var block = vec2<u32>(0u);
    var block_size = vec2<u32>(0u);
    if (!whole) {
        let cells = cells_of(levels_of(payload.levels[start]));
        block = vec2<u32>((word >> 8u) & 255u, (word >> 16u) & 255u) * BLOCK;
        block_size = min(cells - block, vec2<u32>(BLOCK));
        count = 1u;
    }

    var vertices = 0u;
    var triangles = 0u;
    for (var k = 0u; k < count; k++) {
        let size = select(block_size, cells_of(levels_of(payload.levels[start + k])), whole);
        vertices += (size.x + 1u) * (size.y + 1u);
        triangles += size.x * size.y * 2u;
    }

    // Every invocation makes every 32nd vertex and triangle. Which of the
    // pieces one belongs to is found by going over them.
    for (var vertex = lane; vertex < vertices; vertex += LANES) {
        var k = 0u;
        var base = 0u;
        var size = select(block_size, cells_of(levels_of(payload.levels[start])), whole);
        loop {
            let here = (size.x + 1u) * (size.y + 1u);
            if (k + 1u >= count || vertex < base + here) {
                break;
            }
            base += here;
            k++;
            size = cells_of(levels_of(payload.levels[start + k]));
        }
        let place = start + k;
        let local_vertex = vertex - base;
        let at = vec2<u32>(local_vertex / (size.y + 1u), local_vertex % (size.y + 1u));
        let lod = Lod(
            vec2<f32>(payload.along_from[place], payload.along_to[place]),
            vec2<f32>(payload.across_from[place], payload.across_to[place]),
        );
        let uv = lod_uv(lod, levels_of(payload.levels[place]), block + at);
        let point = surface_point(element(payload.first + place), uv);
        mesh_output.vertices[vertex] = VertexOutput(
            args.view_projection * vec4<f32>(point.position, 1.0),
            point.normal,
            lod_detail(lod),
            // A meshlet: this mesh shader of the task shader which starts at
            // the patch `first`. A task shader starts at most 1024.
            payload.first * 1024u + group.x,
        );
    }
    for (var triangle = lane; triangle < triangles; triangle += LANES) {
        var k = 0u;
        var base = 0u;
        var vertex_base = 0u;
        var size = select(block_size, cells_of(levels_of(payload.levels[start])), whole);
        loop {
            let here = size.x * size.y * 2u;
            if (k + 1u >= count || triangle < base + here) {
                break;
            }
            base += here;
            vertex_base += (size.x + 1u) * (size.y + 1u);
            k++;
            size = cells_of(levels_of(payload.levels[start + k]));
        }
        let cell = (triangle - base) / 2u;
        let a = vertex_base + (cell / size.y) * (size.y + 1u) + cell % size.y;
        let c = a + size.y + 1u;
        var indices = vec3<u32>(a + 1u, c + 1u, c);
        if ((triangle - base) % 2u == 0u) {
            indices = vec3<u32>(a, a + 1u, c);
        }
        mesh_output.primitives[triangle] = PrimitiveOutput(indices);
    }
    if (lane == 0u) {
        mesh_output.vertex_count = vertices;
        mesh_output.primitive_count = triangles;
    }
}
