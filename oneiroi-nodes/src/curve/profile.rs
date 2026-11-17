use oneiroi_core::types::{DataType, Trait};

use super::{nurbs::CubicNurbsSegmentCache, sweep::SweptPatch};
use crate::surface::Surface;

/// Everything that can be swept along a curve implements this trait.
pub const PROFILE: Trait = Trait { name: "profile" };

/// A flat outline, the cross-section of a sweep.
///
/// A profile is stored as a slice of `Self` like any other value: a single
/// element for an analytic shape, many for a curve made of segments.
///
/// It has to run counter-clockwise for the swept surface to face outwards.
pub trait Profile: DataType {
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
