use core::ops::Range;
use std::{iter, marker::PhantomData};

use glam::Vec3;

pub mod nurbs;
pub mod ops;

pub trait Curve<Sv> {
    // Required methods
    fn domain(&self) -> Range<f32>;
    fn sample_unchecked(&self, t: f32) -> Sv;

    // Provided methods
    fn sample(&self, t: f32) -> Sv;

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
