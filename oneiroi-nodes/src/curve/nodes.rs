//! The curve functionality as nodes of a graph.

use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4};
use oneiroi_core::{
    node::{
        Caps, Drawable, GpuKernel, GpuOutput, Inputs, NodeDef, NodeRegistry, Outputs, PortDecl,
        Topology,
    },
    types::{DataType, Field, FieldType, Layout, PortDefault, Scalar},
};

use super::nurbs::{CubicNurbsSegmentCache, MovingFrame, clamped_uniform_knots, extract_segments};

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
///
/// Every group of points is a curve of its own, so are its segments. The
/// knots of a curve are the group of knots with the same index, or the
/// last group if there are fewer. Without any knots a curve starts at its
/// first point, ends at its last, and its segments are spread evenly.
pub static NURBS: NodeDef = NodeDef {
    id: "curve.nurbs",
    display_name: "NURBS Curve",
    version: 1,
    inputs: &[
        PortDecl::new::<[f32; 4]>("points").grouped(),
        PortDecl::new::<f32>("knots").grouped(),
    ],
    outputs: &[PortDecl::new::<CubicNurbsSegmentCache>("segments").grouped()],
    cpu: nurbs,
    gpu: &[],
};

fn nurbs(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let points = inputs.get::<[f32; 4]>(0);
    let knots = inputs.get::<f32>(1);
    let knot_groups = inputs.groups(1);

    let mut segments = Vec::new();
    let mut offsets = vec![0];
    for (group, curve) in inputs.groups(0).into_iter().enumerate() {
        let points: Vec<_> = points[curve].iter().map(|p| Vec4::from_array(*p)).collect();
        let knots = match knots.is_empty() {
            true => clamped_uniform_knots(points.len()),
            false => knots[knot_groups[group.min(knot_groups.len() - 1)].clone()].to_vec(),
        };
        segments.extend(extract_segments(&points, &knots));
        offsets.push(segments.len() as u32);
    }
    outputs.set(0, &segments);
    outputs.set_groups(0, &offsets);
}

/// Frames along every curve, a group of them per curve. Where they are
/// is told by the first of these which is given:
///
/// - `t`: at these fractions of the length of the curve,
/// - `spacing`: every `spacing` along the curve, starting at its start,
/// - `count`: as many as `count`, from the start to the end, with the same
///   distance between them.
///
/// Every place is measured along the curve, so the frames do not bunch up
/// where its control points do.
pub static SAMPLE_CURVE: NodeDef = NodeDef {
    id: "curve.sample",
    display_name: "Sample Curve",
    version: 2,
    inputs: &[
        PortDecl::new::<CubicNurbsSegmentCache>("segments").grouped(),
        PortDecl::new::<u32>("count").with_default(PortDefault::U32(&[32])).one(),
        PortDecl::new::<f32>("spacing").with_default(PortDefault::F32(&[0.0])).one(),
        PortDecl::new::<f32>("t"),
    ],
    outputs: &[PortDecl::new::<EvaluatedFrame>("frames").grouped()],
    cpu: sample_curve,
    gpu: &[GpuKernel {
        wgsl: concat!(include_str!("curve.wgsl"), include_str!("sample.wgsl")),
        outputs: &[
            GpuOutput {
                entry: "sample_curve_frames",
                len: |inputs| inputs.groups(0).len() * per_curve(inputs),
            },
            GpuOutput {
                entry: "sample_curve_offsets",
                len: |inputs| inputs.groups(0).len() + 1,
            },
        ],
        requires: Caps::NONE,
        // The count, the spacing and the offsets of the curves.
        reads: &[1, 2, 4],
        // How many frames come of a spacing depends on how long the curves
        // are, which only the CPU knows before they are made.
        accepts: Some(|inputs| {
            inputs.value::<f32>(2) <= 0.0 && inputs.groups(0).iter().all(|curve| !curve.is_empty())
        }),
        examples: &[
            || sample_example(&[9], 8, 0.0, &[]),
            || sample_example(&[9, 5], 5, 0.0, &[]),
            || sample_example(&[6, 4], 3, 0.0, &[0.0, 0.25, 1.0, 1.5, -1.0]),
            || sample_example(&[4], 1, 0.0, &[]),
            || sample_example(&[5], 0, 0.0, &[]),
            || sample_example(&[7], 4, 0.5, &[]),
            || sample_example(&[0], 4, 0.0, &[]),
        ],
    }],
};

