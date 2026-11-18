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

    /// The surface this profile leaves behind when it is moved along
    /// `spine`, a single curve. Nothing gets sampled here: every patch is still the exact
    /// surface, only cut into pieces which are easy to evaluate.
    fn sweep(spine: &[CubicNurbsSegmentCache], profile: &[Self]) -> Vec<Self::Patch>;

    /// How many patches [`Profile::sweep`] makes of all curves of a spine
    /// with `segments` together, out of a profile of `profile` elements.
    fn patches(segments: usize, profile: usize) -> usize;

    /// The same as [`Profile::sweep`] for every curve at once, on the GPU:
    /// `fn sweep_patch(index: u32) -> Patch` and
    /// `fn sweep_offset(group: u32) -> u32`, where each group of patches
    /// starts. The spine is the input `spine`, with its groups, the
    /// profile is `profile`.
    const SWEEP_WGSL: &'static str;

    /// Inputs to compare both with, see [`GpuKernel::examples`].
    ///
    /// [`GpuKernel::examples`]: oneiroi_core::node::GpuKernel::examples
    const SWEEP_EXAMPLES: &'static [fn() -> Vec<Vec<u32>>];
}

/// The inputs of a sweep as words: curves with as many `points` each
/// as spine, `profile` and the offsets of the spine.
pub(crate) fn sweep_example<P: Profile>(points: &[usize], profile: &[P]) -> Vec<Vec<u32>> {
    let (segments, offsets) = super::nodes::example_curves(points);
    vec![
        bytemuck::cast_slice(&segments).to_vec(),
        bytemuck::cast_slice(profile).to_vec(),
        offsets,
    ]
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

    fn patches(segments: usize, profile: usize) -> usize {
        segments * profile
    }

    const SWEEP_WGSL: &'static str = concat!(include_str!("fractions.wgsl"), "
fn sweep_patch(index: u32) -> SweptPatch {
    let per = in_profile_len();
    let at = index / per;
    let piece = index % per;
    let curve = in_spine_group_of(at);
    let along = curve_fractions(in_spine(at), in_spine(in_spine_end(curve) - 1u));
    let around = curve_fractions(in_profile(piece), in_profile(per - 1u));
    return SweptPatch(in_spine(at), in_profile(piece), along, around);
}

fn sweep_offset(group: u32) -> u32 {
    return spine_offset(group) * in_profile_len();
}
");

    const SWEEP_EXAMPLES: &'static [fn() -> Vec<Vec<u32>>] = &[
        || sweep_example(&[6, 5], &square_profile()),
        || sweep_example(&[7], &square_profile()[..1]),
        || sweep_example::<CubicNurbsSegmentCache>(&[6], &[]),
    ];

    fn sweep(spine: &[CubicNurbsSegmentCache], profile: &[Self]) -> Vec<SweptPatch> {
        let around: Vec<_> = fractions(profile).collect();
        spine
            .iter()
            .zip(fractions(spine))
            .flat_map(|(spine, along)| {
                profile
                    .iter()
                    .zip(&around)
                    .map(move |(profile, around)| SweptPatch::new(*spine, *profile, along, *around))
            })
            .collect()
    }
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
