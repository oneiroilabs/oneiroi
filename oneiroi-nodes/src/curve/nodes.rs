//! The curve functionality as nodes of a graph.

use bytemuck::{Pod, Zeroable};

use crate::parallel::per_group;
use glam::{Vec3, Vec4};
use oneiroi_core::{
    node::{
        Caps, Drawable, GpuKernel, GpuOutput, GpuStage, Inputs, NodeDef, NodeRegistry, Outputs, PortDecl,
        Topology,
    },
    types::{DataType, Field, FieldType, Layout, PortDefault, Scalar},
};

use rayon::prelude::*;

use super::nurbs::{CubicNurbsSegmentCache, MovingFrame, NurbsPiece, clamped_uniform_knots, extract_segments, spans};

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
    version: 2,
    inputs: &[
        PortDecl::new::<[f32; 4]>("points").grouped(),
        PortDecl::new::<f32>("knots").grouped(),
    ],
    outputs: &[PortDecl::new::<CubicNurbsSegmentCache>("segments").grouped()],
    cpu: nurbs,
    gpu: &[GpuKernel {
        wgsl: concat!(include_str!("curve.wgsl"), include_str!("nurbs.wgsl")),
        outputs: &[
            GpuOutput {
                entry: "nurbs_segment",
                len: |inputs| segment_counts(inputs).sum(),
                words: None,
            },
            GpuOutput {
                entry: "",
                len: |inputs| inputs.group_count(0) + 1,
                // Known from the knots, so the CPU has them right away.
                words: Some(|inputs| offsets(segment_counts(inputs))),
            },
        ],
        requires: Caps::NONE,
        // The knots, and where the points and the knots of each curve start.
        reads: &[1, 2, 3],
        // Each segment joins the pieces of its curve before it, which is
        // cheap but still grows with the length of the curve.
        accepts: Some(|inputs| {
            (inputs.len(1) == 0 || !inputs.groups(1).is_empty())
                && segment_counts(inputs).all(|count| count <= MAX_GPU_SEGMENTS)
        }),
        examples: &[
            || nurbs_example(&[9, 5], &[]),
            || nurbs_example(&[7], &[&[0., 0., 0., 0., 0.3, 0.3, 0.3, 0.6, 1., 1., 1.]]),
            || {
                let uniform: Vec<f32> = (0..10).map(|k| k as f32 / 9.0).collect();
                nurbs_example(&[6, 6], &[&clamped_uniform_knots(6), &uniform])
            },
            // No curve of 3 points, then knots which do not fit and ones
            // which do.
            || nurbs_example(&[3, 6, 5], &[&clamped_uniform_knots(3), &[0.0; 4], &clamped_uniform_knots(5)]),
            || nurbs_example(&[40], &[]),
            || nurbs_example(&[200, 3, 120], &[]),
            || {
                let line: Vec<[f32; 4]> = (0..6).map(|i| [i as f32, 0.0, 0.0, 1.0]).collect();
                vec![bytemuck::cast_slice(&line).to_vec(), Vec::new(), Vec::new(), Vec::new()]
            },
        ],
        stages: &[
            GpuStage {
                name: "starts",
                ty: u32::INFO,
                entry: "",
                len: |inputs| inputs.group_count(0) + 1,
                words: Some(|inputs| offsets(segment_counts(inputs))),
            },
            GpuStage {
                name: "spans",
                ty: u32::INFO,
                entry: "",
                len: |inputs| segment_counts(inputs).sum(),
                words: Some(span_words),
            },
            GpuStage {
                name: "pieces",
                ty: NurbsPiece::INFO,
                entry: "nurbs_piece",
                len: |inputs| segment_counts(inputs).sum(),
                words: None,
            },
        ],
    }],
};

/// The knot span of every segment of every curve.
fn span_words(inputs: &Inputs<'_>) -> Vec<u32> {
    let mut words = Vec::new();
    for curve in curve_spans(inputs) {
        match curve {
            CurveSpans::Uniform(points) => words.extend(3..points as u32),
            CurveSpans::Knots(points, knots) => {
                words.extend(spans(points, knots).map(|span| span as u32));
            }
            CurveSpans::None => {}
        }
    }
    words
}

