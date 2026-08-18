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
    fn sample_unchecked(&self, t: f32) -> Vec2 {
        todo!()
    }

    fn sample(&self, t: f32) -> Vec2 {
        todo!()
    }

    fn domain(&self) -> std::ops::Range<f32> {
        todo!()
    }

    fn length(&self) -> f32 {
        todo!()
    }

    fn t_at_distance(&self, distance: f32) -> f32 {
        todo!()
    }
}
