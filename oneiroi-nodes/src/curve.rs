use core::ops::Range;
use std::{iter, marker::PhantomData};

use glam::Vec3;

mod nodes;
pub mod nurbs;
pub mod ops;
mod profile;
pub mod shapes;
mod sweep;

pub use nodes::{
    ControlPoint, EvaluatedFrame, NURBS, NURBS_BATCH, SAMPLE_CURVE, SAMPLE_CURVE_BATCH, TO_FRAMES, TRANSLATE, register,
};
pub(crate) use nodes::example_curves;
pub(crate) use profile::{
    LATE, SPIKE, cornered_curve, cornered_spine, example_curve, fractions, sweep_example, sweep_example_along,
    sweep_single_example,
};
pub use profile::{PROFILE, Profile};
pub use shapes::{LINE, OUTLINE, Outline, POLYLINE, TO_CURVE, polyline, register_outline};
pub(crate) use sweep::sweep_curve;
pub use sweep::{Piece, SWEEP, SWEEP_BATCH, SweptPatch, TubePatch, kind, register_profile, swept, swept_batch, tube};
#[cfg(feature = "meta")]
pub use {
    nodes::{NURBS_META, SAMPLE_CURVE_META, TO_FRAMES_META, TRANSLATE_META},
    profile::PROFILE_META,
    shapes::{LINE_META, OUTLINE_META, POLYLINE_META, TO_CURVE_META},
    sweep::SWEEP_META,
};

pub trait Curve<Sample> {
    // Required methods
    fn domain(&self) -> Range<f32>;
    fn sample_unchecked(&self, t: f32) -> Sample;

    // Provided methods
    fn sample(&self, t: f32) -> Sample;

    fn length(&self) -> f32;

    fn t_at_distance(&self, distance: f32) -> f32;
}

pub struct RmfSample {
    pub position: Vec3,
    pub tangent: Vec3,
    pub up: Vec3,
    pub time: f32,
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
