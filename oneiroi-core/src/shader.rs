//! Generates the shaders GPU backends run.
//!
//! A node only brings the function which computes one element of its
//! output. Everything around it is written here: the declarations of the
//! data types, the functions which read the inputs out of the pages of the
//! store, and the entry point which writes the output.

use std::fmt::{self, Write};

use crate::{
    node::{Drawable, GpuKernel, NodeDef, Topology},
    types::{DataType, Field, FieldType, InstanceRow, Layout, TypeInfo},
};

/// How many elements one workgroup of a generated compute shader covers.
pub const WORKGROUP_SIZE: u32 = 64;

/// A compute shader which fills the output of one node, for any number
/// of calls of that node at once.
///
/// Every page of the store is bound at once, so a call can read its inputs
/// from wherever they are and write its output right where the store
/// reserved it. It is bound the same for every shader:
///
/// - group 0, binding 0: every page, a read-write `binding_array<Page>`
///   where a `Page` holds `words: array<u32>`
/// - group 1, binding 0: the calls, a read-only `array<u32>` with a dynamic
///   offset of at most [`CALL_WINDOW`] words. A binding array and a
///   dynamic offset can not share a group. A call is
///   [`call_words`](ComputeShader::call_words) long: page, offset and
///   length in words of every input, then page and offset of its output
///   and how many elements that gets.
///
/// There is an entry point `main_k` for every output `k` of the kernel,
/// which fills that output. The inputs of a call are the declared inputs
/// of the node and then the offsets of every grouped one. Dispatch
/// `(ceil(elements / WORKGROUP_SIZE), calls, 1)` workgroups per output,
/// with the elements of the call which has the most.
#[derive(Debug, Clone)]
pub struct ComputeShader {
    pub wgsl: String,
    pub call_words: usize,
}

/// The entry point of the pass which makes stage `index` of a kernel.
pub fn stage_entry(index: usize) -> String {
    format!("stage_{index}")
}

/// How many words of calls one dispatch can see.
pub const CALL_WINDOW: u32 = 1 << 14;

/// What a device needs for the shaders of this module, as `naga` calls it.
fn bindless() -> naga::valid::Capabilities {
    naga::valid::Capabilities::BUFFER_BINDING_ARRAY
        | naga::valid::Capabilities::STORAGE_BUFFER_BINDING_ARRAY
        | naga::valid::Capabilities::STORAGE_BUFFER_BINDING_ARRAY_NON_UNIFORM_INDEXING
}

const BINDLESS: &str = "struct Page {
    words: array<u32>,
}

@group(0) @binding(0) var<storage, read_write> pages: binding_array<Page>;
@group(1) @binding(0) var<storage, read> calls: array<u32>;
";

/// A shader which copies runs of words from wherever they are to wherever
/// they go, bound like a [`ComputeShader`]. A call is 5 words: page and
/// offset it copies from, how many words, and page and offset it copies to.
pub const GATHER: &str = "enable wgpu_binding_array;

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
";

/// How many words a call of [`GATHER`] takes.
pub const GATHER_CALL_WORDS: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShaderError {
    /// A kernel needs an entry for every output, offsets included.
    Outputs { node: &'static str, found: usize },
    /// The shader does not compile. This is a bug in the kernel of the node.
    Invalid { node: &'static str, message: String },
}

impl fmt::Display for ShaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Outputs { node, found } => {
                write!(f, "the kernel of `{node}` has {found} outputs, not one for each")
            }
            Self::Invalid { node, message } => {
                write!(f, "the shader of `{node}` is invalid:\n{message}")
            }
        }
    }
}

impl std::error::Error for ShaderError {}

