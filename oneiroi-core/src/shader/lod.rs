//! The shaders which draw a surface as fine as it appears on screen.

use std::fmt::Write;

use crate::{node::{Drawable, Topology}, shader::{ShaderError, draw::draw_pieces, validate_with}};

/// How a [`LodShader`] turns patches into triangles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LodMethod {
    /// A task shader decides per patch how fine it has to be and whether
    /// it is in view at all, and mesh shaders make exactly that many
    /// triangles. Needs a device with mesh shaders.
    MeshShader,
    /// Every patch is drawn as the finest grid there is, and the vertex
    /// shader lets the cells which are not needed collapse to nothing.
    /// Works everywhere and looks the same, but shades more vertices.
    VertexPulling,
}

/// A shader which draws the patches of a surface as fine as they appear
/// on screen: every edge of a patch gets as many steps as it is long in
/// pixels, and the steps slide into place when that number changes
/// instead of popping. Two patches which share an edge agree on its
/// steps, so there is no gap between them.
///
/// Everything is bound in group 0: binding 0 is the page the patches live
/// in (read-only `array<u32>`), binding 1 a uniform [`LodArgs`].
///
/// - [`LodMethod::MeshShader`]: entry points `ts_main`, `ms_main` and
///   `fs_main`. Draw one task workgroup per patch.
/// - [`LodMethod::VertexPulling`]: entry points `vs_main` and `fs_main`.
///   Draw every patch as one instance with [`LodShader::vertices`].
///
/// Both have a compute entry point `cs_count` as well, which adds up how
/// many triangles the patches in view get, into an `atomic<u32>` at
/// binding 2. Dispatch `ceil(patches / 64)` workgroups. It is how a budget
/// of triangles can be held, see [`LodShader::COUNT_GROUP`].
#[derive(Debug, Clone)]
pub struct LodShader {
    pub wgsl: String,
}

impl LodShader {
    /// How many patches one workgroup of `cs_count` looks at.
    pub const COUNT_GROUP: u32 = 64;

    /// How many patches one task workgroup of [`LodMethod::MeshShader`]
    /// takes care of. Dispatch `ceil(patches / TASK_GROUP)` of them.
    pub const TASK_GROUP: u32 = 16;

    /// The finest [`LodMethod::MeshShader`] draws: what one task workgroup
    /// asks of mesh shaders has to fit into its payload.
    pub const MESH_MAX_LEVEL: u32 = 6;

    /// How many vertices [`LodMethod::VertexPulling`] draws per patch.
    pub const fn vertices(max_level: u32) -> u32 {
        6 << (2 * max_level)
    }
}

/// The finest a patch can get: `2^MAX_LOD_LEVEL` steps along each side.
pub const MAX_LOD_LEVEL: u32 = 7;

/// How a surface drawn as fine as it appears is coloured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LodColors {
    /// In its own colour, lit.
    #[default]
    Shaded,
    /// By how fine each patch is drawn, from blue for coarse to red for fine.
    Detail,
    /// A colour for every meshlet: what one mesh shader makes, a block of
    /// a large patch or several small patches. Drawn without mesh shaders,
    /// there are no meshlets, and every patch gets a colour instead.
    Meshlets,
}

impl LodColors {
    /// What the shader is told in `debug`.
    pub fn word(self) -> u32 {
        match self {
            Self::Shaded => 0,
            Self::Detail => 1,
            Self::Meshlets => 2,
        }
    }
}

const LOD_COMMON: &str = include_str!("wgsl/lod_common.wgsl");

