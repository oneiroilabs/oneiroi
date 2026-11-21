//! The shaders which draw values, and cull instances before that.

use std::fmt::Write;

use crate::{
    node::{Drawable, Topology},
    shader::{
        CULL_COUNT, CULL_DRAW, CULL_TASKS, FOLD, LodShader, ShaderError, declare, read_element,
        validate,
    },
    types::{DataType, InstanceRow},
};

/// A shader which draws the elements of a [`Drawable`] type straight out
/// of a page of the store. No vertices are stored anywhere: every one is
/// computed from the index it is drawn with.
///
/// The entry points are `vs_main` and `fs_main` and everything is bound in
/// group 0:
///
/// - binding 0: the page the elements live in, read-only `array<u32>`
/// - binding 1: a uniform [`DrawArgs`]
///
/// Draw every element as one instance. A surface is a triangle list with
/// `rings * sides * 6` vertices, a line a line list with `rings * 2`.
#[derive(Debug, Clone)]
pub struct DrawShader {
    pub wgsl: String,
}

const DRAW_ARGS: &str = include_str!("wgsl/draw_args.wgsl");

const SURFACE_STAGES: &str = include_str!("wgsl/surface_stages.wgsl");

const LINE_STAGES: &str = include_str!("wgsl/line_stages.wgsl");

/// What a draw shader of instances has besides the rest: where the rows
/// and the offsets of the groups of the shape are, and turning.
const INSTANCED: &str = include_str!("wgsl/instanced.wgsl");

/// Writes how a draw shader gets to the pieces it draws: the structs, `fn
/// element(index: u32)` and the functions of the drawable. Returns the
/// type of a piece.
///
/// A piece of instances is a piece of the shape with the transform of its
/// instance: the elements of all instances are counted one after the
/// other, and the row of one is found by where the rows start. Its points
/// are the ones of the shape, turned, scaled and moved. Everything which
/// works on pieces, like choosing their detail, then sees each instance
/// where it is.
pub(super) fn draw_pieces(out: &mut String, drawable: &Drawable, lod: bool) -> &'static str {
    let ty = &drawable.ty;
    let mut declared = Vec::new();
    let Some(shape) = ty.prototype else {
        declare(out, &mut declared, ty);
        let _ = writeln!(
            out,
            "fn element(index: u32) -> {} {{\n    let base = args.offset + index * {}u;\n    return {};\n}}\n",
            ty.wgsl,
            ty.words,
            read_element(ty, "page", "base")
        );
        let _ = writeln!(out, "fn piece_present(index: u32) -> bool {{\n    return true;\n}}\n");
        if lod {
            let _ = writeln!(out, "fn piece_count() -> u32 {{\n    return args.patches;\n}}\n");
        }
        let _ = writeln!(out, "{}\n", drawable.wgsl.trim());
        return ty.wgsl;
    };

    declare(out, &mut declared, shape);
    declare(out, &mut declared, ty);
    let _ = writeln!(out, "{INSTANCED}");
    let _ = writeln!(
        out,
        "struct Placed {{\n    piece: {shape},\n    transform: Similarity,\n}}\n

fn instance_row(index: u32) -> {row} {{
    let base = instance_args.rows_offset + index * {row_words}u;
    return {read_row};
}}

// Every visible instance has `slots` places for pieces, as many as the
// longest group of the shape has. Those beyond its own group are empty.
fn piece_count() -> u32 {{
    return instance_visible[0] * instance_args.slots;
}}

fn instance_of(index: u32) -> {row} {{
    return instance_row(instance_visible[1u + index / instance_args.slots]);
}}

fn piece_present(index: u32) -> bool {{
    let row = instance_of(index);
    let start = instance_group_start(row.group);
    return start + index % instance_args.slots < instance_group_end(row.group);
}}

fn element(index: u32) -> Placed {{
    let row = instance_of(index);
    let base = args.offset + (instance_group_start(row.group) + index % instance_args.slots) * {words}u;
    return Placed({read}, row.transform);
}}
",
        shape = shape.wgsl,
        row = ty.wgsl,
        row_words = ty.words,
        read_row = read_element(ty, "instance_rows", "base"),
        words = shape.words,
        read = read_element(shape, "page", "base"),
    );
    // The functions of the shape under other names, and the ones of a
    // piece around them.
    let wgsl = drawable
        .wgsl
        .replace("fn surface_point(", "fn shape_surface_point(")
        .replace("fn line_point(", "fn shape_line_point(");
    let _ = writeln!(out, "{}\n", wgsl.trim());
    let _ = writeln!(
        out,
        "{}",
        match drawable.topology {
            Topology::Surface => "fn surface_point(placed: Placed, uv: vec2<f32>) -> SurfacePoint {
    let point = shape_surface_point(placed.piece, uv);
    return SurfacePoint(
        instance_place(placed.transform, point.position),
        instance_rotate(placed.transform.rotation, point.normal),
    );
}
",
            Topology::Line => "fn line_point(placed: Placed, u: f32) -> vec3<f32> {
    return instance_place(placed.transform, shape_line_point(placed.piece, u));
}
",
        }
    );
    "Placed"
}

