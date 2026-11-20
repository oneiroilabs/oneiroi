//! Curves made from a few numbers: lines, polylines, and the curves of
//! circles and rectangles. They are all NURBS curves like any other, so
//! everything which takes a curve takes them.

use std::f32::consts::FRAC_1_SQRT_2;

use glam::{Vec2, Vec3, Vec4};
use oneiroi_core::{
    node::{GenericPort, Inputs, NodeDef, NodeRegistry, Outputs, PortDecl},
    types::{DataType, PortDefault},
};

use super::nurbs::{CubicNurbsSegmentCache, extract_segments};
use crate::{
    generic::version,
    primitive::{Circle, Rectangle},
};

/// Cubic Bézier pieces, each its four homogeneous control points, as the
/// segments of one curve: every piece becomes a segment of its own, so
/// where two of them meet at an angle the curve has a sharp corner.
fn pieces_to_curve(pieces: &[[Vec4; 4]]) -> Vec<CubicNurbsSegmentCache> {
    let Some(first) = pieces.first() else {
        return Vec::new();
    };
    let mut points = vec![first[0]];
    let mut knots = vec![0.0; 4];
    for (index, piece) in pieces.iter().enumerate() {
        points.extend_from_slice(&piece[1..]);
        // Every knot between two pieces three times: they only share the
        // point where they meet.
        let knot = (index + 1) as f32;
        let times = if index + 1 == pieces.len() { 4 } else { 3 };
        knots.extend(std::iter::repeat_n(knot, times));
    }
    extract_segments(&points, &knots)
}

/// The straight piece from `a` to `b`, which runs as fast everywhere.
fn straight(a: Vec3, b: Vec3) -> [Vec4; 4] {
    [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0].map(|t| a.lerp(b, t).extend(1.0))
}

/// The arc around `center` from `from` to `to`, both a quarter turn apart
/// at the same distance from it: a weighted quadratic, which is exact, as
/// the cubic it is the same as.
fn quarter_arc(center: Vec3, from: Vec3, to: Vec3) -> [Vec4; 4] {
    // The corner of the square the arc is inscribed in, with the weight
    // which makes it a circle.
    let corner = from + to - center;
    let quadratic = [from.extend(1.0), (corner * FRAC_1_SQRT_2).extend(FRAC_1_SQRT_2), to.extend(1.0)];
    [
        quadratic[0],
        (quadratic[0] + quadratic[1] * 2.0) / 3.0,
        (quadratic[1] * 2.0 + quadratic[2]) / 3.0,
        quadratic[2],
    ]
}

/// The curve through `points` with straight edges between them, and one
/// back to the first if it is `closed`. Points which repeat the one before
/// them are left out: an edge of no length has no direction.
pub fn polyline(points: &[Vec3], closed: bool) -> Vec<CubicNurbsSegmentCache> {
    let mut corners: Vec<Vec3> = Vec::with_capacity(points.len() + 1);
    for point in points {
        if corners.last().is_none_or(|last| last.distance_squared(*point) > 0.0) {
            corners.push(*point);
        }
    }
    if closed && corners.len() > 2 && corners[0] != *corners.last().unwrap() {
        corners.push(corners[0]);
    }
    let pieces: Vec<_> = corners.windows(2).map(|edge| straight(edge[0], edge[1])).collect();
    pieces_to_curve(&pieces)
}

/// Something flat in the plane of `x` and `y` which is a closed curve as
/// well, like a circle.
pub trait Outline: DataType + Sync {
    /// The curve along it, counter-clockwise, as the segments of one curve.
    fn curve(&self) -> Vec<CubicNurbsSegmentCache>;
}

/// Four quarter arcs from the `x` axis on, like the circle runs.
impl Outline for Circle {
    fn curve(&self) -> Vec<CubicNurbsSegmentCache> {
        let radius = self.radius().abs();
        if radius == 0.0 {
            return Vec::new();
        }
        let at = |quarter: usize| match quarter % 4 {
            0 => Vec3::X,
            1 => Vec3::Y,
            2 => -Vec3::X,
            _ => -Vec3::Y,
        } * radius;
        let pieces: Vec<_> = (0..4).map(|quarter| quarter_arc(Vec3::ZERO, at(quarter), at(quarter + 1))).collect();
        pieces_to_curve(&pieces)
    }
}

