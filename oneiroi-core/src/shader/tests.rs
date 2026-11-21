use super::*;
use crate::{
    node::{Caps, GpuKernel, GpuOutput, NodeDef, PortDecl},
    types::{DataType, Field, FieldType, Layout, Scalar},
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