/// The expression which reads a field out of `page` at word `base + offset`.
fn read(ty: FieldType, page: &str, base: &str, offset: u32) -> String {
    let word = |index: u32| format!("{page}[{base} + {}u]", offset + index);
    let float = |index: u32| format!("bitcast<f32>({})", word(index));
    let floats = |from: u32, count: u32| {
        let parts: Vec<_> = (from..from + count).map(float).collect();
        parts.join(", ")
    };
    match ty {
        FieldType::U32 => word(0),
        FieldType::F32 => float(0),
        FieldType::Vec2 => format!("vec2<f32>({})", floats(0, 2)),
        FieldType::Vec3 => format!("vec3<f32>({})", floats(0, 3)),
        FieldType::Vec4 => format!("vec4<f32>({})", floats(0, 4)),
        FieldType::Mat4 => {
            let columns: Vec<_> = (0..4)
                .map(|column| format!("vec4<f32>({})", floats(column * 4, 4)))
                .collect();
            format!("mat4x4<f32>({})", columns.join(", "))
        }
        FieldType::Struct { wgsl, fields } => read_struct(wgsl, fields, page, base, offset),
        FieldType::Pad(_) => unreachable!("padding is never read"),
    }
}

/// The expression which builds a struct out of its fields.
fn read_struct(wgsl: &str, fields: &[Field], page: &str, base: &str, start: u32) -> String {
    let mut offset = start;
    let mut parts = Vec::new();
    for field in fields {
        if field.ty.wgsl().is_some() {
            parts.push(read(field.ty, page, base, offset));
        }
        offset += field.ty.words();
    }
    format!("{wgsl}({})", parts.join(", "))
}

fn write_struct(out: &mut String, fields: &[Field], value: &str, page: &str, base: &str, start: u32) {
    let mut offset = start;
    for field in fields {
        let value = format!("{value}.{}", field.name);
        write(out, field.ty, &value, page, base, offset);
        offset += field.ty.words();
    }
}

/// The statements which write `value` to `page` at word `base + offset`.
fn write(out: &mut String, ty: FieldType, value: &str, page: &str, base: &str, offset: u32) {
    let mut word = |index: u32, value: String| {
        let _ = writeln!(out, "    {page}[{base} + {}u] = {value};", offset + index);
    };
    match ty {
        FieldType::U32 => word(0, value.into()),
        FieldType::F32 => word(0, format!("bitcast<u32>({value})")),
        FieldType::Vec2 | FieldType::Vec3 | FieldType::Vec4 => {
            for index in 0..ty.words() {
                word(index, format!("bitcast<u32>({value}[{index}])"));
            }
        }
        FieldType::Mat4 => {
            for index in 0..16 {
                let (column, row) = (index / 4, index % 4);
                word(index, format!("bitcast<u32>({value}[{column}][{row}])"));
            }
        }
        FieldType::Pad(words) => {
            for index in 0..words {
                word(index, "0u".into());
            }
        }
        FieldType::Struct { fields, .. } => write_struct(out, fields, value, page, base, offset),
    }
}

/// The expression which reads a whole element of `ty`.
pub(crate) fn read_element(ty: &TypeInfo, page: &str, base: &str) -> String {
    match ty.layout {
        Layout::Value(field) => read(field, page, base, 0),
        Layout::Struct(fields) => read_struct(ty.wgsl, fields, page, base, 0),
    }
}

fn write_element(out: &mut String, ty: &TypeInfo, value: &str, page: &str, base: &str) {
    match ty.layout {
        Layout::Value(field) => write(out, field, value, page, base, 0),
        Layout::Struct(fields) => write_struct(out, fields, value, page, base, 0),
    }
}

fn declare_struct(out: &mut String, declared: &mut Vec<&'static str>, wgsl: &'static str, fields: &'static [Field]) {
    if declared.contains(&wgsl) {
        return;
    }
    declared.push(wgsl);
    // A struct has to come after the ones it is made of.
    for field in fields {
        if let FieldType::Struct { wgsl, fields } = field.ty {
            declare_struct(out, declared, wgsl, fields);
        }
    }
    let _ = writeln!(out, "struct {wgsl} {{");
    for field in fields {
        if let Some(ty) = field.ty.wgsl() {
            let _ = writeln!(out, "    {}: {ty},", field.name);
        }
    }
    let _ = writeln!(out, "}}\n");
}

