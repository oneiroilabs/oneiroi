use glam::Vec2;

use crate::curve::{Curve, RmfSample, ops::resample::ResampleIter};

pub trait IntoCurveProfile {
    fn profile(&self) -> &[Vec2];
}

pub struct CurveSweepIter<
    'a,
    Sub: Iterator<Item = RmfSample>,
    Profile: IntoCurveProfile,
    Taper: Curve<Vec2>,
> {
    // Upstream resampled path stream
    primary: &'a Sub,

    profile: &'a Profile,

    taper: &'a Taper,
    /* // The cross-section shape data (borrowed, no allocation)
    profile: &'a [ProfileVertex],

    // Internal tracking state
    current_step: usize,
    total_expected_steps: usize,

    // The Flat-Map state trackers
    current_frame: Option<TransformFrame>,
    profile_index: usize, */
}
