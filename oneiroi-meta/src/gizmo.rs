//! Gizmos: what an editor draws of the inputs of a node in its viewport,
//! and how dragging what it draws changes them.
//!
//! Everything here is plain data and plain functions over the words of
//! values, so it needs nothing of the engine. The nodes say what their
//! gizmos are, see [`GizmoMeta`], and an editor draws them however it likes.

/// A point in space.
pub type Point = [f32; 3];

/// Where a handle can be dragged to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Constraint {
    /// Anywhere: an editor moves it in the plane through it which faces the
    /// viewer, unless it is told otherwise.
    Free,
    /// Along the line through it in this direction.
    Axis(Point),
    /// In the plane through it with this normal.
    Plane(Point),
}

/// Something to drag, which stands for an element of an input.
#[derive(Debug, Clone, PartialEq)]
pub struct Handle {
    pub at: Point,
    /// The input it changes.
    pub port: &'static str,
    /// Which element of that input it stands for.
    pub element: usize,
    pub constraint: Constraint,
}

/// Something drawn to show what the handles mean, which is not dragged.
#[derive(Debug, Clone, PartialEq)]
pub enum Guide {
    /// Straight lines through the points, one after the other.
    Polyline(Vec<Point>),
    /// A circle around `center`, in the plane with the normal `normal`.
    Circle { center: Point, radius: f32, normal: Point },
}

/// What a gizmo shows of a node.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Gizmo {
    pub handles: Vec<Handle>,
    pub guides: Vec<Guide>,
}

/// An input set to new words, which a drag makes.
#[derive(Debug, Clone, PartialEq)]
pub struct Edit {
    pub port: &'static str,
    pub words: Vec<u32>,
}

/// The inputs of a node a gizmo can change, by name, as the words of their
/// values. An input which is linked is not among them: what it is depends
/// on another node.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GizmoInputs {
    values: Vec<(String, Vec<u32>)>,
}

impl GizmoInputs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, port: impl Into<String>, words: Vec<u32>) -> Self {
        self.values.push((port.into(), words));
        self
    }

    /// The words of input `port`, unless it is linked.
    pub fn get(&self, port: &str) -> Option<&[u32]> {
        self.values.iter().find(|(name, _)| name == port).map(|(_, words)| words.as_slice())
    }
}

/// The gizmo of a node, by its id.
#[derive(Debug, Clone, Copy)]
pub struct GizmoMeta {
    pub node: &'static str,
    /// What it shows of these inputs.
    pub show: fn(&GizmoInputs) -> Gizmo,
    /// What a handle it showed, dragged to a point, changes.
    pub drag: fn(&GizmoInputs, &Handle, Point) -> Vec<Edit>,
}