/// Where each group starts, of groups this long, and where the last ends.
fn offsets(lengths: impl Iterator<Item = usize>) -> Vec<u32> {
    let mut offsets = vec![0];
    for len in lengths {
        offsets.push(offsets.last().unwrap() + len as u32);
    }
    offsets
}

/// No longer curves than these on the GPU.
const MAX_GPU_SEGMENTS: usize = 4096;

/// The inputs of [`NURBS`] as words: curves with as many `points` each,
/// with a group of `knots` each or none at all.
fn nurbs_example(points: &[usize], knots: &[&[f32]]) -> Vec<Vec<u32>> {
    let (points, offsets) = example_points(points);
    let mut knot_offsets = vec![0];
    let mut all = Vec::new();
    for group in knots {
        all.extend_from_slice(group);
        knot_offsets.push(all.len() as u32);
    }
    if knots.is_empty() {
        knot_offsets.clear();
    }
    vec![
        bytemuck::cast_slice(&points).to_vec(),
        bytemuck::cast_slice(&all).to_vec(),
        offsets,
        knot_offsets,
    ]
}

/// The knots of the curve of group `group` with `points` points.
fn curve_knots<'a>(
    points: usize,
    knots: &'a [f32],
    knot_groups: &[std::ops::Range<usize>],
    group: usize,
) -> std::borrow::Cow<'a, [f32]> {
    match knots.is_empty() {
        true => clamped_uniform_knots(points).into(),
        false => knots[knot_groups[group.min(knot_groups.len() - 1)].clone()].into(),
    }
}

/// The knot spans which become segments, of one curve.
enum CurveSpans<'a> {
    /// No knots of its own: clamped and uniform, so every span between
    /// the ends is a segment, of this many points.
    Uniform(usize),
    /// Knots of its own, for this many points.
    Knots(usize, &'a [f32]),
    /// No curve.
    None,
}

impl CurveSpans<'_> {
    fn count(&self) -> usize {
        match self {
            CurveSpans::Uniform(points) => points - 3,
            CurveSpans::Knots(points, knots) => spans(*points, knots).count(),
            CurveSpans::None => 0,
        }
    }
}

/// The spans of every curve, without making a knot vector for curves
/// which have none: that is asked for every call, and often.
fn curve_spans<'a>(inputs: &'a Inputs<'_>) -> impl Iterator<Item = CurveSpans<'a>> + 'a {
    let knots = inputs.get::<f32>(1);
    let knot_groups = inputs.groups(1);
    inputs.groups(0).into_iter().enumerate().map(move |(group, curve)| {
        let points = curve.len();
        if points < 4 {
            return CurveSpans::None;
        }
        if knots.is_empty() {
            return CurveSpans::Uniform(points);
        }
        let knots = &knots[knot_groups[group.min(knot_groups.len() - 1)].clone()];
        match knots.len() == points + 4 {
            true => CurveSpans::Knots(points, knots),
            false => CurveSpans::None,
        }
    })
}

/// How many segments each curve gets, without making them.
fn segment_counts<'a>(inputs: &'a Inputs<'_>) -> impl Iterator<Item = usize> + 'a {
    curve_spans(inputs).map(|curve| curve.count())
}

fn nurbs(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let points = inputs.get::<[f32; 4]>(0);
    let knots = inputs.get::<f32>(1);
    let knot_groups = inputs.groups(1);

    let (segments, offsets) = per_group(inputs.groups(0), |group, curve| {
        let points: Vec<_> = points[curve].iter().map(|p| Vec4::from_array(*p)).collect();
        let knots = curve_knots(points.len(), knots, &knot_groups, group);
        extract_segments(&points, &knots)
    });
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
                len: |inputs| inputs.group_count(0) * per_curve(inputs),
                words: None,
            },
            GpuOutput {
                entry: "sample_curve_offsets",
                len: |inputs| inputs.group_count(0) + 1,
                words: Some(|inputs| {
                    offsets((0..inputs.group_count(0)).map(|_| per_curve(inputs)))
                }),
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
        stages: &[],
    }],
};

