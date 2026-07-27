use std::range::Range;

use glam::Vec2;

use crate::curve::Curve;

#[derive(Debug, Clone, Copy)]
pub struct Circle {
    radius: f32,
}

impl Circle {
    pub fn new(radius: f32) -> Self {
        Self { radius }
    }
}

impl Curve<Vec2> for Circle {
    fn domain(&self) -> core::range::Range<f32> {
        Range::from(f32::MIN..f32::MAX)
    }

    fn sample_unchecked(&self, t: f32) -> Vec2 {
        todo!()
    }

    fn sample(&self, t: f32) -> Vec2 {
        todo!()
    }
}