/// Curves of as many `points` each, which bend in every direction, and the
/// other inputs of [`SAMPLE_CURVE`] as words.
fn sample_example(points: &[usize], count: u32, spacing: f32, t: &[f32]) -> Vec<Vec<u32>> {
    let mut segments = Vec::new();
    let mut offsets = vec![0];
    for (curve, points) in points.iter().enumerate() {
        let points: Vec<_> = (0..*points)
            .map(|i| {
                let t = i as f32 + curve as f32 * 0.3;
                Vec4::new((t * 0.9).sin() * 2.0, t * 0.7, (t * 1.3).cos(), 1.0 + t * 0.1)
            })
            .collect();
        if points.len() >= 4 {
            segments.extend(extract_segments(&points, &clamped_uniform_knots(points.len())));
        }
        offsets.push(segments.len() as u32);
    }
    vec![
        bytemuck::cast_slice(&segments).to_vec(),
        vec![count],
        vec![spacing.to_bits()],
        bytemuck::cast_slice(t).to_vec(),
        offsets,
    ]
}

/// How many frames every curve gets unless they are spaced.
fn per_curve(inputs: &Inputs<'_>) -> usize {
    match inputs.len(3) {
        0 => inputs.value::<u32>(1) as usize,
        given => given,
    }
}

/// No more frames than these on a single curve, however small the spacing.
const MAX_SPACED: usize = 1 << 16;

fn sample_curve(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let segments = inputs.get::<CubicNurbsSegmentCache>(0);
    let spacing = inputs.value::<f32>(2);
    let ts = inputs.get::<f32>(3);
    let per = per_curve(inputs);

    let mut frames = Vec::new();
    let mut offsets = vec![0];
    for curve in inputs.groups(0) {
        let curve = &segments[curve];
        if let (Some(first), Some(last)) = (curve.first(), curve.last()) {
            let start = first.cumulative_length() - first.length();
            let length = last.cumulative_length() - start;
            let at = |distance: f32| -> EvaluatedFrame { frame_at(curve, distance).into() };
            if !ts.is_empty() {
                frames.extend(ts.iter().map(|t| at(t.clamp(0.0, 1.0) * length + start)));
            } else if spacing > 0.0 {
                let count = ((length / spacing) as usize + 1).min(MAX_SPACED);
                frames.extend((0..count).map(|i| at(i as f32 * spacing + start)));
            } else {
                let fraction = |i: usize| match per {
                    0 | 1 => 0.0,
                    _ => i as f32 / (per - 1) as f32,
                };
                frames.extend((0..per).map(|i| at(fraction(i) * length + start)));
            }
        }
        offsets.push(frames.len() as u32);
    }
    outputs.set(0, &frames);
    outputs.set_groups(0, &offsets);
}

/// The frame `distance` along the segments of a curve, measured as their
/// lengths are: from the start of the whole curve.
fn frame_at(curve: &[CubicNurbsSegmentCache], distance: f32) -> MovingFrame {
    let index = curve
        .partition_point(|segment| distance >= segment.cumulative_length())
        .min(curve.len() - 1);
    let segment = &curve[index];
    let local = distance - (segment.cumulative_length() - segment.length());
    segment.frame(segment.u_at_distance(local))
}

