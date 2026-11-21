//! The gizmos of the nodes of this crate: what an editor draws of their
//! inputs in its viewport, and how dragging changes them. See
//! [`oneiroi_meta::gizmo`].
//!
//! Only an editor needs them, so they are only there with the `meta`
//! feature, like the rest of what people see of the nodes.

use bytemuck::Pod;
use glam::{Vec2, Vec3};
use oneiroi_core::types::DataType;
use oneiroi_meta::{
    MetaRegistry,
    gizmo::{Constraint, Edit, Gizmo, GizmoInputs, GizmoMeta, Guide, Handle, Point},
};

use crate::{
    curve::{CURVE_2D, ControlPoint, LINE, NURBS, POLYLINE, TRANSLATE},
    primitive::{CIRCLE, RECTANGLE},
    value::{COMBINE_XYZ, VEC3},
};

/// A value which is somewhere in space, and can be dragged elsewhere.
pub trait Handled: DataType {
    /// Where it is.
    fn position(&self) -> Vec3;
    /// It, dragged to `to`. Whatever else it is stays as it is.
    fn moved(&mut self, to: Vec3);
    /// Where it can be dragged.
    const CONSTRAINT: Constraint = Constraint::Free;
}

impl Handled for Vec3 {
    fn position(&self) -> Vec3 {
        *self
    }

    fn moved(&mut self, to: Vec3) {
        *self = to;
    }
}

/// A point in the plane of `x` and `y`.
impl Handled for Vec2 {
    fn position(&self) -> Vec3 {
        self.extend(0.0)
    }

    fn moved(&mut self, to: Vec3) {
        *self = to.truncate();
    }

    const CONSTRAINT: Constraint = Constraint::Plane([0.0, 0.0, 1.0]);
}

/// Its position, which keeps its weight.
impl Handled for ControlPoint {
    fn position(&self) -> Vec3 {
        self.position
    }

    fn moved(&mut self, to: Vec3) {
        self.position = to;
    }
}

/// The elements of `words`, as many whole ones as there are.
fn read<T: Pod>(words: &[u32]) -> Vec<T> {
    let size = size_of::<T>() / 4;
    bytemuck::pod_collect_to_vec(&words[..words.len() / size * size])
}

fn words<T: Pod>(values: &[T]) -> Vec<u32> {
    bytemuck::cast_slice(values).to_vec()
}

fn point(at: Vec3) -> Point {
    at.to_array()
}

/// The numbers of input `port`, or none if it is linked.
fn numbers(inputs: &GizmoInputs, port: &str) -> Option<Vec<f32>> {
    inputs.get(port).map(read::<f32>)
}

/// A handle for every element of input `port`, joined by straight lines one
/// after the other if it is a `polygon`, like the control points of a curve.
fn points<T: Handled>(inputs: &GizmoInputs, port: &'static str, polygon: bool) -> Gizmo {
    let Some(values) = inputs.get(port).map(read::<T>) else {
        return Gizmo::default();
    };
    let at: Vec<Point> = values.iter().map(|value| point(value.position())).collect();
    let handles = at
        .iter()
        .enumerate()
        .map(|(element, at)| Handle { at: *at, port, element, constraint: T::CONSTRAINT })
        .collect();
    let guides = match polygon && at.len() > 1 {
        true => vec![Guide::Polyline(at)],
        false => Vec::new(),
    };
    Gizmo { handles, guides }
}

/// The element of `handle` dragged to `to`.
fn moved<T: Handled>(inputs: &GizmoInputs, handle: &Handle, to: Point) -> Vec<Edit> {
    let Some(mut values) = inputs.get(handle.port).map(read::<T>) else {
        return Vec::new();
    };
    let Some(value) = values.get_mut(handle.element) else {
        return Vec::new();
    };
    value.moved(Vec3::from_array(to));
    vec![Edit { port: handle.port, words: words(&values) }]
}

