//! A curve drawn in an editor as a graph over `x`, like how big a profile
//! is along a sweep. It is a NURBS curve like any other, flat in the plane
//! of `x` and `y`.

use glam::{Vec2, Vec3, Vec4};
use oneiroi_core::node::{Inputs, Outputs, PortDecl};

use super::{
    nurbs::{CubicNurbsSegmentCache, clamped_uniform_knots, extract_segments},
    shapes::{pieces_to_curve, polyline},
};

/// How a [`CURVE_2D`] gets from point to point, by the index of its choice.
pub mod interpolation {
    /// Through every point, without overshooting between two of them.
    pub const SMOOTH: u32 = 0;
    /// Straight from point to point.
    pub const LINEAR: u32 = 1;
    /// Pulled by the points as the control points of a NURBS curve: through
    /// the first and the last, and smoother than through all of them.
    pub const NURBS: u32 = 2;
}

/// The names of the choices of [`CURVE_2D`], in the order of [`interpolation`].
pub const INTERPOLATIONS: &[&str] = &["smooth", "linear", "nurbs"];

node! {
    /// A curve drawn as a graph over `x`: through points, from the left to the
    /// right. It is flat in the plane of `x` and `y`.
    ///
    /// Used where a curve says how much of something there is along the way,
    /// like the `scale` of a sweep: `x` is how far along, from `0` to `1`, and
    /// `y` how much there. Any input which takes a curve can draw one of these
    /// right where it is, without a node of its own.
    pub static CURVE_2D = "curve.curve_2d" {
        label: "Curve 2D",
        category: "Curve",
        version: 1,
        inputs {
            /// The points, taken from the left to the right by their `x`. Of points
            /// with the same `x`, only the last counts.
            port PortDecl::new::<Vec2>("points").with_default(&[Vec2::new(0.0, 1.0), Vec2::new(1.0, 1.0)]);
            /// How the curve gets from point to point: `smooth` through every point,
            /// `linear` straight, or `nurbs` pulled by the points as the control
            /// points of a NURBS curve. With fewer than four points, `nurbs` goes
            /// straight.
            port PortDecl::new::<u32>("interpolation").choices(INTERPOLATIONS);
        }
        outputs {
            /// The curve.
            port PortDecl::new::<CubicNurbsSegmentCache>("curve").one();
        }
        cpu: curve_2d,
        gpu: &[],
    }
}

fn curve_2d(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let interpolation = inputs.choice(1) as u32;
    outputs.set(0, &curve_through(inputs.get::<Vec2>(0), interpolation));
}

/// The segments of the curve of [`CURVE_2D`] through `points`, which the
/// editor draws as well.
pub fn curve_through(points: &[Vec2], interpolation: u32) -> Vec<CubicNurbsSegmentCache> {
    let points = by_x(points);
    match interpolation {
        interpolation::LINEAR => linear(&points),
        interpolation::NURBS if points.len() >= 4 => {
            let control: Vec<Vec4> = points.iter().map(|point| point.extend(0.0).extend(1.0)).collect();
            extract_segments(&control, &clamped_uniform_knots(control.len()))
        }
        interpolation::NURBS => linear(&points),
        _ => smooth(&points),
    }
}

/// `points` from the left to the right, and of those with the same `x` the
/// last. Numbers which are no numbers are left out.
fn by_x(points: &[Vec2]) -> Vec<Vec2> {
    let mut sorted: Vec<Vec2> = points.iter().copied().filter(|point| point.is_finite()).collect();
    // Stable, so of the same `x` the last stays last.
    sorted.sort_by(|a, b| a.x.total_cmp(&b.x));
    let mut kept: Vec<Vec2> = Vec::with_capacity(sorted.len());
    for point in sorted {
        match kept.last_mut() {
            Some(last) if last.x == point.x => *last = point,
            _ => kept.push(point),
        }
    }
    kept
}

fn linear(points: &[Vec2]) -> Vec<CubicNurbsSegmentCache> {
    let points: Vec<Vec3> = points.iter().map(|point| point.extend(0.0)).collect();
    polyline(&points, false)
}

