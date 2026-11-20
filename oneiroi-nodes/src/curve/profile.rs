use oneiroi_core::types::DataType;

use super::{nurbs::CubicNurbsSegmentCache, sweep::SweptPatch};
use crate::surface::Surface;

describe_trait! {
    /// A flat outline which can be swept along a curve, like a circle or a
    /// curve in the plane of `x` and `y`. It has to run counter-clockwise for
    /// the surface it sweeps out to face outwards.
    pub const PROFILE = "profile" { label: "Profile", hue: Orange }
}

/// A flat outline, the cross-section of a sweep.
///
/// A profile is stored as a slice of `Self` like any other value: a single
/// element for an analytic shape, many for a curve made of segments.
///
/// It has to run counter-clockwise for the swept surface to face outwards.
pub trait Profile: DataType + Sync {
    /// What a piece of the surface it sweeps out is.
    type Patch: Surface;

    /// What of `profile` is swept along the curve of group `group`. By
    /// default all of it, the same for every curve.
    fn select(profile: &[Self], _group: usize) -> &[Self] {
        profile
    }

    /// How many patches each segment of a curve sweeps out of a profile of
    /// `profile` elements. Each cap at an end of it has as many.
    fn pieces(profile: usize) -> usize;

    /// Piece `piece` of `profile` swept along `segment`, or of a cap of the
    /// curve: a patch of [`kind`](super::sweep::kind), cut at its ends by
    /// `miters`, which lies `along` of the whole curve. Nothing gets sampled
    /// here: every patch is still the exact surface, only cut into pieces
    /// which are easy to evaluate.
    fn patch(
        segment: CubicNurbsSegmentCache,
        profile: &[Self],
        piece: usize,
        along: [f32; 2],
        kind: u32,
        miters: [[f32; 4]; 2],
    ) -> Self::Patch;

    /// The same for the GPU: `alias Patch`, `fn sweep_pieces() -> u32` and
    /// `fn sweep_piece(segment: CubicNurbsSegmentCache, curve: u32, piece:
    /// u32, along: vec2<f32>, kind: u32, miter_start: vec4<f32>,
    /// miter_end: vec4<f32>) -> Patch`, behind `fractions.wgsl` and
    /// `sweep.wgsl`, which make every patch of them. The profile is the
    /// input `profile`.
    const SWEEP_WGSL: &'static str;

    /// Inputs to compare both with, see [`GpuKernel::examples`].
    ///
    /// [`GpuKernel::examples`]: oneiroi_core::node::GpuKernel::examples
    const SWEEP_EXAMPLES: &'static [fn() -> Vec<Vec<u32>>];
}

/// The inputs of a sweep as words: curves with as many `points` each
/// as spine, `profile`, whether the ends are capped and the offsets of the
/// spine.
pub(crate) fn sweep_example<P: Profile>(points: &[usize], profile: &[P], ends: u32) -> Vec<Vec<u32>> {
    let (segments, offsets) = super::nodes::example_curves(points);
    sweep_example_along(&segments, &offsets, profile, ends)
}

/// [`sweep_example`] along `spine` with the offsets `offsets`.
pub(crate) fn sweep_example_along<P: Profile>(
    spine: &[CubicNurbsSegmentCache],
    offsets: &[u32],
    profile: &[P],
    ends: u32,
) -> Vec<Vec<u32>> {
    vec![
        bytemuck::cast_slice(spine).to_vec(),
        bytemuck::cast_slice(profile).to_vec(),
        vec![ends],
        offsets.to_vec(),
    ]
}

/// Curves with corners and one which is closed, to sweep: a polyline which
/// turns twice, and a rectangle.
pub(crate) fn cornered_spine() -> (Vec<CubicNurbsSegmentCache>, Vec<u32>) {
    use glam::Vec3;
    let turns = super::polyline(&[Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0), Vec3::new(2.0, 1.5, 0.5), Vec3::new(0.0, 2.0, 0.0)], false);
    let closed = super::polyline(&[Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0), Vec3::Y], true);
    let offsets = vec![0, turns.len() as u32, (turns.len() + closed.len()) as u32];
    ([turns, closed].concat(), offsets)
}

/// Where along the whole curve each of its segments starts and ends, as
/// fractions of its length.
pub(crate) fn fractions(segments: &[CubicNurbsSegmentCache]) -> impl Iterator<Item = [f32; 2]> + '_ {
    let total = segments
        .last()
        .map_or(1.0, |last| last.cumulative_length())
        .max(f32::MIN_POSITIVE);
    segments.iter().map(move |segment| {
        let end = segment.cumulative_length();
        [(end - segment.length()) / total, end / total]
    })
}

/// A NURBS curve as profile. Only its x and y are used. Every segment of
/// it sweeps out patches of its own, so a corner between two of them
/// stays a sharp edge.
impl Profile for CubicNurbsSegmentCache {
    type Patch = SweptPatch;

    fn pieces(profile: usize) -> usize {
        profile
    }

    fn patch(
        segment: CubicNurbsSegmentCache,
        profile: &[Self],
        piece: usize,
        along: [f32; 2],
        kind: u32,
        miters: [[f32; 4]; 2],
    ) -> SweptPatch {
        let around = fractions(profile).nth(piece).unwrap_or([0.0, 1.0]);
        SweptPatch::cut(segment, profile[piece], along, around, kind, miters)
    }

    const SWEEP_WGSL: &'static str = concat!(include_str!("curve.wgsl"), include_str!("fractions.wgsl"), include_str!("sweep.wgsl"), "
alias Patch = SweptPatch;

fn sweep_pieces() -> u32 {
    return in_profile_len();
}

fn sweep_piece(
    segment: CubicNurbsSegmentCache,
    curve: u32,
    piece: u32,
    along: vec2<f32>,
    kind: u32,
    miter_start: vec4<f32>,
    miter_end: vec4<f32>,
) -> SweptPatch {
    let around = curve_fractions(in_profile(piece), in_profile(in_profile_len() - 1u));
    return SweptPatch(segment, in_profile(piece), along, around, miter_start, miter_end, kind);
}
");

    const SWEEP_EXAMPLES: &'static [fn() -> Vec<Vec<u32>>] = &[
        || sweep_example(&[6, 5], &square_profile(), 0),
        || sweep_example(&[7], &square_profile()[..1], 1),
        || sweep_example::<CubicNurbsSegmentCache>(&[6], &[], 0),
        || {
            let (spine, offsets) = cornered_spine();
            sweep_example_along(&spine, &offsets, &square_profile(), 0)
        },
    ];
}

/// A square as a curve, with straight edges and sharp corners.
fn square_profile() -> Vec<CubicNurbsSegmentCache> {
    let corners = [(0.2, -0.2), (0.2, 0.2), (-0.2, 0.2), (-0.2, -0.2), (0.2, -0.2)];
    let mut points = vec![glam::Vec4::new(0.2, -0.2, 0., 1.)];
    for pair in corners.windows(2) {
        let (a, b) = (glam::Vec2::from(pair[0]), glam::Vec2::from(pair[1]));
        for step in 1..=3 {
            let point = a.lerp(b, step as f32 / 3.0);
            points.push(glam::Vec4::new(point.x, point.y, 0., 1.));
        }
    }
    let mut knots = vec![0.0; 4];
    for knot in [0.25, 0.5, 0.75] {
        knots.extend([knot; 3]);
    }
    knots.extend([1.0; 4]);
    super::nurbs::extract_segments(&points, &knots)
}