/// The control points of a curve, joined as its control polygon.
fn nurbs(inputs: &GizmoInputs) -> Gizmo {
    points::<ControlPoint>(inputs, "points", true)
}

/// The corners of a polyline, and the edges between them.
fn polyline(inputs: &GizmoInputs) -> Gizmo {
    points::<Vec3>(inputs, "points", true)
}

/// The points of a Curve 2D, in the plane of `x` and `y`.
fn curve_2d(inputs: &GizmoInputs) -> Gizmo {
    points::<Vec2>(inputs, "points", true)
}

/// Vectors typed in, each a point on its own.
fn vectors(inputs: &GizmoInputs) -> Gizmo {
    points::<Vec3>(inputs, "values", false)
}

/// The start and the end of a line, and the line between them.
fn line(inputs: &GizmoInputs) -> Gizmo {
    let mut gizmo = Gizmo::default();
    let ends: Vec<Option<Vec3>> = ["start", "end"]
        .iter()
        .map(|port| inputs.get(port).and_then(|words| read::<Vec3>(words).first().copied()))
        .collect();
    for (port, end) in ["start", "end"].into_iter().zip(&ends) {
        if let Some(end) = end {
            gizmo.handles.push(Handle { at: point(*end), port, element: 0, constraint: Constraint::Free });
        }
    }
    if let [Some(start), Some(end)] = ends[..] {
        gizmo.guides.push(Guide::Polyline(vec![point(start), point(end)]));
    }
    gizmo
}

/// The radius, as a handle on `x` which only moves along it, and the circle.
fn circle(inputs: &GizmoInputs) -> Gizmo {
    let Some(radius) = numbers(inputs, "radius").and_then(|radius| radius.first().copied()) else {
        return Gizmo::default();
    };
    Gizmo {
        handles: vec![Handle {
            at: [radius.abs(), 0.0, 0.0],
            port: "radius",
            element: 0,
            constraint: Constraint::Axis([1.0, 0.0, 0.0]),
        }],
        guides: vec![Guide::Circle { center: [0.0; 3], radius: radius.abs(), normal: [0.0, 0.0, 1.0] }],
    }
}

fn circle_dragged(_: &GizmoInputs, _: &Handle, to: Point) -> Vec<Edit> {
    vec![Edit { port: "radius", words: words(&[to[0].abs()]) }]
}

/// A corner, which sets the width and the height, and the rectangle.
fn rectangle(inputs: &GizmoInputs) -> Gizmo {
    let side = |port| numbers(inputs, port).and_then(|values| values.first().copied());
    let (Some(width), Some(height)) = (side("width"), side("height")) else {
        return Gizmo::default();
    };
    let (x, y) = (width.abs() * 0.5, height.abs() * 0.5);
    Gizmo {
        handles: vec![Handle { at: [x, y, 0.0], port: "width", element: 0, constraint: Constraint::Plane([0.0, 0.0, 1.0]) }],
        guides: vec![Guide::Polyline(vec![[x, y, 0.0], [-x, y, 0.0], [-x, -y, 0.0], [x, -y, 0.0], [x, y, 0.0]])],
    }
}

fn rectangle_dragged(_: &GizmoInputs, _: &Handle, to: Point) -> Vec<Edit> {
    vec![
        Edit { port: "width", words: words(&[to[0].abs() * 2.0]) },
        Edit { port: "height", words: words(&[to[1].abs() * 2.0]) },
    ]
}

/// How far a curve is moved: an arrow from where it was.
fn translate(inputs: &GizmoInputs) -> Gizmo {
    let Some(offset) = inputs.get("offset").and_then(|words| read::<Vec3>(words).first().copied()) else {
        return Gizmo::default();
    };
    Gizmo {
        handles: vec![Handle { at: point(offset), port: "offset", element: 0, constraint: Constraint::Free }],
        guides: vec![Guide::Polyline(vec![[0.0; 3], point(offset)])],
    }
}