/// From the lower right corner up, with a quarter arc for every round
/// corner.
impl Outline for Rectangle {
    fn curve(&self) -> Vec<CubicNurbsSegmentCache> {
        let half = self.size() * 0.5;
        if half.min_element() <= 0.0 {
            return Vec::new();
        }
        let radius = self.corner_radius();
        let mut pieces = Vec::with_capacity(8);
        // The corners counter-clockwise, each with the direction of the
        // side which leaves it.
        let corners = [
            (Vec2::new(half.x, -half.y), Vec2::Y),
            (Vec2::new(half.x, half.y), -Vec2::X),
            (Vec2::new(-half.x, half.y), -Vec2::Y),
            (Vec2::new(-half.x, -half.y), Vec2::X),
        ];
        for (index, (corner, along)) in corners.iter().enumerate() {
            let (next, _) = corners[(index + 1) % 4];
            // The side, cut short by the round corners at both ends.
            let (start, end) = (*corner + *along * radius, next - *along * radius);
            if start.distance_squared(end) > 0.0 {
                pieces.push(straight(start.extend(0.0), end.extend(0.0)));
            }
            if radius > 0.0 {
                let (_, turn) = corners[(index + 1) % 4];
                let center = next - *along * radius + turn * radius;
                pieces.push(quarter_arc(center.extend(0.0), end.extend(0.0), (next + turn * radius).extend(0.0)));
            }
        }
        pieces_to_curve(&pieces)
    }
}

describe_trait! {
    /// A flat closed shape which is a curve as well, like a circle or a
    /// rectangle. It becomes one wherever a curve is needed.
    pub const OUTLINE = "outline" { label: "Outline", hue: Orange }
}

generic_node! {
    /// The curve along a flat shape, like a circle or a rectangle, a curve for
    /// each shape.
    ///
    /// A shape linked to a port which takes a curve goes through this on its
    /// own, so it is only needed to make that visible.
    pub static TO_CURVE = "curve.to_curve" {
        label: "To Curve",
        category: "Curve",
        inputs {
            /// The shapes.
            port GenericPort::implementing("shape", OUTLINE), "Shapes";
        }
        outputs {
            /// Their curves, counter-clockwise, one for each shape.
            port GenericPort::exact::<CubicNurbsSegmentCache>("curve").grouped(), "Curves";
        }
    }
}

/// Makes `T` an outline: [`TO_CURVE`] takes it, and so does every port
/// which takes a curve.
pub fn register_outline<T: Outline>(registry: &mut NodeRegistry) {
    let node = version(&TO_CURVE, T::NAME, |id| NodeDef {
        id,
        inputs: OutlinePorts::<T>::INPUTS,
        outputs: OutlinePorts::<T>::OUTPUTS,
        version: 1,
        cpu: to_curve::<T>,
        gpu: &[],
    });
    registry.register_instance(&TO_CURVE, node);
}

struct OutlinePorts<T>(std::marker::PhantomData<T>);

impl<T: Outline> OutlinePorts<T> {
    const INPUTS: &'static [PortDecl] = &[PortDecl::new::<T>("shape")];
    const OUTPUTS: &'static [PortDecl] = &[PortDecl::new::<CubicNurbsSegmentCache>("curve").grouped()];
}

fn to_curve<T: Outline>(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let (segments, offsets) = curves(inputs.get::<T>(0).iter().map(T::curve));
    outputs.set(0, &segments);
    outputs.set_groups(0, &offsets);
}

/// Curves one after the other, and where each starts.
fn curves(curves: impl Iterator<Item = Vec<CubicNurbsSegmentCache>>) -> (Vec<CubicNurbsSegmentCache>, Vec<u32>) {
    let mut segments = Vec::new();
    let mut offsets = vec![0];
    for curve in curves {
        segments.extend(curve);
        offsets.push(segments.len() as u32);
    }
    (segments, offsets)
}

node! {
    /// Straight lines, each from a start to an end.
    pub static LINE = "curve.line" {
        label: "Line",
        category: "Curve",
        version: 1,
        inputs {
            /// Where each line starts. The shorter of the starts and ends repeats its
            /// last one.
            port PortDecl::new::<Vec3>("start").with_default(PortDefault::F32(&[0.0; 3]));
            /// Where each line ends.
            port PortDecl::new::<Vec3>("end").with_default(PortDefault::F32(&[1.0, 0.0, 0.0]));
        }
        outputs {
            /// The lines, a curve for each.
            port PortDecl::new::<CubicNurbsSegmentCache>("curve").grouped(), "Curves";
        }
        cpu: line,
        gpu: &[],
    }
}

fn line(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let (starts, ends) = (inputs.get::<Vec3>(0), inputs.get::<Vec3>(1));
    let count = match starts.is_empty() || ends.is_empty() {
        true => 0,
        false => starts.len().max(ends.len()),
    };
    let at = |values: &[Vec3], index: usize| values[index.min(values.len() - 1)];
    let (segments, offsets) = curves((0..count).map(|index| polyline(&[at(starts, index), at(ends, index)], false)));
    outputs.set(0, &segments);
    outputs.set_groups(0, &offsets);
}

node! {
    /// Straight edges through points, with sharp corners where they meet.
    pub static POLYLINE = "curve.polyline" {
        label: "Polyline",
        category: "Curve",
        version: 1,
        inputs {
            /// The points, a polyline for each group of them.
            port PortDecl::new::<Vec3>("points").grouped(), "Points";
            /// Whether an edge leads from the last point back to the first.
            port PortDecl::new::<u32>("closed").choices(&["open", "closed"]);
        }
        outputs {
            /// The polylines, a curve for each.
            port PortDecl::new::<CubicNurbsSegmentCache>("curve").grouped(), "Curves";
        }
        cpu: polyline_node,
        gpu: &[],
    }
}

