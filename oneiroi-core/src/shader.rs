//! Generates the shaders GPU backends run.
//!
//! A node only brings the function which computes one element of its
//! output. Everything around it is written here: the declarations of the
//! data types, the functions which read the inputs out of the pages of the
//! store, and the entry point which writes the output.

use std::fmt::{self, Write};

use crate::{
    node::{GpuKernel, NodeDef},
    types::{Field, FieldType, Layout, TypeInfo},
};

/// How many elements one workgroup of a generated compute shader covers.
pub const WORKGROUP_SIZE: u32 = 64;

/// A compute shader which fills the output of one node, for any number
/// of calls of that node at once.
///
/// Its entry point is `main` and everything is bound in group 0. For a
/// node with `n` inputs:
///
/// - bindings `0..n`: the page input `i` of every call lives in,
///   read-only `array<u32>`
/// - binding `n`: where the outputs go, read-write `array<u32>`
/// - binding `n + 1`: the calls, read-only `array<u32>`. A call is
///   [`call_words`](ComputeShader::call_words) long: the offset and the
///   length in words of each input, then the offset in words of its output
///   and how many elements that has.
///
/// Dispatch `(ceil(elements / WORKGROUP_SIZE), calls, 1)` workgroups,
/// with the elements of the call which has the most.
#[derive(Debug, Clone)]
pub struct ComputeShader {
    pub wgsl: String,
    pub call_words: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShaderError {
    /// Only nodes with exactly one output can be generated for.
    Outputs { node: &'static str, found: usize },
    /// The shader does not compile. This is a bug in the kernel of the node.
    Invalid { node: &'static str, message: String },
}

impl fmt::Display for ShaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Outputs { node, found } => {
                write!(f, "`{node}` has {found} outputs, a kernel fills exactly one")
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
    let [output] = node.outputs else {
        return Err(ShaderError::Outputs {
            node: node.id,
            found: node.outputs.len(),
        });
    };
    let inputs = node.inputs.len();
    let mut wgsl = String::new();
    let out = &mut wgsl;

    let _ = writeln!(out, "// Generated for `{}`.\n", node.id);
    let mut declared = Vec::new();
    for ty in node.inputs.iter().map(|port| &port.ty).chain([&output.ty]) {
        declare(out, &mut declared, ty);
    }

    let call_words = inputs * 2 + 2;
    for index in 0..inputs {
        let _ = writeln!(
            out,
            "@group(0) @binding({index}) var<storage, read> page_{index}: array<u32>;"
        );
    }
    let _ = writeln!(
        out,
        "@group(0) @binding({inputs}) var<storage, read_write> out_page: array<u32>;"
    );
    let _ = writeln!(
        out,
        "@group(0) @binding({}) var<storage, read> calls: array<u32>;\n",
        inputs + 1
    );
    let _ = writeln!(out, "// Where the call this invocation works on starts in `calls`.");
    let _ = writeln!(out, "var<private> call: u32;\n");

    for (index, port) in node.inputs.iter().enumerate() {
        let (name, ty, words) = (port.name, port.ty.wgsl, port.ty.words);
        let (offset, len) = (index * 2, index * 2 + 1);
        let _ = writeln!(
            out,
            "fn in_{name}_len() -> u32 {{\n    return calls[call + {len}u] / {words}u;\n}}\n"
        );
        let element = read_element(&port.ty, &format!("page_{index}"), "base");
        let _ = writeln!(
            out,
            "fn in_{name}(index: u32) -> {ty} {{\n    let base = calls[call + {offset}u] + index * {words}u;\n    return {element};\n}}\n"
        );
    }

    let _ = writeln!(out, "{}\n", kernel.wgsl.trim());

    let _ = writeln!(
        out,
        "@compute @workgroup_size({WORKGROUP_SIZE})\nfn main(@builtin(global_invocation_id) id: vec3<u32>) {{"
    );
    let _ = writeln!(out, "    call = id.y * {call_words}u;");
    let _ = writeln!(
        out,
        "    if (id.x >= calls[call + {}u]) {{\n        return;\n    }}",
        inputs * 2 + 1
    );
    let _ = writeln!(out, "    let value = {}(id.x);", kernel.entry);
    let _ = writeln!(
        out,
        "    let base = calls[call + {}u] + id.x * {}u;",
        inputs * 2,
        output.ty.words
    );
    write_element(out, &output.ty, "value", "out_page", "base");
    let _ = writeln!(out, "}}");

    validate(node.id, &wgsl)?;
    Ok(ComputeShader { wgsl, call_words })
}

fn validate(node: &'static str, wgsl: &str) -> Result<(), ShaderError> {
    let invalid = |message| ShaderError::Invalid { node, message };
    let module =
        naga::front::wgsl::parse_str(wgsl).map_err(|error| invalid(error.emit_to_string(wgsl)))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map_err(|error| invalid(error.emit_to_string(wgsl)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        node::{Caps, PortDecl, Shape},
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
            display_name: "Grow",
            inputs: INPUTS,
            outputs: OUTPUTS,
            shape: Shape::Map,
            cpu: |_, _| {},
            gpu: &[],
        };
        let kernel = GpuKernel {
            wgsl,
            entry: "grow",
            requires: Caps::NONE,
            len: |inputs| inputs.len(0),
            reads: &[],
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
        assert_eq!(compute(&node, &kernel).unwrap().call_words, 6);
        assert!(shader.contains("call = id.y * 6u;"));
        // 21 words per particle: the padding is skipped when reading and
        // zeroed when writing.
        assert!(shader.contains("index * 21u"));
        assert!(shader.contains("out_page[base + 3u] = 0u;"));
        assert!(shader.contains("out_page[base + 20u] = value.id;"));
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
            display_name: "Swap",
            inputs: INPUTS,
            outputs: OUTPUTS,
            shape: Shape::Map,
            cpu: |_, _| {},
            gpu: &[],
        };
        let kernel = GpuKernel {
            wgsl: "fn swap(index: u32) -> Pair {
                let pair = in_pairs(index);
                return Pair(pair.second, pair.weight, pair.first);
            }",
            entry: "swap",
            requires: Caps::NONE,
            len: |inputs| inputs.len(0),
            reads: &[],
        };
        let shader = compute(&node, &kernel).unwrap().wgsl;

        // The inner struct is declared once and before the outer one.
        assert_eq!(shader.matches("struct Particle {").count(), 1);
        assert!(shader.find("struct Particle {") < shader.find("struct Pair {"));
        // 21 + 1 + 21 words, with the second particle behind the weight.
        assert!(shader.contains("index * 43u"));
        assert!(shader.contains("out_page[base + 42u] = value.second.id;"));
        assert!(shader.contains("out_page[base + 21u] = bitcast<u32>(value.weight);"));
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
    fn kernels_fill_exactly_one_output() {
        let (mut node, kernel) = node(GROW);
        node.outputs = &[];
        assert_eq!(
            compute(&node, &kernel).unwrap_err(),
            ShaderError::Outputs {
                node: "test.grow",
                found: 0
            }
        );
    }
}