/// Declares the struct of `ty` and the ones inside of it, each only once.
pub(crate) fn declare(out: &mut String, declared: &mut Vec<&'static str>, ty: &TypeInfo) {
    if let Layout::Struct(fields) = ty.layout {
        declare_struct(out, declared, ty.wgsl, fields);
    }
}

/// Writes the compute shader which runs `kernel` for `node`, and checks
/// that it compiles.
pub fn compute(node: &NodeDef, kernel: &GpuKernel) -> Result<ComputeShader, ShaderError> {
    // The declared outputs, then the offsets of those with their own groups.
    let outputs: Vec<TypeInfo> = node
        .outputs
        .iter()
        .map(|port| port.ty)
        .chain(node.hidden_outputs().map(|_| u32::INFO))
        .chain(node.total_outputs().map(|_| u32::INFO))
        .collect();
    if kernel.outputs.len() != outputs.len() {
        return Err(ShaderError::Outputs {
            node: node.id,
            found: kernel.outputs.len(),
        });
    }
    let hidden: Vec<usize> = node.hidden_inputs().collect();
    let prototypes: Vec<usize> = node.prototype_inputs().collect();
    let inputs = node.all_inputs();
    let mut wgsl = String::new();
    let out = &mut wgsl;

    let _ = writeln!(out, "enable wgpu_binding_array;\n");
    let _ = writeln!(out, "// Generated for `{}`.\n", node.id);
    let mut declared = Vec::new();
    let stages = kernel.stages;
    for ty in node
        .inputs
        .iter()
        .map(|port| &port.ty)
        .chain(&outputs)
        .chain(stages.iter().map(|stage| &stage.ty))
        .chain(prototypes.iter().filter_map(|index| node.inputs[*index].ty.prototype))
    {
        declare(out, &mut declared, ty);
    }

    // A call is every input, then every stage, then where the pass writes.
    let call_words = (inputs + stages.len()) * 3 + 3;
    let _ = writeln!(out, "{BINDLESS}");
    let _ = writeln!(out, "// Where the call this invocation works on starts in `calls`.");
    let _ = writeln!(out, "var<private> call: u32;\n");

    for (index, port) in node.inputs.iter().enumerate() {
        let (name, ty, words) = (port.name, port.ty.wgsl, port.ty.words);
        let (page, offset, len) = (index * 3, index * 3 + 1, index * 3 + 2);
        let _ = writeln!(
            out,
            "fn in_{name}_len() -> u32 {{\n    return calls[call + {len}u] / {words}u;\n}}\n"
        );
        let element = read_element(&port.ty, "pages[page].words", "base");
        let _ = writeln!(
            out,
            "fn in_{name}(index: u32) -> {ty} {{\n    let page = calls[call + {page}u];\n    let base = calls[call + {offset}u] + index * {words}u;\n    return {element};\n}}\n"
        );
    }

    // The groups of grouped inputs, from their offsets. No offsets at all
    // are a single group of everything.
    for (place, index) in hidden.iter().enumerate() {
        let name = node.inputs[*index].name;
        let at = (node.inputs.len() + place) * 3;
        let _ = writeln!(
            out,
            "fn in_{name}_offset(group: u32) -> u32 {{
    return pages[calls[call + {at}u]].words[calls[call + {}u] + group];
}}

fn in_{name}_groups() -> u32 {{
    let offsets = calls[call + {}u];
    if (offsets == 0u) {{
        return 1u;
    }}
    return offsets - 1u;
}}

fn in_{name}_start(group: u32) -> u32 {{
    if (calls[call + {}u] == 0u) {{
        return 0u;
    }}
    return min(in_{name}_offset(group), in_{name}_len());
}}

fn in_{name}_end(group: u32) -> u32 {{
    if (calls[call + {}u] == 0u) {{
        return in_{name}_len();
    }}
    return min(in_{name}_offset(group + 1u), in_{name}_len());
}}

// The group an element is in, by a binary search through the offsets.
fn in_{name}_group_of(index: u32) -> u32 {{
    var low = 0u;
    var high = in_{name}_groups();
    while (high - low > 1u) {{
        let middle = (low + high) / 2u;
        if (in_{name}_start(middle) <= index) {{
            low = middle;
        }} else {{
            high = middle;
        }}
    }}
    return low;
}}
",
            at + 1,
            at + 2,
            at + 2,
            at + 2,
        );
    }
    // An input which is not grouped is a single group of everything, read
    // the same way: a kernel which makes a thing for each group of a list
    // makes the one thing of a single one with no change.
    for (index, port) in node.inputs.iter().enumerate() {
        if hidden.contains(&index) || port.ty.prototype.is_some() {
            continue;
        }
        let name = port.name;
        let _ = writeln!(
            out,
            "fn in_{name}_groups() -> u32 {{
    return 1u;
}}

