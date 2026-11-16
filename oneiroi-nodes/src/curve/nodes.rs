//! The curve functionality as nodes of a graph.

use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4};
use oneiroi_core::{
    node::{Caps, GpuKernel, Inputs, NodeDef, NodeRegistry, Outputs, PortDecl, Shape},
    types::{DataType, Field, FieldType, Layout, PortDefault, Scalar},
};

use super::nurbs::{CubicNurbsSegmentCache, MovingFrame, extract_segments, locate};

/// A [`MovingFrame`] in the layout of the WGSL struct with the same name.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, PartialEq)]
pub struct EvaluatedFrame {
    position: [f32; 3],
    _padding0: f32,
    tangent: [f32; 3],
    _padding1: f32,
    normal: [f32; 3],
    _padding2: f32,
    binormal: [f32; 3],
    _padding3: f32,
}

impl DataType for EvaluatedFrame {
    type ConfigurationOptions = ();

    const NAME: &'static str = "evaluated_frame";
    const WGSL: &'static str = "EvaluatedFrame";
    const SCALAR: Scalar = Scalar::F32;
    const LAYOUT: Layout = Layout::Struct(&[
        Field::new("position", FieldType::Vec3),
        Field::pad(1),
        Field::new("tangent", FieldType::Vec3),
        Field::pad(1),
        Field::new("normal", FieldType::Vec3),
        Field::pad(1),
        Field::new("binormal", FieldType::Vec3),
        Field::pad(1),
    ]);
}

impl From<MovingFrame> for EvaluatedFrame {
    fn from(frame: MovingFrame) -> Self {
        Self {
            position: frame.position.to_array(),
            tangent: frame.tangent.to_array(),
            normal: frame.normal.to_array(),
            binormal: frame.binormal.to_array(),
            ..Self::zeroed()
        }
    }
}

impl From<EvaluatedFrame> for MovingFrame {
    fn from(frame: EvaluatedFrame) -> Self {
        Self {
            position: frame.position.into(),
            tangent: frame.tangent.into(),
            normal: frame.normal.into(),
            binormal: frame.binormal.into(),
        }
    }
}

/// Control points (weight in `w`) and knots to the segments of the curve.
pub static NURBS: NodeDef = NodeDef {
    id: "curve.nurbs",
    display_name: "NURBS Curve",
    inputs: &[
        PortDecl::new::<[f32; 4]>("points"),
        PortDecl::new::<f32>("knots"),
    ],
    outputs: &[PortDecl::new::<CubicNurbsSegmentCache>("segments")],
    // The frame of a segment continues the one of the segment before.
    shape: Shape::Scan,
    cpu: nurbs,
    gpu: &[],
};

fn nurbs(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let points: Vec<_> = inputs
        .get::<[f32; 4]>(0)
        .iter()
        .map(|point| Vec4::from_array(*point))
        .collect();
    outputs.set(0, &extract_segments(&points, inputs.get(1)));
}

/// `samples` frames per segment, the first and last one at its ends.
pub static FRAMES: NodeDef = NodeDef {
    id: "curve.frames",
    display_name: "Curve Frames",
    inputs: &[
        PortDecl::new::<CubicNurbsSegmentCache>("segments"),
        PortDecl::new::<u32>("samples").with_default(PortDefault::U32(&[32])),
    ],
    outputs: &[PortDecl::new::<EvaluatedFrame>("frames")],
    shape: Shape::Expand,
    cpu: frames,
    gpu: &[GpuKernel {
        wgsl: include_str!("frames.wgsl"),
        entry: "curve_frames",
        requires: Caps::NONE,
        len: frames_len,
    }],
};

fn frames_samples(inputs: &Inputs<'_>) -> usize {
    inputs.value::<u32>(1).max(2) as usize
}

fn frames_len(inputs: &Inputs<'_>) -> usize {
    inputs.get::<CubicNurbsSegmentCache>(0).len() * frames_samples(inputs)
}

fn frames(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let segments = inputs.get::<CubicNurbsSegmentCache>(0);
    let samples = frames_samples(inputs);

    let frames = outputs.alloc::<EvaluatedFrame>(0, frames_len(inputs));
    for (segment, frames) in segments.iter().zip(frames.chunks_exact_mut(samples)) {
        for (sample, frame) in frames.iter_mut().enumerate() {
            *frame = segment.frame(sample as f32 / (samples - 1) as f32).into();
        }
    }
}

/// The position on the curve for every `t` in `0..=1`.
pub static SAMPLE: NodeDef = NodeDef {
    id: "curve.sample",
    display_name: "Sample Curve",
    inputs: &[
        PortDecl::new::<CubicNurbsSegmentCache>("segments"),
        PortDecl::new::<f32>("t"),
    ],
    outputs: &[PortDecl::new::<Vec3>("positions")],
    shape: Shape::Map,
    cpu: sample,
    gpu: &[],
};