/// The functions which need to know the type of a patch.
fn lod_typed(name: &str) -> String {
    format!(
        "// How many steps the edge from `a` to `b` gets: its length on screen in
// steps of the size which was asked for.
fn lod_edge(piece: {name}, a: vec2<f32>, b: vec2<f32>) -> f32 {{
    let first = surface_point(piece, a).position;
    var last = first;
    var length = 0.0;
    var on_screen = 0.0;
    for (var step = 1u; step <= 4u; step++) {{
        let next = surface_point(piece, mix(a, b, f32(step) / 4.0)).position;
        length += distance(last, next);
        on_screen += distance(lod_pixels(last), lod_pixels(next));
        last = next;
    }}

    // An edge which ends where it starts goes around something. With
    // fewer than four steps there would be nothing left of that.
    var least = 1.0;
    if (distance(first, last) < 1e-3 * length) {{
        least = 4.0;
    }}
    let most = f32(1u << args.max_level);
    return clamp(on_screen / max(args.pixels, 1.0), min(least, most), most);
}}

fn lod_of(piece: {name}) -> Lod {{
    return Lod(
        vec2<f32>(
            lod_edge(piece, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0)),
            lod_edge(piece, vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 1.0)),
        ),
        vec2<f32>(
            lod_edge(piece, vec2<f32>(0.0, 0.0), vec2<f32>(0.0, 1.0)),
            lod_edge(piece, vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0)),
        ),
    );
}}

@compute @workgroup_size(64)
fn cs_count(@builtin(workgroup_id) group: vec3<u32>, @builtin(local_invocation_id) local: vec3<u32>) {{
    let index = (group.x + group.y * FOLD) * 64u + local.x;
    if (index >= piece_count() || !piece_present(index)) {{
        return;
    }}
    let piece = element(index);
    if (lod_hidden(piece)) {{
        return;
    }}
    let inside = lod_inside(lod_of(piece));
    let cells = (1u << lod_level(inside.x)) * (1u << lod_level(inside.y));
    atomicAdd(&counter, cells * 2u);
}}

// Whether nothing of the patch can be on screen: a few points of it are
// all beyond the same side of what the camera sees, with room to spare
// for what bulges out between them.
fn lod_hidden(piece: {name}) -> bool {{
    var beyond = 63u;
    for (var i = 0u; i < 3u; i++) {{
        for (var j = 0u; j < 3u; j++) {{
            let uv = vec2<f32>(f32(i), f32(j)) * 0.5;
            let clip = args.view_projection * vec4<f32>(surface_point(piece, uv).position, 1.0);
            let reach = clip.w * 1.5;
            var sides = 0u;
            sides |= u32(clip.x < -reach);
            sides |= u32(clip.x > reach) << 1u;
            sides |= u32(clip.y < -reach) << 2u;
            sides |= u32(clip.y > reach) << 3u;
            sides |= u32(clip.w < 0.0) << 4u;
            beyond &= sides;
        }}
    }}
    return beyond != 0u;
}}
",
    )
}

const LOD_VERTEX: &str = include_str!("wgsl/lod_vertex.wgsl");

const LOD_MESH: &str = include_str!("wgsl/lod_mesh.wgsl");

/// Writes the shader which draws a surface as fine as it appears, and
/// checks that it compiles. Only surfaces can be drawn like that.
pub fn draw_lod(drawable: &Drawable, method: LodMethod) -> Result<LodShader, ShaderError> {
    let ty = &drawable.ty;
    if drawable.topology != Topology::Surface {
        return Err(ShaderError::Invalid {
            node: ty.name,
            message: "only a surface has detail to choose".into(),
        });
    }

    let mut wgsl = String::new();
    let out = &mut wgsl;
    if method == LodMethod::MeshShader {
        let _ = writeln!(out, "enable wgpu_mesh_shader;\n");
    }
    let _ = writeln!(out, "// Generated to draw `{}` as fine as it appears.\n", ty.name);
    let _ = writeln!(
        out,
        "struct SurfacePoint {{\n    position: vec3<f32>,\n    normal: vec3<f32>,\n}}\n"
    );
    let _ = writeln!(out, "{LOD_COMMON}");
    let piece = draw_pieces(out, drawable, true);
    let _ = writeln!(out, "{}", lod_typed(piece));
    let _ = write!(
        out,
        "{}",
        match method {
            LodMethod::MeshShader => LOD_MESH,
            LodMethod::VertexPulling => LOD_VERTEX,
        }
    );

    let capabilities = match method {
        LodMethod::MeshShader => naga::valid::Capabilities::MESH_SHADER,
        LodMethod::VertexPulling => naga::valid::Capabilities::empty(),
    };
    validate_with(ty.name, &wgsl, capabilities)?;
    Ok(LodShader { wgsl })
}
