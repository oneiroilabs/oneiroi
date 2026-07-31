use std::marker::PhantomData;

use crate::curve::Curve;

pub struct ResampleIter<'a, O, C: Curve<O>> {
    curve: &'a C,
    _p: core::marker::PhantomData<O>,

    current_distance: f32,
    step_size: f32,
}

impl<'a, O, C: Curve<O>> ResampleIter<'a, O, C> {
    pub fn new(curve: &'a C, step_size: f32) -> Self {
        Self {
            curve,
            /* lut,
            total_length, */
            current_distance: 0.0,
            step_size,
            _p: PhantomData,
        }
    }
}

impl<'a, O, C: Curve<O>> Iterator for ResampleIter<'a, O, C> {
    type Item = O;

    // Exact count allows the engine to pre-allocate GPU memory perfectly!
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining_dist = self.curve.length() - self.current_distance;
        if remaining_dist <= 0.0 {
            return (0, Some(0));
        }
        let count = (remaining_dist / self.step_size).ceil() as usize + 1;
        (count, Some(count))
    }

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_distance > self.curve.length() + 0.001 {
            return None;
        }

        let t = self.curve.t_at_distance(self.current_distance);

        // 2. Evaluate the position mathematically
        let position = self.curve.sample(t);

        // 3. Step forward for the next iteration
        self.current_distance += self.step_size;

        Some(position)
    }
}
