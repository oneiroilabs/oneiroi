//! Runs the same programs on the CPU and on the GPU and compares.
//!
//! These tests need a graphics adapter. A software one is enough. Without
//! any they pass without having checked anything.

use oneiroi_core::{
    exec::{Engine, Executor, Instance},
    program::{ProgramBuilder, ProgramId},
};
use oneiroi_nodes::curve::{EvaluatedFrame, FRAMES, NURBS, nurbs::clamped_uniform_knots};
use oneiroi_wgpu::{GpuExecutor, GpuStats};

fn gpu() -> Option<Engine<GpuExecutor>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()));
    let Ok(adapter) = adapter else {
        eprintln!("no graphics adapter, nothing was checked");
        return None;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    Some(Engine::new(GpuExecutor::new(device, queue)))
}

/// A curve which bends in every direction.
fn points(count: usize) -> Vec<[f32; 4]> {
    (0..count)
        .map(|i| {
            let t = i as f32;
            [(t * 0.9).sin() * 2.0, t * 0.7, (t * 1.3).cos(), 1.0]
        })
        .collect()
}

/// The frames of a curve through `count` points.
fn program<E: Executor>(engine: &mut Engine<E>, count: usize) -> ProgramId {
    let mut b = ProgramBuilder::new("frames");
    let points = b.param("points", &points(count));
    let knots = b.constant(&clamped_uniform_knots(count));
    let samples = b.param::<u32>("samples", &[8]);
    let segments = b.node(&NURBS, &[points, knots]).unwrap()[0];
    let frames = b.node(&FRAMES, &[segments, samples]).unwrap()[0];
    b.output("frames", frames);
    engine.add_program(b.finish())
}

fn frames<E: Executor>(engine: &mut Engine<E>, instance: &mut Instance) -> Vec<f32> {
    engine.evaluate([&mut *instance]);
    let frames = engine.fetch::<EvaluatedFrame>(instance, "frames").unwrap();
    bytemuck::cast_slice(frames).to_vec()
}

fn assert_close(gpu: &[f32], cpu: &[f32]) {
    assert_eq!(gpu.len(), cpu.len());
    let worst = gpu
        .iter()
        .zip(cpu)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max);
    assert!(worst < 1e-4, "the backends are {worst} apart");
}

#[test]
fn both_backends_compute_the_same_frames() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    let mut cpu = Engine::default();

    for count in [4, 9, 200] {
        let (gpu_program, cpu_program) = (program(&mut gpu, count), program(&mut cpu, count));
        let mut on_gpu = gpu.instantiate(gpu_program);
        let mut on_cpu = cpu.instantiate(cpu_program);

        let a = frames(&mut gpu, &mut on_gpu);
        let b = frames(&mut cpu, &mut on_cpu);
        // 16 floats per frame, 8 frames per segment.
        assert_eq!(a.len(), (count - 3) * 8 * 16);
        assert_close(&a, &b);

        // More samples than one workgroup covers.
        gpu.set_param(&mut on_gpu, "samples", &[100u32]).unwrap();
        cpu.set_param(&mut on_cpu, "samples", &[100u32]).unwrap();
        assert_close(&frames(&mut gpu, &mut on_gpu), &frames(&mut cpu, &mut on_cpu));
    }

    assert!(gpu.executor().errors().is_empty());
}

#[test]
fn only_nodes_with_a_kernel_run_on_the_gpu() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    let program = program(&mut gpu, 9);
    let mut instance = gpu.instantiate(program);

    // The curve is built on the CPU, its frames are computed on the GPU.
    frames(&mut gpu, &mut instance);
    assert_eq!(gpu.executor().stats(), GpuStats { gpu: 1, cpu: 1, downloads: 1 });

    // What is in the store already is not computed again, wherever it
    // came from.
    frames(&mut gpu, &mut instance);
    assert_eq!(gpu.executor().stats(), GpuStats { gpu: 1, cpu: 1, downloads: 1 });

    gpu.set_param(&mut instance, "samples", &[3u32]).unwrap();
    assert_eq!(frames(&mut gpu, &mut instance).len(), 6 * 3 * 16);
    assert_eq!(gpu.executor().stats(), GpuStats { gpu: 2, cpu: 1, downloads: 2 });
}