fn in_{name}_start(group: u32) -> u32 {{
    return 0u;
}}

fn in_{name}_end(group: u32) -> u32 {{
    return in_{name}_len();
}}

fn in_{name}_group_of(index: u32) -> u32 {{
    return 0u;
}}
"
        );
    }

    // The shapes instances refer to, and their groups, read like inputs.
    for (place, index) in prototypes.iter().enumerate() {
        let port = &node.inputs[*index];
        let name = port.name;
        let shape = port.ty.prototype.expect("only instances have a shape");
        let (ty, words) = (shape.wgsl, shape.words);
        let at = (node.inputs.len() + hidden.len() + 3 * place) * 3;
        let groups = at + 3;
        let total = at + 6;
        let element = read_element(shape, "pages[page].words", "base");
        let _ = writeln!(
            out,
            "fn in_{name}_prototype_len() -> u32 {{
    return calls[call + {len}u] / {words}u;
}}

fn in_{name}_prototype(index: u32) -> {ty} {{
    let page = calls[call + {at}u];
    let base = calls[call + {offset}u] + index * {words}u;
    return {element};
}}

fn in_{name}_prototype_groups() -> u32 {{
    let offsets = calls[call + {groups_len}u];
    if (offsets == 0u) {{
        return 1u;
    }}
    return offsets - 1u;
}}

fn in_{name}_prototype_start(group: u32) -> u32 {{
    if (calls[call + {groups_len}u] == 0u) {{
        return 0u;
    }}
    return min(pages[calls[call + {groups}u]].words[calls[call + {groups_offset}u] + group], in_{name}_prototype_len());
}}

fn in_{name}_prototype_end(group: u32) -> u32 {{
    if (calls[call + {groups_len}u] == 0u) {{
        return in_{name}_prototype_len();
    }}
    return min(pages[calls[call + {groups}u]].words[calls[call + {groups_offset}u] + group + 1u], in_{name}_prototype_len());
}}

// How many elements all copies of the instances have together, and how
// their rows repeat: see `InstanceTotal`.
fn in_{name}_total_word(word: u32) -> u32 {{
    if (calls[call + {total_len}u] <= word) {{
        return 0u;
    }}
    return pages[calls[call + {total}u]].words[calls[call + {total_offset}u] + word];
}}

fn in_{name}_total() -> u32 {{
    return in_{name}_total_word(0u);
}}