fn polyline_node(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let points = inputs.get::<Vec3>(0);
    let closed = inputs.choice(1) == 1;
    let (segments, offsets) = curves(inputs.groups(0).into_iter().map(|group| polyline(&points[group], closed)));
    outputs.set(0, &segments);
    outputs.set_groups(0, &offsets);
}

pub fn register(registry: &mut NodeRegistry) {
    registry.register(&LINE);
    registry.register(&POLYLINE);
    registry.register(&crate::primitive::RECTANGLE);
    register_outline::<Circle>(registry);
    register_outline::<Rectangle>(registry);
    registry.add_conversion(&TO_CURVE);
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use super::*;

    fn length(curve: &[CubicNurbsSegmentCache]) -> f32 {
        curve.last().map_or(0.0, |segment| segment.cumulative_length())
    }

    /// Points all along `curve`, a few on every segment.
    fn points(curve: &[CubicNurbsSegmentCache]) -> Vec<Vec3> {
        curve
            .iter()
            .flat_map(|segment| (0..=16).map(|step| segment.evaluate(step as f32 / 16.0).0))
            .collect()
    }

    #[test]
    fn a_polyline_goes_straight_from_point_to_point() {
        let corners = [Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0), Vec3::new(3.0, 4.0, 0.0), Vec3::new(3.0, 4.0, 0.0)];
        let open = polyline(&corners, false);
        assert_eq!(open.len(), 2);
        assert!((length(&open) - 7.0).abs() < 1e-4);
        for (segment, (from, to)) in open.iter().zip([(corners[0], corners[1]), (corners[1], corners[2])]) {
            assert!(segment.evaluate(0.0).0.distance(from) < 1e-5);
            assert!(segment.evaluate(1.0).0.distance(to) < 1e-5);
            // As fast everywhere, so halfway is in the middle.
            assert!(segment.evaluate(0.5).0.distance((from + to) * 0.5) < 1e-5);
        }

        let closed = polyline(&corners, true);
        assert_eq!(closed.len(), 3);
        assert!((length(&closed) - 12.0).abs() < 1e-4);
        assert!(closed[2].evaluate(1.0).0.distance(Vec3::ZERO) < 1e-5);
        assert!(polyline(&corners[..1], true).is_empty());
    }

    #[test]
    fn a_circle_is_exact_as_a_curve() {
        let curve = Circle::new(2.0).curve();
        assert_eq!(curve.len(), 4);
        assert!((length(&curve) - TAU * 2.0).abs() < 1e-3, "{}", length(&curve));
        for point in points(&curve) {
            assert!((point.length() - 2.0).abs() < 1e-5 && point.z == 0.0);
        }
        // Counter-clockwise from the x axis, like the circle runs.
        assert!(curve[0].evaluate(0.0).0.distance(Vec3::new(2.0, 0.0, 0.0)) < 1e-5);
        assert!(curve[0].evaluate(1.0).0.distance(Vec3::new(0.0, 2.0, 0.0)) < 1e-5);
        assert!(Circle::new(0.0).curve().is_empty());
    }

    #[test]
    fn a_rectangle_has_sharp_or_round_corners() {
        let sharp = Rectangle::new(4.0, 2.0, 0.0).curve();
        assert_eq!(sharp.len(), 4);
        assert!((length(&sharp) - 12.0).abs() < 1e-4);
        assert!(sharp[0].evaluate(0.0).0.distance(Vec3::new(2.0, -1.0, 0.0)) < 1e-5);
        assert!(sharp[3].evaluate(1.0).0.distance(Vec3::new(2.0, -1.0, 0.0)) < 1e-5);

        let round = Rectangle::new(4.0, 2.0, 0.5).curve();
        assert_eq!(round.len(), 8);
        let expected = 12.0 - 8.0 * 0.5 + TAU * 0.5;
        assert!((length(&round) - expected).abs() < 1e-3, "{}", length(&round));
        for point in points(&round) {
            // Inside the rectangle, and on the arc where it is round.
            assert!(point.x.abs() <= 2.0 + 1e-5 && point.y.abs() <= 1.0 + 1e-5);
            let center = Vec2::new(point.x.signum() * 1.5, point.y.signum() * 0.5);
            if point.x.abs() > 1.5 && point.y.abs() > 0.5 {
                assert!((point.truncate().distance(center) - 0.5).abs() < 1e-4);
            }
        }
        // A side as long as both corners together is only the corners.
        assert_eq!(Rectangle::new(1.0, 2.0, 0.5).curve().len(), 6);
        assert!(Rectangle::new(0.0, 2.0, 0.0).curve().is_empty());
    }
}