/// The curve moved by an offset.
pub static TRANSLATE: NodeDef = NodeDef {
    id: "curve.translate",
    display_name: "Translate Curve",
    version: 1,
    inputs: &[
        PortDecl::new::<CubicNurbsSegmentCache>("segments").grouped(),
        PortDecl::new::<Vec3>("offset").with_default(PortDefault::F32(&[0.0; 3])).one(),
    ],
    outputs: &[PortDecl::new::<CubicNurbsSegmentCache>("segments").grouped_like(0)],
    cpu: translate,
    gpu: &[],
};

fn translate(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let offset = inputs.value::<Vec3>(1);
    let moved: Vec<_> = inputs
        .get::<CubicNurbsSegmentCache>(0)
        .iter()
        .map(|segment| segment.translated(offset))
        .collect();
    outputs.set(0, &moved);
}

/// A sphere around the segments in `words`, found by looking at a few
/// points of each of them.
fn curve_bounds(words: &[u32]) -> Option<([f32; 3], f32)> {
    const STEPS: usize = 8;

    let segments: &[CubicNurbsSegmentCache] = bytemuck::try_cast_slice(words).ok()?;
    let points = segments.iter().flat_map(|segment| {
        (0..=STEPS).map(move |step| segment.evaluate(step as f32 / STEPS as f32).0)
    });
    let (min, max) = points.fold((Vec3::MAX, Vec3::MIN), |(min, max), point| {
        (min.min(point), max.max(point))
    });
    (min.x <= max.x).then(|| (((min + max) * 0.5).to_array(), (max - min).length() * 0.5))
}