// The row the element `index` of all copies belongs to: the block it is
// in by dividing, and then the row among the rows of the first block.
fn in_{name}_row_of(index: u32) -> u32 {{
    var rows = in_{name}_total_word(1u);
    var elements = in_{name}_total_word(2u);
    var block = 0u;
    var within = index;
    if (rows == 0u || elements == 0u) {{
        rows = in_{name}_len();
    }} else {{
        block = index / elements;
        within = index % elements;
    }}
    var low = 0u;
    var high = rows;
    while (high - low > 1u) {{
        let middle = (low + high) / 2u;
        // Only the word which says where the row starts.
        let first = pages[calls[call + {rows_page}u]].words[calls[call + {rows_offset}u] + middle * {row_words}u + {first_word}u];
        if (first <= within) {{
            low = middle;
        }} else {{
            high = middle;
        }}
    }}
    return block * rows + low;
}}
",
            len = at + 2,
            offset = at + 1,
            groups_len = groups + 2,
            groups_offset = groups + 1,
            total_len = total + 2,
            total_offset = total + 1,
            rows_page = *index * 3,
            rows_offset = *index * 3 + 1,
            row_words = InstanceRow::INFO.words,
            first_word = InstanceRow::INFO.words - 1,
        );
    }

    // The stages, read like inputs.
    for (index, stage) in stages.iter().enumerate() {
        let (name, ty, words) = (stage.name, stage.ty.wgsl, stage.ty.words);
        let at = (inputs + index) * 3;
        let _ = writeln!(
            out,
            "fn stage_{name}_len() -> u32 {{\n    return calls[call + {}u] / {words}u;\n}}\n",
            at + 2
        );
        let element = read_element(&stage.ty, "pages[page].words", "base");
        let _ = writeln!(
            out,
            "fn stage_{name}(index: u32) -> {ty} {{\n    let page = calls[call + {at}u];\n    let base = calls[call + {}u] + index * {words}u;\n    return {element};\n}}\n",
            at + 1
        );
    }

    let _ = writeln!(out, "{}\n", kernel.wgsl.trim());

    // An entry point for every stage made on the GPU and every output,
    // which fills one element of it.
    let output = (inputs + stages.len()) * 3;
    let passes = stages
        .iter()
        .enumerate()
        .filter(|(_, stage)| stage.words.is_none())
        .map(|(index, stage)| (stage_entry(index), stage.ty, stage.entry))
        .chain(
            outputs
                .iter()
                .zip(kernel.outputs)
                .enumerate()
                .map(|(index, (ty, entry))| (format!("main_{index}"), *ty, entry.entry)),
        );
    for (main, ty, entry) in passes {
        let ty = &ty;
        let _ = writeln!(
            out,
            "@compute @workgroup_size({WORKGROUP_SIZE})\nfn {main}(\n    @builtin(global_invocation_id) id: vec3<u32>,\n    @builtin(workgroup_id) group: vec3<u32>,\n) {{"
        );
        let _ = writeln!(out, "    call = group.y * {call_words}u;");
        let _ = writeln!(
            out,
            "    if (id.x >= calls[call + {}u]) {{\n        return;\n    }}",
            output + 2
        );
        let _ = writeln!(out, "    let value = {entry}(id.x);");
        let _ = writeln!(out, "    let page = calls[call + {output}u];");
        let _ = writeln!(
            out,
            "    let base = calls[call + {}u] + id.x * {}u;",
            output + 1,
            ty.words
        );
        write_element(out, ty, "value", "pages[page].words", "base");
        let _ = writeln!(out, "}}\n");
    }

    validate_with(node.id, &wgsl, bindless())?;
    Ok(ComputeShader { wgsl, call_words })
}

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

/// The uniform of a [`DrawShader`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawArgs {
    /// Column-major, as WGSL wants it.
    pub view_projection: [[f32; 4]; 4],
    /// Points from what is drawn towards the light.
    pub light: [f32; 4],
    pub color: [f32; 4],
    /// The word the first element starts at in the page.
    pub offset: u32,
    /// Steps along every element.
    pub rings: u32,
    /// Steps across every patch of a surface.
    pub sides: u32,
    pub _padding: u32,
}

const DRAW_ARGS: &str = "struct DrawArgs {
    view_projection: mat4x4<f32>,
    light: vec4<f32>,
    color: vec4<f32>,
    offset: u32,
    rings: u32,
    sides: u32,
}

@group(0) @binding(0) var<storage, read> page: array<u32>;
@group(0) @binding(1) var<uniform> args: DrawArgs;
";

const SURFACE_STAGES: &str = "struct VertexOutput {
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
";

const LINE_STAGES: &str = "@vertex
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
";

/// What a draw shader of instances has besides the rest: where the rows
/// and the offsets of the groups of the shape are, and turning.
const INSTANCED: &str = "struct InstanceArgs {
    // The word the first row starts at, in `instance_rows`.
    rows_offset: u32,
    rows: u32,
    // The word the offsets of the groups of the shape start at, and how
    // many there are. None at all are a single group of everything.
    groups_offset: u32,
    groups: u32,
    // How many elements the shape has, and its longest group.
    elements: u32,
    slots: u32,
}