/// The vectors made of the numbers, element by element. Only when none of
/// the numbers is linked: what a vector is depends on all of them.
fn combine_xyz(inputs: &GizmoInputs) -> Gizmo {
    let Some(axes) = xyz(inputs) else {
        return Gizmo::default();
    };
    let count = match axes.iter().any(Vec::is_empty) {
        true => 0,
        false => axes.iter().map(Vec::len).max().unwrap_or(0),
    };
    let handles = (0..count)
        .map(|element| Handle {
            at: [0, 1, 2].map(|axis| axes[axis][element.min(axes[axis].len() - 1)]),
            port: "x",
            element,
            constraint: Constraint::Free,
        })
        .collect();
    Gizmo { handles, guides: Vec::new() }
}

fn xyz(inputs: &GizmoInputs) -> Option<[Vec<f32>; 3]> {
    Some([numbers(inputs, "x")?, numbers(inputs, "y")?, numbers(inputs, "z")?])
}

/// Every list as long as it takes to hold the element, repeating its last
/// number as it did before, and the element set.
fn combine_xyz_dragged(inputs: &GizmoInputs, handle: &Handle, to: Point) -> Vec<Edit> {
    let Some(mut axes) = xyz(inputs) else {
        return Vec::new();
    };
    ["x", "y", "z"]
        .into_iter()
        .zip(axes.iter_mut())
        .zip(to)
        .map(|((port, values), to)| {
            let last = values.last().copied().unwrap_or(0.0);
            if values.len() <= handle.element {
                values.resize(handle.element + 1, last);
            }
            values[handle.element] = to;
            Edit { port, words: words(values) }
        })
        .collect()
}