/// The shader which finds the instances which are worth drawing, before
/// they are drawn. Instances are left out which are entirely beside what
/// the camera sees, or smaller on screen than `min_pixels`. The sphere
/// around each group of the shape is known, and turned, scaled and moved
/// with an instance it is around it as well.
///
/// Bound in group 0: binding 0 the page of the rows, 1 a uniform
/// [`CullArgs`], 2 the spheres (`array<vec4<f32>>`, center and radius),
/// 3 where the visible ones go (`array<atomic<u32>>`, how many and then
/// which rows; zero the first word before), 4 the indirect buffer.
///
/// Entry points: `cs_cull`, `ceil(rows / 64)` workgroups, and then
/// `cs_finish`, a single one, which writes the arguments of the draw.
pub fn cull() -> Result<String, ShaderError> {
    let row = &InstanceRow::INFO;
    let mut wgsl = String::new();
    let out = &mut wgsl;
    declare(out, &mut Vec::new(), row);
    let _ = writeln!(
        out,
        "struct CullArgs {{
    view_projection: mat4x4<f32>,
    viewport: vec2<f32>,
    min_pixels: f32,
    rows_offset: u32,
    rows: u32,
    slots: u32,
    vertices: u32,
}}

const FOLD: u32 = {FOLD}u;
const PATCHES: u32 = {patches}u;
const COUNT_GROUP: u32 = {count}u;

@group(0) @binding(0) var<storage, read> page: array<u32>;
@group(0) @binding(1) var<uniform> args: CullArgs;
@group(0) @binding(2) var<storage, read> spheres: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read_write> visible: array<atomic<u32>>;
@group(0) @binding(4) var<storage, read_write> indirect: array<u32>;

fn row_at(index: u32) -> {name} {{
    let base = args.rows_offset + index * {words}u;
    return {read};
}}

fn rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {{
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}}

fn matrix_row(m: mat4x4<f32>, i: u32) -> vec4<f32> {{
    return vec4<f32>(m[0][i], m[1][i], m[2][i], m[3][i]);
}}

// Whether a sphere is entirely beyond a plane of what the camera sees.
fn beyond(plane: vec4<f32>, center: vec3<f32>, radius: f32) -> bool {{
    return dot(plane.xyz, center) + plane.w < -radius * length(plane.xyz);
}}

@compute @workgroup_size(64)
fn cs_cull(@builtin(global_invocation_id) id: vec3<u32>) {{
    if (id.x >= args.rows) {{
        return;
    }}
    let row = row_at(id.x);
    let sphere = spheres[row.group];
    if (sphere.w < 0.0) {{
        // An empty group: there is nothing to draw.
        return;
    }}
    let t = row.transform;
    let center = rotate(t.rotation, sphere.xyz) * t.scale + t.offset;
    // Some room for what bulges out between the points the sphere was
    // found with.
    let radius = sphere.w * abs(t.scale) * 1.1 + 1e-4;

    let m = args.view_projection;
    let x = matrix_row(m, 0u);
    let y = matrix_row(m, 1u);
    let w = matrix_row(m, 3u);
    if (beyond(w + x, center, radius) || beyond(w - x, center, radius)
        || beyond(w + y, center, radius) || beyond(w - y, center, radius)
        || beyond(w, center, radius)) {{
        return;
    }}
    // How large it is on screen, where its center is in front.
    let depth = dot(w.xyz, center) + w.w;
    if (depth > radius) {{
        let pixels = radius * length(y.xyz) / depth * args.viewport.y;
        if (pixels < args.min_pixels) {{
            return;
        }}
    }}
    let slot = atomicAdd(&visible[0], 1u);
    atomicStore(&visible[1u + slot], id.x);
}}

// Workgroups for `count` invocations which each take `per` of them, folded.
fn folded(count: u32, per: u32) -> vec2<u32> {{
    let groups = (count + per - 1u) / per;
    return vec2<u32>(min(groups, FOLD), (groups + FOLD - 1u) / FOLD);
}}

@compute @workgroup_size(1)
fn cs_finish() {{
    let pieces = atomicLoad(&visible[0]) * args.slots;
    let tasks = folded(pieces, PATCHES);
    indirect[{tasks_at}] = tasks.x;
    indirect[{tasks_at} + 1] = tasks.y;
    indirect[{tasks_at} + 2] = 1u;
    indirect[{draw_at}] = args.vertices;
    indirect[{draw_at} + 1] = pieces;
    indirect[{draw_at} + 2] = 0u;
    indirect[{draw_at} + 3] = 0u;
    let count = folded(pieces, COUNT_GROUP);
    indirect[{count_at}] = count.x;
    indirect[{count_at} + 1] = count.y;
    indirect[{count_at} + 2] = 1u;
}}
",
        patches = LodShader::TASK_GROUP,
        count = LodShader::COUNT_GROUP,
        name = row.wgsl,
        words = row.words,
        read = read_element(row, "page", "base"),
        tasks_at = CULL_TASKS,
        draw_at = CULL_DRAW,
        count_at = CULL_COUNT,
    );
    validate("cull", &wgsl)?;
    Ok(wgsl)
}

/// Writes the shader which draws values of a drawable type, and checks
/// that it compiles.
pub fn draw(drawable: &Drawable) -> Result<DrawShader, ShaderError> {
    let ty = &drawable.ty;
    let mut wgsl = String::new();
    let out = &mut wgsl;

    let _ = writeln!(out, "// Generated to draw `{}`.\n", ty.name);
    if drawable.topology == Topology::Surface {
        let _ = writeln!(
            out,
            "struct SurfacePoint {{\n    position: vec3<f32>,\n    normal: vec3<f32>,\n}}\n"
        );
    }
    let _ = writeln!(out, "{DRAW_ARGS}");
    draw_pieces(out, drawable, false);
    let _ = write!(
        out,
        "{}",
        match drawable.topology {
            Topology::Surface => SURFACE_STAGES,
            Topology::Line => LINE_STAGES,
        }
    );

    validate(ty.name, &wgsl)?;
    Ok(DrawShader { wgsl })
}
