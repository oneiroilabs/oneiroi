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

/// Nodes on the GPU, nodes on the CPU, compute passes and downloads.
fn counts(stats: GpuStats) -> (usize, usize, usize, usize) {
    (stats.gpu, stats.cpu, stats.dispatches, stats.downloads)
}

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
    assert_eq!(counts(gpu.executor().stats()), (1, 1, 1, 1));

    // What is in the store already is not computed again, wherever it
    // came from.
    frames(&mut gpu, &mut instance);
    assert_eq!(counts(gpu.executor().stats()), (1, 1, 1, 1));

    gpu.set_param(&mut instance, "samples", &[3u32]).unwrap();
    assert_eq!(frames(&mut gpu, &mut instance).len(), 6 * 3 * 16);
    assert_eq!(counts(gpu.executor().stats()), (2, 1, 2, 2));
}

#[test]
fn calls_of_one_node_share_a_pass() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    let mut cpu = Engine::default();

    // Curves of different lengths and with different numbers of samples.
    let mut on_gpu = Vec::new();
    let mut on_cpu = Vec::new();
    for (count, samples) in [(4, 2u32), (9, 70), (30, 8), (6, 33), (12, 8)] {
        let (gpu_program, cpu_program) = (program(&mut gpu, count), program(&mut cpu, count));
        let (mut a, mut b) = (gpu.instantiate(gpu_program), cpu.instantiate(cpu_program));
        gpu.set_param(&mut a, "samples", &[samples]).unwrap();
        cpu.set_param(&mut b, "samples", &[samples]).unwrap();
        on_gpu.push(a);
        on_cpu.push(b);
    }

    // Five curves on the CPU, then all of their frames in a single pass.
    assert_eq!(gpu.evaluate(&mut on_gpu), 10);
    assert_eq!(counts(gpu.executor().stats()), (5, 5, 1, 0));

    cpu.evaluate(&mut on_cpu);
    for (a, b) in on_gpu.iter().zip(&on_cpu) {
        let a = gpu.fetch::<EvaluatedFrame>(a, "frames").unwrap();
        let b = cpu.read::<EvaluatedFrame>(b, "frames").unwrap();
        assert_close(bytemuck::cast_slice(a), bytemuck::cast_slice(b));
    }
}

#[test]
fn nothing_is_made_again_once_it_runs() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    let id = program(&mut gpu, 30);
    let mut instance = gpu.instantiate(id);
    frames(&mut gpu, &mut instance);
    let warm = gpu.executor().stats();

    // Every change runs the frames on the GPU again. None of it needs a
    // buffer of its own.
    for samples in [10u32, 12, 16, 9, 30, 4] {
        gpu.set_param(&mut instance, "samples", &[samples]).unwrap();
        gpu.plan([&mut instance]);
        gpu.flush();
        let stats = gpu.executor().stats();
        assert_eq!(stats.buffers, warm.buffers, "{stats:?}");
    }
    // Each run was handed to the device at once.
    let stats = gpu.executor().stats();
    assert_eq!(stats.submits - warm.submits, 6, "{stats:?}");
    assert!(stats.buffers <= 4, "the ring, a page and the readback: {stats:?}");
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
        assert_eq!(counts(gpu.executor().stats()), (2, 1, 2, 0));
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
        assert_eq!(counts(gpu.executor().stats()), (1, 2, 1, 1));
        close(
            gpu.read::<Vec3>(&instance, "positions").unwrap(),
            &reference(None).0,
        );
    }

    #[test]
    fn elements_can_be_taken_of_what_only_the_gpu_has() {
        let Some(mut gpu) = gpu() else {
            return;
        };
        // The position of each frame, one frame at a time.
        let mut b = ProgramBuilder::new("one position");
        let frame = b.param::<EvaluatedFrame>("frame", &[]);
        let position = b.node(&ON_GPU, &[frame]).unwrap()[0];
        b.output("positions", position);
        let one = gpu.add_program(b.finish());

        let mut b = ProgramBuilder::new("each position");
        let points = b.param("points", &points(9));
        let knots = b.constant(&clamped_uniform_knots(9));
        let samples = b.constant::<u32>(&[8]);
        let segments = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let frames = b.node(&FRAMES, &[segments, samples]).unwrap()[0];
        let positions = b
            .for_each(gpu.programs(), one, &[Some(frames)], 0, 1000)
            .unwrap();
        b.output("positions", positions[0]);
        let id = gpu.add_program(b.finish());
        let mut instance = gpu.instantiate(id);
        gpu.evaluate([&mut instance]);

        // The frames never came over: each one was read where it is.
        assert_eq!(gpu.executor().stats().downloads, 0);
        close(
            gpu.fetch::<Vec3>(&instance, "positions").unwrap(),
            &reference(None).0,
        );
        assert_eq!(gpu.executor().stats().downloads, 1);
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

mod for_each {
    use oneiroi_nodes::curve::nurbs::CubicNurbsSegmentCache;

    use super::*;

    /// 16 frames of a single segment.
    fn one_segment<E: Executor>(engine: &mut Engine<E>) -> ProgramId {
        let mut b = ProgramBuilder::new("one segment");
        let segment = b.param::<CubicNurbsSegmentCache>("segment", &[]);
        let samples = b.constant::<u32>(&[16]);
        let frames = b.node(&FRAMES, &[segment, samples]).unwrap()[0];
        b.output("frames", frames);
        engine.add_program(b.finish())
    }

    /// The frames of a curve, made one segment at a time and gathered, and
    /// the same frames made at once.
    fn program<E: Executor>(engine: &mut Engine<E>) -> ProgramId {
        let one = one_segment(engine);
        let mut b = ProgramBuilder::new("each segment");
        let points = b.param("points", &points(9));
        let knots = b.constant(&clamped_uniform_knots(9));
        let samples = b.constant::<u32>(&[16]);
        let segments = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let each = b
            .for_each(engine.programs(), one, &[Some(segments)], 0, 100)
            .unwrap();
        let whole = b.node(&FRAMES, &[segments, samples]).unwrap()[0];
        b.output("each", each[0]);
        b.output("whole", whole);
        engine.add_program(b.finish())
    }

    fn frames<E: Executor>(engine: &mut Engine<E>, instance: &Instance, name: &str) -> Vec<f32> {
        let frames = engine.fetch::<EvaluatedFrame>(instance, name).unwrap();
        bytemuck::cast_slice(frames).to_vec()
    }

    #[test]
    fn calls_for_each_element_run_on_the_gpu_too() {
        let Some(mut gpu) = gpu() else {
            return;
        };
        let mut cpu = Engine::default();
        let (on_gpu, on_cpu) = (program(&mut gpu), program(&mut cpu));
        let (mut a, mut b) = (gpu.instantiate(on_gpu), cpu.instantiate(on_cpu));
        gpu.evaluate([&mut a]);
        cpu.evaluate([&mut b]);

        let each = frames(&mut gpu, &a, "each");
        assert_eq!(each.len(), 6 * 16 * 16);
        assert_close(&each, &frames(&mut cpu, &b, "each"));
        // Segment by segment is the same as all at once.
        assert_close(&each, &frames(&mut gpu, &a, "whole"));

        // The curve on the CPU, then the frames of all six segments in a
        // single pass and gathered on the device. Only what was asked for
        // came back.
        let stats = gpu.executor().stats();
        assert_eq!((stats.cpu, stats.gpu, stats.downloads), (1, 7, 2), "{stats:?}");
        // The frames of the segments, putting them together, and the
        // frames of the whole curve.
        assert_eq!(stats.dispatches, 3, "{stats:?}");
    }
}