#[test]
fn edge_cases_match_the_cpu() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    let mut cpu = Engine::default();
    let (gpu_program, cpu_program) = (program(&mut gpu, 9), program(&mut cpu, 9));
    let mut on_gpu = gpu.instantiate(gpu_program);
    let mut on_cpu = cpu.instantiate(cpu_program);

    // Fewer than two samples per segment are two, and so is none at all.
    for samples in [&[0u32][..], &[1], &[]] {
        gpu.set_param(&mut on_gpu, "samples", samples).unwrap();
        cpu.set_param(&mut on_cpu, "samples", samples).unwrap();
        let a = frames(&mut gpu, &mut on_gpu);
        assert_eq!(a.len(), 6 * 2 * 16);
        assert_close(&a, &frames(&mut cpu, &mut on_cpu));
    }

    // Three points are no curve, so there is nothing to run a shader for.
    gpu.set_param(&mut on_gpu, "points", &points(3)).unwrap();
    assert!(frames(&mut gpu, &mut on_gpu).is_empty());

    // A straight line has no curvature to derive a frame from.
    let line: Vec<_> = (0..6).map(|i| [i as f32, 0.0, 0.0, 1.0]).collect();
    gpu.set_param(&mut on_gpu, "points", &line).unwrap();
    cpu.set_param(&mut on_cpu, "points", &line).unwrap();
    gpu.set_param(&mut on_gpu, "samples", &[5u32]).unwrap();
    cpu.set_param(&mut on_cpu, "samples", &[5u32]).unwrap();
    let a = frames(&mut gpu, &mut on_gpu);
    assert!(a.iter().all(|value| value.is_finite()));
    assert_close(&a, &frames(&mut cpu, &mut on_cpu));
}

mod staying_on_the_gpu {
    use glam::Vec3;
    use oneiroi_core::node::{Caps, GpuKernel, Inputs, NodeDef, Outputs, PortDecl, Shape};
    use oneiroi_nodes::curve::nurbs::MovingFrame;

    use super::*;

    fn positions(inputs: &Inputs<'_>, outputs: &mut Outputs) {
        let positions: Vec<_> = inputs
            .get::<EvaluatedFrame>(0)
            .iter()
            .map(|frame| MovingFrame::from(*frame).position)
            .collect();
        outputs.set(0, &positions);
    }

    const INPUTS: &[PortDecl] = &[PortDecl::new::<EvaluatedFrame>("frames")];
    const OUTPUTS: &[PortDecl] = &[PortDecl::new::<Vec3>("positions")];

    /// Reads what another kernel computed.
    static ON_GPU: NodeDef = NodeDef {
        id: "test.positions",
        display_name: "Positions",
        inputs: INPUTS,
        outputs: OUTPUTS,
        shape: Shape::Map,
        cpu: positions,
        gpu: &[GpuKernel {
            wgsl: "fn positions(index: u32) -> vec3<f32> { return in_frames(index).position; }",
            entry: "positions",
            requires: Caps::NONE,
            len: |inputs| inputs.len(0),
            reads: &[],
        }],
    };

    /// The same node without a kernel.
    static ON_CPU: NodeDef = NodeDef {
        id: "test.positions_cpu",
        display_name: "Positions",
        inputs: INPUTS,
        outputs: OUTPUTS,
        shape: Shape::Map,
        cpu: positions,
        gpu: &[],
    };

    fn chain<E: Executor>(engine: &mut Engine<E>, last: &'static NodeDef) -> ProgramId {
        let mut b = ProgramBuilder::new("positions");
        let points = b.param("points", &points(9));
        let knots = b.constant(&clamped_uniform_knots(9));
        let samples = b.constant::<u32>(&[8]);
        let segments = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let frames = b.node(&FRAMES, &[segments, samples]).unwrap()[0];
        let positions = b.node(last, &[frames]).unwrap()[0];
        b.output("frames", frames);
        b.output("positions", positions);
        engine.add_program(b.finish())
    }