/// Makes the gizmos of the nodes of this crate known to `meta`.
pub fn describe(meta: &mut MetaRegistry) {
    for gizmo in [
        GizmoMeta { node: NURBS.id, show: nurbs, drag: moved::<ControlPoint> },
        GizmoMeta { node: POLYLINE.id, show: polyline, drag: moved::<Vec3> },
        GizmoMeta { node: VEC3.id, show: vectors, drag: moved::<Vec3> },
        GizmoMeta { node: CURVE_2D.id, show: curve_2d, drag: moved::<Vec2> },
        GizmoMeta { node: LINE.id, show: line, drag: moved::<Vec3> },
        GizmoMeta { node: CIRCLE.id, show: circle, drag: circle_dragged },
        GizmoMeta { node: RECTANGLE.id, show: rectangle, drag: rectangle_dragged },
        GizmoMeta { node: TRANSLATE.id, show: translate, drag: moved::<Vec3> },
        GizmoMeta { node: COMBINE_XYZ.id, show: combine_xyz, drag: combine_xyz_dragged },
    ] {
        meta.add_gizmo(gizmo);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gizmo_of(id: &str) -> GizmoMeta {
        let mut meta = MetaRegistry::new();
        describe(&mut meta);
        *meta.gizmo(id).unwrap()
    }

    #[test]
    fn control_points_are_dragged_and_keep_their_weight() {
        let nurbs = gizmo_of("curve.nurbs");
        let points = [ControlPoint::at(Vec3::ZERO), ControlPoint::new(Vec3::X, 2.0), ControlPoint::at(Vec3::Y)];
        let inputs = GizmoInputs::new().with("points", words(&points));
        let gizmo = (nurbs.show)(&inputs);
        assert_eq!(gizmo.handles.len(), 3);
        assert_eq!(gizmo.handles[1].at, [1.0, 0.0, 0.0]);
        // Joined as the control polygon.
        assert_eq!(gizmo.guides, [Guide::Polyline(vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]])]);

        let edits = (nurbs.drag)(&inputs, &gizmo.handles[1], [1.0, 2.0, 3.0]);
        let moved: Vec<ControlPoint> = read(&edits[0].words);
        assert_eq!(moved[1], ControlPoint::new(Vec3::new(1.0, 2.0, 3.0), 2.0));
        assert_eq!((moved[0], moved[2]), (points[0], points[2]));

        // Linked, the points are not the node's to show.
        assert_eq!((nurbs.show)(&GizmoInputs::new()), Gizmo::default());
    }

    #[test]
    fn a_curve_2d_stays_flat() {
        let curve = gizmo_of("curve.curve_2d");
        let inputs = GizmoInputs::new().with("points", words(&[Vec2::ZERO, Vec2::ONE]));
        let gizmo = (curve.show)(&inputs);
        assert_eq!(gizmo.handles[1].constraint, Constraint::Plane([0.0, 0.0, 1.0]));
        let edits = (curve.drag)(&inputs, &gizmo.handles[1], [0.5, 2.0, 7.0]);
        assert_eq!(read::<Vec2>(&edits[0].words), [Vec2::ZERO, Vec2::new(0.5, 2.0)]);
    }

    #[test]
    fn shapes_are_changed_by_their_handles() {
        let circle = gizmo_of("curve.circle");
        let inputs = GizmoInputs::new().with("radius", words(&[0.5f32]));
        let gizmo = (circle.show)(&inputs);
        assert_eq!(gizmo.handles[0].constraint, Constraint::Axis([1.0, 0.0, 0.0]));
        let edits = (circle.drag)(&inputs, &gizmo.handles[0], [-2.0, 1.0, 0.0]);
        assert_eq!(read::<f32>(&edits[0].words), [2.0]);

        let rectangle = gizmo_of("curve.rectangle");
        let inputs = GizmoInputs::new().with("width", words(&[1.0f32])).with("height", words(&[3.0f32]));
        let gizmo = (rectangle.show)(&inputs);
        assert_eq!(gizmo.handles[0].at, [0.5, 1.5, 0.0]);
        let edits = (rectangle.drag)(&inputs, &gizmo.handles[0], [1.0, 0.25, 0.0]);
        assert_eq!(edits.iter().map(|edit| read::<f32>(&edit.words)[0]).collect::<Vec<_>>(), [2.0, 0.5]);
    }

    #[test]
    fn vectors_of_numbers_grow_where_they_are_dragged() {
        let combine = gizmo_of("value.combine_xyz");
        let inputs = GizmoInputs::new()
            .with("x", words(&[0.0f32, 1.0, 2.0]))
            .with("y", words(&[5.0f32]))
            .with("z", words(&[0.0f32]));
        let gizmo = (combine.show)(&inputs);
        assert_eq!(gizmo.handles.iter().map(|handle| handle.at).collect::<Vec<_>>(), [
            [0.0, 5.0, 0.0],
            [1.0, 5.0, 0.0],
            [2.0, 5.0, 0.0]
        ]);
        // The third vector gets a `y` and a `z` of its own; the others keep
        // theirs.
        let edits = (combine.drag)(&inputs, &gizmo.handles[2], [2.0, 6.0, 1.0]);
        let axes: Vec<Vec<f32>> = edits.iter().map(|edit| read(&edit.words)).collect();
        assert_eq!(axes, [vec![0.0, 1.0, 2.0], vec![5.0, 5.0, 6.0], vec![0.0, 0.0, 1.0]]);
        // One linked axis and there is nothing to drag.
        assert!((combine.show)(&GizmoInputs::new().with("x", words(&[0.0f32]))).handles.is_empty());
    }

    #[test]
    fn a_line_is_dragged_by_its_ends() {
        let line = gizmo_of("curve.line");
        let inputs = GizmoInputs::new().with("start", words(&[Vec3::ZERO])).with("end", words(&[Vec3::X]));
        let gizmo = (line.show)(&inputs);
        assert_eq!(gizmo.handles.iter().map(|handle| handle.port).collect::<Vec<_>>(), ["start", "end"]);
        let edits = (line.drag)(&inputs, &gizmo.handles[1], [0.0, 2.0, 0.0]);
        assert_eq!((edits[0].port, read::<Vec3>(&edits[0].words)), ("end", vec![Vec3::new(0.0, 2.0, 0.0)]));
    }
}
