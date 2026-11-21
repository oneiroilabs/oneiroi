//! The compute shaders which run the kernels of nodes, and the one which
//! gathers parts together.

use std::fmt::Write;

use crate::{
    node::{GpuKernel, NodeDef},
    shader::{
        BINDLESS, ShaderError, WORKGROUP_SIZE, bindless, declare, layout::write_element,
        read_element, validate_with,
    },
    types::{DataType, InstanceRow, TypeInfo},
};

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

/// A shader which copies runs of words from wherever they are to wherever
/// they go, bound like a [`ComputeShader`]. A call is 5 words: page and
/// offset it copies from, how many words, and page and offset it copies to.
pub const GATHER: &str = include_str!("wgsl/gather.wgsl");

/// How many words a call of [`GATHER`] takes.
pub const GATHER_CALL_WORDS: usize = 5;

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