fn sample(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let segments = inputs.get::<CubicNurbsSegmentCache>(0);
    let ts = inputs.get::<f32>(1);
    if segments.is_empty() {
        return;
    }

    let positions = outputs.alloc::<Vec3>(0, ts.len());
    for (position, t) in positions.iter_mut().zip(ts) {
        let (index, u) = locate(segments.len(), *t);
        *position = segments[index].evaluate(u).0;
    }
}

pub fn register(registry: &mut NodeRegistry) {
    registry.register(&NURBS);
    registry.register(&FRAMES);
    registry.register(&SAMPLE);
    registry.register(&crate::primitive::CIRCLE);
    super::register_profile::<crate::primitive::Circle>(registry);
    super::register_profile::<CubicNurbsSegmentCache>(registry);
}

#[cfg(test)]
mod tests {
    use oneiroi_core::{exec::Engine, program::ProgramBuilder};

    use super::*;
    use crate::curve::nurbs::{CubicNurbs, clamped_uniform_knots};

    fn points() -> Vec<[f32; 4]> {
        vec![
            [0.0, 0.0, 0.0, 1.],
            [1.0, 2.0, 0.5, 1.],
            [2.0, -1.0, 1.0, 1.],
            [3.0, 3.0, 0.0, 1.],
            [4.0, 0.0, -1.0, 1.],
            [5.0, 2.0, 0.0, 1.],
        ]
    }

    /// The control points are exposed, the knots are fixed.
    fn program() -> oneiroi_core::program::Program {
        let mut b = ProgramBuilder::new("tube spine");
        let points = b.param("points", &points());
        let samples = b.param::<u32>("samples", &[8]);
        let knots = b.constant(&clamped_uniform_knots(6));
        let ts = b.constant::<f32>(&[0.0, 0.5, 1.0]);

        let segments = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let frames = b.node(&FRAMES, &[segments, samples]).unwrap()[0];
        let positions = b.node(&SAMPLE, &[segments, ts]).unwrap()[0];
        b.output("frames", frames);
        b.output("positions", positions);
        b.finish()
    }

    #[test]
    fn the_graph_matches_the_direct_api() {
        let mut engine = Engine::default();
        let id = engine.add_program(program());
        let mut instance = engine.instantiate(id);
        assert_eq!(engine.evaluate([&mut instance]), 3);

        let curve = CubicNurbs::new(
            points().into_iter().map(Vec4::from_array).collect(),
            clamped_uniform_knots(6),
        );

        let positions = engine.read::<Vec3>(&instance, "positions").unwrap();
        assert_eq!(
            positions,
            [curve.evaluate(0.0), curve.evaluate(0.5), curve.evaluate(1.0)]
        );

        let frames = engine.read::<EvaluatedFrame>(&instance, "frames").unwrap();
        assert_eq!(frames.len(), curve.segments().len() * 8);
        assert_eq!(frames[8], curve.segments()[1].frame(0.0).into());
    }

    #[test]
    fn the_curve_is_only_rebuilt_when_its_points_change() {
        let mut engine = Engine::default();
        let id = engine.add_program(program());
        let mut a = engine.instantiate(id);
        let mut b = engine.instantiate(id);
        engine.set_param(&mut b, "samples", &[16u32]).unwrap();

        // Both share the segments and the sampled positions.
        assert_eq!(engine.evaluate([&mut a, &mut b]), 3 + 1);

        let mut moved = points();
        moved[2][1] = 4.0;
        engine.set_param(&mut a, "points", &moved).unwrap();
        assert_eq!(engine.evaluate([&mut a, &mut b]), 3);
    }

    #[test]
    fn invalid_curves_are_empty_instead_of_a_panic() {
        let mut engine = Engine::default();
        let id = engine.add_program(program());
        let mut instance = engine.instantiate(id);
        engine
            .set_param(&mut instance, "points", &points()[..3])
            .unwrap();
        engine.evaluate([&mut instance]);

        assert_eq!(
            engine.read::<EvaluatedFrame>(&instance, "frames"),
            Some(&[][..])
        );
        assert_eq!(engine.read::<Vec3>(&instance, "positions"), Some(&[][..]));
    }

    #[test]
    fn gpu_kernels_compile() {
        let mut registry = NodeRegistry::new();
        crate::register(&mut registry);
        let mut checked = 0;
        for node in registry.iter() {
            for kernel in node.gpu {
                oneiroi_core::shader::compute(node, kernel).unwrap_or_else(|e| panic!("{e}"));
                checked += 1;
            }
        }
        assert_eq!(checked, 1);
    }

    #[test]
    fn every_node_is_registered_once() {
        let mut registry = NodeRegistry::new();
        crate::register(&mut registry);
        crate::register(&mut registry);
        assert_eq!(registry.iter().count(), 6);
        assert!(registry.get("curve.nurbs").is_some());
    }
}
