use glam::Vec2;

use crate::curve::{Curve, RmfSample, ops::resample::ResampleIter};

pub struct CurveSweepIter<
    'a,
    Target: Iterator<Item = RmfSample>,
    Profile: Curve<Vec2>,
    Taper: Curve<f32>,
> {
    // Upstream resampled path stream
    target: &'a Target,

    profile: Option<&'a Profile>,

    taper: Option<&'a Taper>,
    /* // The cross-section shape data (borrowed, no allocation)
    profile: &'a [ProfileVertex],

    // Internal tracking state
    current_step: usize,
    total_expected_steps: usize,

    // The Flat-Map state trackers
    current_frame: Option<TransformFrame>,
    profile_index: usize, */
}
