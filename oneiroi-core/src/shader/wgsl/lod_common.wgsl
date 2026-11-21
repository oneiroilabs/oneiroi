// Workgroups which go beyond what one dimension can count go on in the
// next one: workgroup `x + y * FOLD`.
const FOLD: u32 = 32768u;

struct LodArgs {
    view_projection: mat4x4<f32>,
    light: vec4<f32>,
    color: vec4<f32>,
    viewport: vec2<f32>,
    pixels: f32,
    offset: u32,
    max_level: u32,
    debug: u32,
    patches: u32,
}

@group(0) @binding(0) var<storage, read> page: array<u32>;
@group(0) @binding(1) var<uniform> args: LodArgs;
// Only for `cs_count`: how many triangles all patches get together.
@group(0) @binding(2) var<storage, read_write> counter: atomic<u32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    // How fine the patch is drawn, from 0 to 1.
    @location(1) detail: f32,
    // What it was drawn as a part of: the meshlet of a mesh shader, or
    // the patch when vertices are pulled. Only for the colours of
    // `LOD_MESHLETS`.
    @location(2) @interpolate(flat) group: u32,
}

// How many steps each edge of a patch gets. They are not whole numbers:
// what is behind the point is how far the next finer steps have slid in.
struct Lod {
    // The edges which run along the patch, at v = 0 and at v = 1.
    along: vec2<f32>,
    // The edges which run across it, at u = 0 and at u = 1.
    across: vec2<f32>,
}

fn lod_pixels(position: vec3<f32>) -> vec2<f32> {
    let clip = args.view_projection * vec4<f32>(position, 1.0);
    return (clip.xy / max(clip.w, 1e-4) * 0.5 + 0.5) * args.viewport;
}

fn lod_level(factor: f32) -> u32 {
    return u32(ceil(log2(max(factor, 1.0))));
}

// Where vertex `index` of a grid with 2^grid_level steps lies on an edge
// which gets `factor` steps. The edge only has the vertices of the level
// above `factor`, so the others of the grid fall onto those. Every second
// one of them is on its way from its neighbour to its own place, which it
// reaches when `factor` reaches the level.
fn lod_morph(index: u32, grid_level: u32, factor: f32) -> f32 {
    let level = min(lod_level(factor), grid_level);
    let shift = grid_level - level;
    let k = (index + ((1u << shift) >> 1u)) >> shift;
    let steps = f32(1u << level);
    var place = f32(k) / steps;
    if (level > 0u && (k & 1u) == 1u) {
        let coarser = f32(1u << (level - 1u));
        let slid = clamp((factor - coarser) / coarser, 0.0, 1.0);
        place = mix(f32(k - 1u) / steps, place, slid);
    }
    return place;
}

// The inside of a patch is as fine as its finest edge in each direction.
fn lod_inside(lod: Lod) -> vec2<f32> {
    return vec2<f32>(max(lod.along.x, lod.along.y), max(lod.across.x, lod.across.y));
}

// The uv of vertex `index` of a grid with 2^levels steps. The patch may
// need fewer steps than the grid has, then several vertices of the grid
// share a place. The vertices which end up on the rim of the patch follow
// the steps of the edge they are on, which is what the patch next to it
// does as well.
fn lod_uv(lod: Lod, levels: vec2<u32>, index: vec2<u32>) -> vec2<f32> {
    let inside = lod_inside(lod);
    let needed = min(vec2<u32>(lod_level(inside.x), lod_level(inside.y)), levels);
    let shift = levels - needed;
    let k = (index + ((vec2<u32>(1u) << shift) >> vec2<u32>(1u))) >> shift;
    let last = vec2<u32>(1u) << needed;

    var factor = inside;
    if (k.y == 0u) {
        factor.x = lod.along.x;
    } else if (k.y == last.y) {
        factor.x = lod.along.y;
    }
    if (k.x == 0u) {
        factor.y = lod.across.x;
    } else if (k.x == last.x) {
        factor.y = lod.across.y;
    }
    return vec2<f32>(
        lod_morph(k.x, needed.x, factor.x),
        lod_morph(k.y, needed.y, factor.y),
    );
}

// A hash which scatters neighbouring numbers, so neighbouring meshlets get
// hues far apart (PCG).
fn lod_hash(value: u32) -> u32 {
    let state = value * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

// A bright colour of a hue picked by `hash`.
fn lod_hue(hash: u32) -> vec3<f32> {
    let hue = f32(hash & 0xffffu) / 65535.0;
    let rgb = clamp(abs(fract(vec3<f32>(hue) + vec3<f32>(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - 3.0) - 1.0, vec3<f32>(0.0), vec3<f32>(1.0));
    return mix(rgb, vec3<f32>(1.0), 0.2);
}

fn lod_detail(lod: Lod) -> f32 {
    let inside = lod_inside(lod);
    return log2(max(inside.x, inside.y)) / f32(max(args.max_level, 1u));
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Both sides are lit the same, a preview should never hide a surface
    // just because it faces the wrong way.
    let diffuse = abs(dot(normalize(input.normal), normalize(args.light.xyz)));
    var color = args.color.rgb;
    if (args.debug == 1u) {
        color = mix(vec3<f32>(0.0, 0.2, 1.0), vec3<f32>(1.0, 0.2, 0.0), input.detail);
    } else if (args.debug == 2u) {
        color = lod_hue(lod_hash(input.group));
    }
    return vec4<f32>(color * (0.2 + 0.8 * diffuse), 1.0);
}