@group(0) @binding(3) var<storage, read> instance_rows: array<u32>;
@group(0) @binding(4) var<storage, read> instance_groups: array<u32>;
@group(0) @binding(5) var<uniform> instance_args: InstanceArgs;
// How many instances are visible, and then which rows they are: see
// `cull`, which made it before.
@group(0) @binding(6) var<storage, read> instance_visible: array<u32>;

fn instance_rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

fn instance_place(t: Similarity, p: vec3<f32>) -> vec3<f32> {
    return instance_rotate(t.rotation, p) * t.scale + t.offset;
}

fn instance_group_start(group: u32) -> u32 {
    if (instance_args.groups == 0u) {
        return 0u;
    }
    return min(instance_groups[instance_args.groups_offset + group], instance_args.elements);
}

fn instance_group_end(group: u32) -> u32 {
    if (instance_args.groups == 0u) {
        return instance_args.elements;
    }
    return min(instance_groups[instance_args.groups_offset + group + 1u], instance_args.elements);
}
";

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
fn draw_pieces(out: &mut String, drawable: &Drawable, lod: bool) -> &'static str {
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

/// The uniform of a draw shader of instances, at binding 5.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceArgs {
    pub rows_offset: u32,
    pub rows: u32,
    pub groups_offset: u32,
    pub groups: u32,
    /// How many elements the shape has.
    pub elements: u32,
    /// How many elements its longest group has.
    pub slots: u32,
    pub _padding: [u32; 2],
}

/// The uniform of [`cull`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CullArgs {
    /// Column-major, as WGSL wants it.
    pub view_projection: [[f32; 4]; 4],
    /// The size in pixels of what is drawn into.
    pub viewport: [f32; 2],
    /// Instances smaller than this many pixels across are not drawn.
    pub min_pixels: f32,
    /// The word the first row starts at in its page, and how many there are.
    pub rows_offset: u32,
    pub rows: u32,
    /// How many places for pieces each visible instance gets: see
    /// [`InstanceArgs::slots`].
    pub slots: u32,
    /// How many vertices a draw without mesh shaders has per piece.
    pub vertices: u32,
    pub _padding: u32,
}

/// Where [`cull`] writes the arguments of what comes after it, in words of
/// the indirect buffer: task shaders, a draw without them, and counting.
pub const CULL_TASKS: u64 = 0;
pub const CULL_DRAW: u64 = 4;
pub const CULL_COUNT: u64 = 8;
/// How long the indirect buffer is, in words.
pub const CULL_WORDS: u64 = 12;

/// How many workgroups one dimension of a dispatch takes, the rest goes on
/// in the next one: see `FOLD` in the shaders.
pub const FOLD: u32 = 32768;

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

/// The uniform of a [`LodShader`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LodArgs {
    /// Column-major, as WGSL wants it.
    pub view_projection: [[f32; 4]; 4],
    /// Points from what is drawn towards the light.
    pub light: [f32; 4],
    pub color: [f32; 4],
    /// The size in pixels of what is drawn into.
    pub viewport: [f32; 2],
    /// How long a step along an edge should be on screen, in pixels.
    pub pixels: f32,
    /// The word the first patch starts at in the page.
    pub offset: u32,
    /// Patches never get finer than `2^max_level` steps along a side.
    /// At most [`MAX_LOD_LEVEL`].
    pub max_level: u32,
    /// How surfaces are coloured, see [`LodColors`].
    pub debug: u32,
    /// How many patches there are.
    pub patches: u32,
    pub _padding: u32,
}

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

const LOD_COMMON: &str = "// Workgroups which go beyond what one dimension can count go on in the
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
";

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

const LOD_VERTEX: &str = "@vertex
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
";

const LOD_MESH: &str = "// A large patch is cut into blocks of this many cells along each side,
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
";

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

