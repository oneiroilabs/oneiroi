//! Every node which has a kernel computes on the GPU what it computes on
//! the CPU, for every example the kernel brings along.
//!
//! This needs a graphics adapter. A software one is enough. Without any
//! it passes without having checked anything.

use oneiroi_core::{
    exec::{Engine, Executor, Instance},
    node::{Card, NodeDef, NodeRegistry},
    program::{ProgramBuilder, ProgramId, groups_param},
    types::Scalar,
};
use oneiroi_wgpu::GpuExecutor;

fn gpu() -> Option<Engine<GpuExecutor>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()));
    let Ok(adapter) = adapter else {
        eprintln!("no graphics adapter, nothing was checked");
        return None;
    };
    let descriptor = wgpu::DeviceDescriptor {
        required_features: adapter.features() & oneiroi_wgpu::FEATURES,
        required_limits: adapter.limits(),
        ..Default::default()
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&descriptor)).ok()?;
    if !GpuExecutor::supports(&device) {
        eprintln!("no binding arrays, nothing was checked");
        return None;
    }
    Some(Engine::new(GpuExecutor::new(device, queue)))
}

/// The node alone, with a parameter for every input. Every output is an
/// output of the program under the name of its port.
fn program<E: Executor>(engine: &mut Engine<E>, node: &'static NodeDef) -> ProgramId {
    let mut b = ProgramBuilder::new(node.id);
    let inputs: Vec<_> = node
        .inputs
        .iter()
        .map(|port| {
            let default = port.default.words();
            match port.card {
                Card::One => b.param_one(port.name, port.ty, default),
                Card::Many => b.param_words(port.name, port.ty, default),
                Card::Grouped => b.param_grouped(port.name, port.ty, default),
            }
        })
        .collect();
    let outputs = b.node(node, &inputs).unwrap();
    for (port, slot) in node.outputs.iter().zip(outputs) {
        b.output(port.name, slot);
    }
    engine.add_program(b.finish())
}

/// Hands the words of an example to the parameters, see
/// [`GpuKernel::examples`](oneiroi_core::node::GpuKernel::examples).
fn set<E: Executor>(engine: &Engine<E>, instance: &mut Instance, node: &NodeDef, example: &[Vec<u32>]) {
    let grouped = node.inputs.iter().filter(|port| port.card == Card::Grouped);
    let names = node
        .inputs
        .iter()
        .map(|port| port.name.to_string())
        .chain(grouped.map(|port| groups_param(port.name)));
    let mut given = 0;
    for (name, words) in names.zip(example) {
        engine.set_param_words(instance, &name, words).unwrap();
        given += 1;
    }
    assert_eq!(given, example.len(), "`{}` has an example with too many inputs", node.id);
}

fn close(node: &NodeDef, name: &str, scalar: Scalar, gpu: &[u32], cpu: &[u32]) {
    assert_eq!(gpu.len(), cpu.len(), "`{}`, `{name}`: the lengths differ", node.id);
    for (index, (a, b)) in gpu.iter().zip(cpu).enumerate() {
        let same = match scalar {
            Scalar::F32 => {
                let (a, b) = (f32::from_bits(*a), f32::from_bits(*b));
                (a - b).abs() <= 1e-4 * (1.0 + b.abs()) || (a.is_nan() && b.is_nan())
            }
            _ => a == b,
        };
        assert!(same, "`{}`, `{name}`: word {index} is {a:#x} on the GPU and {b:#x} on the CPU", node.id);
    }
}

#[test]
fn every_kernel_computes_what_the_cpu_computes() {
    let mut registry = NodeRegistry::new();
    oneiroi_nodes::register(&mut registry);
    let Some(mut gpu) = gpu() else {
        return;
    };

    let mut checked = 0;
    for node in registry.iter() {
        let Some(kernel) = node.gpu.first() else {
            continue;
        };
        assert!(!kernel.examples.is_empty(), "the kernel of `{}` has no examples", node.id);

        let mut cpu = Engine::default();
        let before = gpu.executor().stats().gpu;
        let (on_gpu, on_cpu) = (program(&mut gpu, node), program(&mut cpu, node));
        for example in kernel.examples {
            let example = example();
            let (mut a, mut b) = (gpu.instantiate(on_gpu), cpu.instantiate(on_cpu));
            set(&gpu, &mut a, node, &example);
            set(&cpu, &mut b, node, &example);

            gpu.evaluate([&mut a]);
            cpu.evaluate([&mut b]);

            for port in node.outputs {
                let words = gpu.fetch::<u32>(&a, port.name).unwrap().to_vec();
                let expected = cpu.fetch::<u32>(&b, port.name).unwrap();
                close(node, port.name, port.ty.scalar, &words, expected);
                let groups = gpu.fetch_groups(&a, port.name).map(<[u32]>::to_vec);
                assert_eq!(groups.as_deref(), cpu.fetch_groups(&b, port.name), "`{}`, `{}`: the groups differ", node.id, port.name);
            }
            checked += 1;
        }
        // Some examples may be left to the CPU, but not all of them.
        assert!(gpu.executor().stats().gpu > before, "`{}` never ran on the GPU", node.id);
    }
    assert!(checked > 0);
}