    fn reference(points: Option<&[[f32; 4]]>) -> (Vec<Vec3>, Vec<f32>) {
        let mut cpu = Engine::default();
        let program = chain(&mut cpu, &ON_CPU);
        let mut instance = cpu.instantiate(program);
        if let Some(points) = points {
            cpu.set_param(&mut instance, "points", points).unwrap();
        }
        cpu.evaluate([&mut instance]);
        let frames = cpu.read::<EvaluatedFrame>(&instance, "frames").unwrap();
        (
            cpu.read::<Vec3>(&instance, "positions").unwrap().to_vec(),
            bytemuck::cast_slice(frames).to_vec(),
        )
    }

    fn close(gpu: &[Vec3], cpu: &[Vec3]) {
        assert_eq!(gpu.len(), cpu.len());
        assert!(gpu.iter().zip(cpu).all(|(a, b)| a.distance(*b) < 1e-4));
    }

    #[test]
    fn a_kernel_reads_what_another_one_left_on_the_gpu() {
        let Some(mut gpu) = gpu() else {
            return;
        };
        let program = chain(&mut gpu, &ON_GPU);
        let mut instance = gpu.instantiate(program);
        gpu.evaluate([&mut instance]);

        // Two kernels ran and nothing came back.
        assert_eq!(gpu.executor().stats(), GpuStats { gpu: 2, cpu: 1, downloads: 0 });
        assert!(gpu.read::<EvaluatedFrame>(&instance, "frames").is_none());
        assert!(gpu.read::<Vec3>(&instance, "positions").is_none());

        // Only what is asked for is brought back.
        let positions = gpu.fetch::<Vec3>(&instance, "positions").unwrap().to_vec();
        close(&positions, &reference(None).0);
        assert_eq!(gpu.executor().stats().downloads, 1);
        assert!(gpu.read::<EvaluatedFrame>(&instance, "frames").is_none());

        // Once it is here it stays here.
        gpu.fetch::<Vec3>(&instance, "positions").unwrap();
        assert_eq!(gpu.executor().stats().downloads, 1);
        assert!(gpu.read::<Vec3>(&instance, "positions").is_some());
    }

    #[test]
    fn a_node_on_the_cpu_gets_what_it_reads() {
        let Some(mut gpu) = gpu() else {
            return;
        };
        let program = chain(&mut gpu, &ON_CPU);
        let mut instance = gpu.instantiate(program);
        gpu.evaluate([&mut instance]);

        // The frames were brought back for the node which has no kernel.
        assert_eq!(gpu.executor().stats(), GpuStats { gpu: 1, cpu: 2, downloads: 1 });
        close(
            gpu.read::<Vec3>(&instance, "positions").unwrap(),
            &reference(None).0,
        );
    }

    #[test]
    fn the_cpu_can_write_next_to_what_the_gpu_holds() {
        let Some(mut gpu) = gpu() else {
            return;
        };
        let program = chain(&mut gpu, &ON_GPU);
        let mut first = gpu.instantiate(program);
        gpu.evaluate([&mut first]);

        // More curves are built on the CPU and uploaded into the pages
        // which hold the frames of the first one.
        for count in 0..6 {
            let moved: Vec<_> = points(9)
                .into_iter()
                .map(|[x, y, z, w]| [x + count as f32, y, z, w])
                .collect();
            let mut other = gpu.instantiate(program);
            gpu.set_param(&mut other, "points", &moved).unwrap();
            gpu.evaluate([&mut other]);
            close(
                gpu.fetch::<Vec3>(&other, "positions").unwrap(),
                &reference(Some(&moved)).0,
            );
        }

        let frames = gpu.fetch::<EvaluatedFrame>(&first, "frames").unwrap();
        assert_close(bytemuck::cast_slice(frames), &reference(None).1);
    }
}