pub fn register(registry: &mut NodeRegistry) {
    // A curve can be looked at as it is: a line through its segments.
    registry.register_drawable(Drawable {
        ty: CubicNurbsSegmentCache::INFO,
        topology: Topology::Line,
        wgsl: concat!(include_str!("curve.wgsl"), include_str!("curve_line.wgsl")),
        bounds: curve_bounds,
    });
    registry.register_drawable(crate::primitive::CIRCLE_LINE);

    registry.register(&NURBS);
    registry.register(&SAMPLE_CURVE);
    registry.register(&TRANSLATE);
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

    /// The control points are exposed, the knots are fixed. Frames by
    /// count and at a few places.
    fn program() -> oneiroi_core::program::Program {
        let mut b = ProgramBuilder::new("tube spine");
        let points = b.param("points", &points());
        let count = b.param_one_of::<u32>("count", 8);
        let knots = b.constant(&clamped_uniform_knots(6));
        let no_spacing = b.constant::<f32>(&[0.0]);
        let no_ts = b.constant::<f32>(&[]);
        let ts = b.constant::<f32>(&[0.0, 0.5, 1.0]);

        let segments = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let frames = b.node(&SAMPLE_CURVE, &[segments, count, no_spacing, no_ts]).unwrap()[0];
        let at = b.node(&SAMPLE_CURVE, &[segments, count, no_spacing, ts]).unwrap()[0];
        b.output("frames", frames);
        b.output("at", at);
        b.finish()
    }

    fn curve() -> CubicNurbs {
        CubicNurbs::new(
            points().into_iter().map(Vec4::from_array).collect(),
            clamped_uniform_knots(6),
        )
    }

    fn close(a: MovingFrame, b: MovingFrame) -> bool {
        a.position.distance(b.position) < 1e-4 && a.tangent.distance(b.tangent) < 1e-4
    }

    #[test]
    fn the_graph_matches_the_direct_api() {
        let mut engine = Engine::default();
        let id = engine.add_program(program());
        let mut instance = engine.instantiate(id);
        assert_eq!(engine.evaluate([&mut instance]), 3);
        let curve = curve();

        // Places are measured along the curve, the same as the curve does.
        let at = engine.read::<EvaluatedFrame>(&instance, "at").unwrap();
        let length = curve.length();
        for (frame, t) in at.iter().zip([0.0, 0.5, 1.0]) {
            let expected = curve.frame(curve.t_at_distance(t * length));
            assert!(close((*frame).into(), expected), "{t}");
        }

        let frames = engine.read::<EvaluatedFrame>(&instance, "frames").unwrap();
        let expected = curve.sample_equidistant(8);
        assert_eq!(frames.len(), 8);
        for (frame, expected) in frames.iter().zip(expected) {
            assert!(close((*frame).into(), expected));
        }
    }

    #[test]
    fn frames_can_be_spaced() {
        let mut b = ProgramBuilder::new("spaced");
        let points = b.param("points", &points());
        let knots = b.constant(&clamped_uniform_knots(6));
        let count = b.constant::<u32>(&[8]);
        let spacing = b.param_one_of::<f32>("spacing", 0.5);
        let no_ts = b.constant::<f32>(&[]);
        let segments = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let frames = b.node(&SAMPLE_CURVE, &[segments, count, spacing, no_ts]).unwrap()[0];
        b.output("frames", frames);

        let mut engine = Engine::default();
        let id = engine.add_program(b.finish());
        let mut instance = engine.instantiate(id);
        engine.evaluate([&mut instance]);

        let length = curve().length();
        let frames = engine.read::<EvaluatedFrame>(&instance, "frames").unwrap();
        assert_eq!(frames.len(), (length / 0.5) as usize + 1);
        assert_eq!(engine.groups(&instance, "frames"), Some(&[0, frames.len() as u32][..]));
        // Neighbours are about as far apart as asked, a bit less where the
        // curve bends between them.
        for pair in frames.windows(2) {
            let (a, b) = (MovingFrame::from(pair[0]), MovingFrame::from(pair[1]));
            let distance = a.position.distance(b.position);
            assert!(distance <= 0.5 + 1e-3 && distance > 0.4, "{distance}");
        }
    }

    #[test]
    fn the_curve_is_only_rebuilt_when_its_points_change() {
        let mut engine = Engine::default();
        let id = engine.add_program(program());
        let mut a = engine.instantiate(id);
        let mut b = engine.instantiate(id);
        engine.set_param(&mut b, "count", &[16u32]).unwrap();

        // Both share the segments, b samples them anew.
        assert_eq!(engine.evaluate([&mut a, &mut b]), 3 + 2);

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
        assert_eq!(engine.read::<EvaluatedFrame>(&instance, "at"), Some(&[][..]));
        assert_eq!(engine.groups(&instance, "frames"), Some(&[0, 0][..]));
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

        // And so does everything which says it can be drawn as it is: the
        // two kinds of surface, the curve and the circle.
        for ty in registry.implementors(crate::surface::SURFACE) {
            assert!(registry.drawable(&ty).is_some(), "every surface can be drawn");
        }
        use oneiroi_core::{
            node::Topology,
            shader::{LodMethod, draw, draw_lod},
        };
        let (mut drawn, mut detailed) = (0, 0);
        for drawable in registry.drawables() {
            draw(drawable).unwrap_or_else(|e| panic!("{e}"));
            drawn += 1;
            // A surface can also be drawn as fine as it appears, both ways.
            for method in [LodMethod::VertexPulling, LodMethod::MeshShader] {
                match (drawable.topology, draw_lod(drawable, method)) {
                    (Topology::Surface, Ok(_)) => detailed += 1,
                    (Topology::Surface, Err(error)) => panic!("{error}"),
                    (Topology::Line, result) => assert!(result.is_err()),
                }
            }
        }
        assert_eq!((drawn, detailed), (4, 4));
    }

    #[test]
    fn every_node_is_registered_once() {
        let mut registry = NodeRegistry::new();
        crate::register(&mut registry);
        crate::register(&mut registry);
        // 5 nodes of curves, a tessellation for each of the 2 surfaces
        // and a sweep for each of the 2 profiles, and 2 values.
        assert_eq!(registry.iter().count(), 10);
        assert!(registry.get("curve.nurbs").is_some());
    }
}