fn validate(node: &'static str, wgsl: &str) -> Result<(), ShaderError> {
    validate_with(node, wgsl, naga::valid::Capabilities::empty())
}

fn validate_with(
    node: &'static str,
    wgsl: &str,
    capabilities: naga::valid::Capabilities,
) -> Result<(), ShaderError> {
    let invalid = |message| ShaderError::Invalid { node, message };
    let module =
        naga::front::wgsl::parse_str(wgsl).map_err(|error| invalid(error.emit_to_string(wgsl)))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        capabilities,
    )
    .validate(&module)
    .map_err(|error| invalid(error.emit_to_string(wgsl)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        node::{Caps, GpuOutput, PortDecl},
        types::{DataType, Scalar},
    };

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Particle {
        position: [f32; 3],
        _padding: f32,
        transform: [[f32; 4]; 4],
        id: u32,
    }

    impl DataType for Particle {
        type ConfigurationOptions = ();

        const NAME: &'static str = "particle";
        const WGSL: &'static str = "Particle";
        const SCALAR: Scalar = Scalar::F32;
        const LAYOUT: Layout = Layout::Struct(&[
            Field::new("position", FieldType::Vec3),
            Field::pad(1),
            Field::new("transform", FieldType::Mat4),
            Field::new("id", FieldType::U32),
        ]);
    }

    fn node(wgsl: &'static str) -> (NodeDef, GpuKernel) {
        const INPUTS: &[PortDecl] = &[
            PortDecl::new::<Particle>("particles"),
            PortDecl::new::<f32>("scale"),
        ];
        const OUTPUTS: &[PortDecl] = &[PortDecl::new::<Particle>("particles")];
        let node = NodeDef {
            id: "test.grow",
            inputs: INPUTS,
            outputs: OUTPUTS,
            version: 0,
            cpu: |_, _| {},
            gpu: &[],
        };
        let kernel = GpuKernel {
            wgsl,
            outputs: &[GpuOutput {
                entry: "grow",
                len: |inputs| inputs.len(0),
                words: None,
            }],
            requires: Caps::NONE,
            reads: &[],
            accepts: None,
            examples: &[],
            stages: &[],
        };
        (node, kernel)
    }

    const GROW: &str = "
        fn grow(index: u32) -> Particle {
            var particle = in_particles(index);
            particle.position *= in_scale(0u);
            particle.id = in_particles_len();
            return particle;
        }";

    #[test]
    fn the_shader_around_a_kernel_compiles() {
        let (node, kernel) = node(GROW);
        let shader = compute(&node, &kernel).unwrap().wgsl;

        // The type is declared once, without its padding.
        assert_eq!(shader.matches("struct Particle {").count(), 1);
        assert!(!shader.contains("_padding"));
        // Both inputs can be read.
        assert!(shader.contains("fn in_particles(index: u32) -> Particle"));
        assert!(shader.contains("fn in_scale(index: u32) -> f32"));
        // A call names where both inputs are and where the output goes.
        assert_eq!(compute(&node, &kernel).unwrap().call_words, 9);
        assert!(shader.contains("call = group.y * 9u;"));
        // 21 words per particle: the padding is skipped when reading and
        // zeroed when writing.
        assert!(shader.contains("index * 21u"));
        assert!(shader.contains("pages[page].words[base + 3u] = 0u;"));
        assert!(shader.contains("pages[page].words[base + 20u] = value.id;"));
    }

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Pair {
        first: Particle,
        weight: f32,
        second: Particle,
    }

    impl DataType for Pair {
        type ConfigurationOptions = ();

        const NAME: &'static str = "pair";
        const WGSL: &'static str = "Pair";
        const SCALAR: Scalar = Scalar::F32;
        const LAYOUT: Layout = Layout::Struct(&[
            Field::nested("first", Particle::INFO),
            Field::nested("weight", f32::INFO),
            Field::nested("second", Particle::INFO),
        ]);
    }

    #[test]
    fn structs_can_be_made_of_structs() {
        const INPUTS: &[PortDecl] = &[PortDecl::new::<Pair>("pairs")];
        const OUTPUTS: &[PortDecl] = &[PortDecl::new::<Pair>("pairs")];
        let node = NodeDef {
            id: "test.swap",
            inputs: INPUTS,
            outputs: OUTPUTS,
            version: 0,
            cpu: |_, _| {},
            gpu: &[],
        };
        let kernel = GpuKernel {
            wgsl: "fn swap(index: u32) -> Pair {
                let pair = in_pairs(index);
                return Pair(pair.second, pair.weight, pair.first);
            }",
            outputs: &[GpuOutput {
                entry: "swap",
                len: |inputs| inputs.len(0),
                words: None,
            }],
            requires: Caps::NONE,
            reads: &[],
            accepts: None,
            examples: &[],
            stages: &[],
        };
        let shader = compute(&node, &kernel).unwrap().wgsl;

        // The inner struct is declared once and before the outer one.
        assert_eq!(shader.matches("struct Particle {").count(), 1);
        assert!(shader.find("struct Particle {") < shader.find("struct Pair {"));
        // 21 + 1 + 21 words, with the second particle behind the weight.
        assert!(shader.contains("index * 43u"));
        assert!(shader.contains("pages[page].words[base + 42u] = value.second.id;"));
        assert!(shader.contains("pages[page].words[base + 21u] = bitcast<u32>(value.weight);"));
    }

    #[test]
    fn a_broken_kernel_is_reported_with_its_node() {
        let (node, kernel) = node("fn grow(index: u32) -> Particle { return in_nothing(index); }");
        let Err(ShaderError::Invalid { node, message }) = compute(&node, &kernel) else {
            panic!("the kernel calls a function which does not exist");
        };
        assert_eq!(node, "test.grow");
        assert!(message.contains("in_nothing"), "{message}");

        // A kernel of the wrong type does not get past the validation.
        let (node, kernel) = self::node("fn grow(index: u32) -> f32 { return 1.0; }");
        assert!(matches!(
            compute(&node, &kernel),
            Err(ShaderError::Invalid { .. })
        ));
    }

    #[test]
    fn the_gather_shader_compiles() {
        validate_with("gather", GATHER, bindless()).unwrap();
    }

    #[test]
    fn every_output_needs_an_entry() {
        let (mut node, kernel) = node(GROW);
        node.outputs = &[];
        assert_eq!(
            compute(&node, &kernel).unwrap_err(),
            ShaderError::Outputs {
                node: "test.grow",
                found: 1
            }
        );
    }

    #[test]
    fn groups_can_be_read_and_made() {
        // The length of every group, as many groups as the input has.
        const INPUTS: &[PortDecl] = &[PortDecl::new::<f32>("values").grouped()];
        const OUTPUTS: &[PortDecl] = &[PortDecl::new::<u32>("lengths").grouped()];
        let node = NodeDef {
            id: "test.lengths",
            version: 0,
            inputs: INPUTS,
            outputs: OUTPUTS,
            cpu: |_, _| {},
            gpu: &[],
        };
        let kernel = GpuKernel {
            wgsl: "
                fn length_of(group: u32) -> u32 {
                    return in_values_end(group) - in_values_start(group);
                }
                fn one_each(group: u32) -> u32 {
                    return group + in_values_group_of(0u) * 0u;
                }",
            outputs: &[
                GpuOutput {
                    entry: "length_of",
                    len: |_| 0,
                    words: None,
                },
                GpuOutput {
                    entry: "one_each",
                    len: |_| 0,
                    words: None,
                },
            ],
            requires: Caps::NONE,
            reads: &[],
            accepts: None,
            examples: &[],
            stages: &[],
        };
        let shader = compute(&node, &kernel).unwrap();
        // The values and their offsets, and where the output goes.
        assert_eq!(shader.call_words, 9);
        assert!(shader.wgsl.contains("fn main_0(") && shader.wgsl.contains("fn main_1("));
        assert!(shader.wgsl.contains("fn in_values_group_of(index: u32) -> u32"));
    }
}
