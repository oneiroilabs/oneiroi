use glam::{Vec2, Vec3Swizzles};
use oneiroi_core::types::{DataType, Trait};

use super::nurbs::{CubicNurbsSegmentCache, locate};

/// Everything that can be swept along a curve implements this trait.
pub const PROFILE: Trait = Trait { name: "profile" };

/// A flat outline, the cross-section of a sweep.
///
/// A profile is stored as a slice of `Self` like any other value: a single
/// element for an analytic shape, many for a curve made of segments.
pub trait Profile: DataType {
    /// Position and normal at `v` in `0..=1` along the outline.
    ///
    /// The normal points to the right of the direction the outline runs
    /// in, which is outwards for one that runs counter-clockwise.
    /// `profile` is never empty.
    fn point(profile: &[Self], v: f32) -> (Vec2, Vec2);

    /// Every `v` strictly between 0 and 1 at which the outline has a
    /// corner, in ascending order. The normal has two values there.
    fn breaks(_profile: &[Self]) -> Vec<f32> {
        Vec::new()
    }
}

/// A NURBS curve as profile. Only its x and y are used.
impl Profile for CubicNurbsSegmentCache {
    fn point(profile: &[Self], v: f32) -> (Vec2, Vec2) {
        let (index, u) = locate(profile.len(), v);
        let (position, _, tangent) = profile[index].evaluate(u);
        let tangent = tangent.xy().normalize_or_zero();
        (position.xy(), Vec2::new(tangent.y, -tangent.x))
    }

    fn breaks(profile: &[Self]) -> Vec<f32> {
        profile
            .windows(2)
            .enumerate()
            .filter(|(_, pair)| {
                let (end, start) = (pair[0].evaluate(1.0).2, pair[1].evaluate(0.0).2);
                end.xy().normalize_or_zero().dot(start.xy().normalize_or_zero()) < 0.999
            })
            .map(|(index, _)| (index + 1) as f32 / profile.len() as f32)
            .collect()
    }
}