/// A cubic through every point, with the slopes of Fritsch and Carlson: it
/// rises or falls between two points only as they do, so it never goes
/// beyond them, and it is flat where a point is higher or lower than both
/// of its neighbours.
fn smooth(points: &[Vec2]) -> Vec<CubicNurbsSegmentCache> {
    if points.len() < 3 {
        return linear(points);
    }
    let slopes: Vec<f32> = points.windows(2).map(|pair| (pair[1].y - pair[0].y) / (pair[1].x - pair[0].x)).collect();
    let mut tangents = Vec::with_capacity(points.len());
    tangents.push(slopes[0]);
    for pair in slopes.windows(2) {
        tangents.push(match pair[0] * pair[1] > 0.0 {
            true => (pair[0] + pair[1]) * 0.5,
            false => 0.0,
        });
    }
    tangents.push(slopes[slopes.len() - 1]);
    for (index, slope) in slopes.iter().enumerate() {
        if *slope == 0.0 {
            tangents[index] = 0.0;
            tangents[index + 1] = 0.0;
            continue;
        }
        let (a, b) = (tangents[index] / slope, tangents[index + 1] / slope);
        let length = (a * a + b * b).sqrt();
        if length > 3.0 {
            tangents[index] = 3.0 / length * a * slope;
            tangents[index + 1] = 3.0 / length * b * slope;
        }
    }
    let pieces: Vec<[Vec4; 4]> = points
        .windows(2)
        .zip(tangents.windows(2))
        .map(|(pair, tangent)| {
            let third = (pair[1].x - pair[0].x) / 3.0;
            [
                pair[0],
                pair[0] + Vec2::new(third, tangent[0] * third),
                pair[1] - Vec2::new(third, tangent[1] * third),
                pair[1],
            ]
            .map(|point| point.extend(0.0).extend(1.0))
        })
        .collect();
    pieces_to_curve(&pieces)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `y` of the curve at `x`, found by halving along the segment which
    /// covers `x`.
    fn y_at(curve: &[CubicNurbsSegmentCache], x: f32) -> f32 {
        let segment = curve
            .iter()
            .find(|segment| segment.evaluate(1.0).0.x >= x)
            .unwrap_or(curve.last().unwrap());
        let (mut low, mut high) = (0.0f32, 1.0f32);
        for _ in 0..40 {
            let middle = (low + high) * 0.5;
            match segment.evaluate(middle).0.x < x {
                true => low = middle,
                false => high = middle,
            }
        }
        segment.evaluate(low).0.y
    }

    const KEYS: [Vec2; 4] = [Vec2::new(0.0, 0.0), Vec2::new(0.25, 0.75), Vec2::new(0.55, 0.7), Vec2::new(1.0, 1.0)];

    #[test]
    fn a_smooth_curve_goes_through_its_points_and_not_beyond() {
        let curve = curve_through(&KEYS, interpolation::SMOOTH);
        assert_eq!(curve.len(), 3);
        for key in KEYS {
            assert!((y_at(&curve, key.x) - key.y).abs() < 1e-4, "{key}");
        }
        // Between the two middle points it falls a little and no further.
        for step in 0..=20 {
            let x = 0.25 + 0.3 * step as f32 / 20.0;
            let y = y_at(&curve, x);
            assert!((0.7 - 1e-4..=0.75 + 1e-4).contains(&y), "{x}: {y}");
        }
        // `x` only grows, so it is a graph over `x`.
        for segment in &curve {
            let xs: Vec<f32> = (0..=16).map(|step| segment.evaluate(step as f32 / 16.0).0.x).collect();
            assert!(xs.windows(2).all(|pair| pair[1] >= pair[0]));
        }
    }

    #[test]
    fn points_are_taken_from_the_left_to_the_right() {
        let shuffled = [KEYS[2], KEYS[0], KEYS[3], KEYS[1], Vec2::new(0.25, 0.5)];
        let curve = curve_through(&shuffled, interpolation::LINEAR);
        assert_eq!(curve.len(), 3);
        assert!((y_at(&curve, 0.25) - 0.5).abs() < 1e-5);
        assert!((y_at(&curve, 0.4) - 0.6).abs() < 1e-5);
        assert!(curve_through(&[Vec2::ONE], interpolation::SMOOTH).is_empty());
    }

    #[test]
    fn nurbs_is_pulled_by_its_points() {
        let curve = curve_through(&KEYS, interpolation::NURBS);
        assert_eq!(curve.len(), 1);
        // Through the first and the last point only.
        assert!(curve[0].evaluate(0.0).0.distance(Vec3::ZERO) < 1e-5);
        assert!(curve[0].evaluate(1.0).0.distance(Vec3::ONE.with_z(0.0)) < 1e-5);
        assert!((y_at(&curve, 0.25) - 0.75).abs() > 0.05);
        // With fewer points it goes straight.
        assert_eq!(curve_through(&KEYS[..3], interpolation::NURBS).len(), 2);
    }
}