/// Control points of curves with as many `points` each, which bend in
/// every direction, and the offsets where each curve starts.
pub(crate) fn example_points(points: &[usize]) -> (Vec<Vec4>, Vec<u32>) {
    let mut all = Vec::new();
    let mut offsets = vec![0];
    for (curve, points) in points.iter().enumerate() {
        all.extend((0..*points).map(|i| {
            let t = i as f32 + curve as f32 * 0.3;
            Vec4::new((t * 0.9).sin() * 2.0, t * 0.7, (t * 1.3).cos(), 1.0 + t * 0.1)
        }));
        offsets.push(all.len() as u32);
    }
    (all, offsets)
}

/// The segments of [`example_points`], and the offsets where the
/// segments of each curve start.
pub(crate) fn example_curves(points: &[usize]) -> (Vec<CubicNurbsSegmentCache>, Vec<u32>) {
    let (all, point_offsets) = example_points(points);
    let mut segments = Vec::new();
    let mut offsets = vec![0];
    for curve in point_offsets.windows(2) {
        let points = &all[curve[0] as usize..curve[1] as usize];
        segments.extend(extract_segments(points, &clamped_uniform_knots(points.len())));
        offsets.push(segments.len() as u32);
    }
    (segments, offsets)
}

/// The inputs of [`SAMPLE_CURVE`] as words, along curves of as many
/// `points` each.
fn sample_example(points: &[usize], count: u32, spacing: f32, t: &[f32]) -> Vec<Vec<u32>> {
    let (segments, offsets) = example_curves(points);
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

    let (frames, offsets) = per_group(inputs.groups(0), |_, curve| {
        let curve = &segments[curve];
        let (Some(first), Some(last)) = (curve.first(), curve.last()) else {
            return Vec::new();
        };
        let start = first.cumulative_length() - first.length();
        let length = last.cumulative_length() - start;
        let at = |distance: f32| -> EvaluatedFrame { frame_at(curve, distance).into() };
        if !ts.is_empty() {
            ts.iter().map(|t| at(t.clamp(0.0, 1.0) * length + start)).collect()
        } else if spacing > 0.0 {
            let count = ((length / spacing) as usize + 1).min(MAX_SPACED);
            (0..count).map(|i| at(i as f32 * spacing + start)).collect()
        } else {
            let fraction = |i: usize| match per {
                0 | 1 => 0.0,
                _ => i as f32 / (per - 1) as f32,
            };
            (0..per).map(|i| at(fraction(i) * length + start)).collect()
        }
    });
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
    gpu: &[GpuKernel {
        wgsl: "
fn translate(index: u32) -> CubicNurbsSegmentCache {
    var segment = in_segments(index);
    if (in_offset_len() == 0u) {
        return segment;
    }
    // The columns are homogeneous, so the offset scales with the weight.
    let offset = in_offset(0u);
    let m = segment.monomial_basis;
    segment.monomial_basis = mat4x4<f32>(
        m[0] + vec4<f32>(offset * m[0].w, 0.0),
        m[1] + vec4<f32>(offset * m[1].w, 0.0),
        m[2] + vec4<f32>(offset * m[2].w, 0.0),
        m[3] + vec4<f32>(offset * m[3].w, 0.0),
    );
    return segment;
}",
        outputs: &[GpuOutput {
            entry: "translate",
            len: |inputs| inputs.len(0),
            words: None,
        }],
        requires: Caps::NONE,
        reads: &[],
        accepts: None,
        examples: &[
            || translate_example(&[9, 5], [1.0, -2.0, 0.5]),
            || translate_example(&[6], [0.0; 3]),
            || translate_example(&[], [3.0, 0.0, 0.0]),
        ],
        stages: &[],
    }],
};

fn translate_example(points: &[usize], offset: [f32; 3]) -> Vec<Vec<u32>> {
    let (segments, offsets) = example_curves(points);
    vec![
        bytemuck::cast_slice(&segments).to_vec(),
        bytemuck::cast_slice(&offset).to_vec(),
        offsets,
    ]
}

fn translate(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let offset = inputs.value::<Vec3>(1);
    let moved: Vec<_> = inputs
        .get::<CubicNurbsSegmentCache>(0)
        .par_iter()
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
        assert_eq!(checked, 6);

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
        // 5 nodes of curves, a mesh for each of the 2 surfaces
        // and a sweep for each of the 2 profiles, and 2 values.
        assert_eq!(registry.iter().count(), 10);
        assert!(registry.get("curve.nurbs").is_some());
    }
}
