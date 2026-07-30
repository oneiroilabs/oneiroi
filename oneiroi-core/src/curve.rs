use core::ops::Range;
use std::marker::PhantomData;

use glam::Vec3;

pub trait Curve<T> {
    // Required methods
    fn domain(&self) -> Range<f32>;
    fn sample_unchecked(&self, t: f32) -> T;

    // Provided methods
    fn sample(&self, t: f32) -> T;

    fn length(&self) -> f32;

    fn t_at_distance(&self, distance: f32) -> f32;
}

struct CurveSample {
    pub position: Vec3,
    pub tangent: Vec3,
    pub up: Vec3,
    pub time: f32,
}

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

// Polyline
// Polygon

// Lowering.

// Whole Graph with debug info
// Lower to -> Compiled Graph -> Ideally same representation CPU and GPU side.
// Compiled Graph has  executor.
// How to add gizmos? -> Probably to the node somehow but that also requires evaluation first to properly display.
// CPU execution step is always in front as a base evaluation -> So the two "extra" executors should then run on GPU and CPU.

// How to design the nodes in order to have a property wrapper?
// -> Nodes should pretty much not be in core but rather in oneiroi-node or oeiroi-graph or similar.
// There they can receive special handling where performace is not a concern at all.

// We need to support impl Trait as return Types aka. opaque types to type check aginst it.
// So we can throw compile errors when a connection changes to a unsupported output type or smth.
// That also means a socket based caching architecture.

// In light of a lua integration... how to design the exposed develop feature for properties?
// We probably need to write a rust wrapper for it and expose it as a function which would actually be pretty elegant i suppose.

// The Core graph aka the graph which got compiled should then probably also live in the core crate.
