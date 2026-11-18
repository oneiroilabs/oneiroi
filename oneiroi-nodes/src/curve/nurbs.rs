use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec4, Vec4Swizzles};
use oneiroi_core::types::{DataType, Field, FieldType, Layout, Scalar};

use crate::curve::Curve;

// 5-Point Gauss–Legendre Quadrature
const GAUSS_NODES: [f32; 5] = [0.0, -0.538_469_3, 0.538_469_3, -0.906_179_85, 0.906_179_85];
const GAUSS_WEIGHTS: [f32; 5] = [
    0.568_888_9,
    0.478_628_67,
    0.478_628_67,
    0.236_926_89,
    0.236_926_89,
];

/// At most this many Newton steps find where a segment is how long.
pub const NEWTON_STEPS: u32 = 8;

/// How many double reflection steps carry the frame across one segment.
/// This is one less than the 32 lanes the GPU evaluates per segment, so
/// both walk over the same samples.
pub const RMF_STEPS: u32 = 31;

/// GPU-Friendly structure accelerating the evaluation by:
/// - Caching the monomial basis via Bezier Extraction.
/// - Caching the normal at both ends in two dimensional space.
/// - Caching the length of the preceeding and currect segment.
///
/// It is stored as plain floats so it can live in the word pages of the
/// store. The layout is the one of the WGSL struct with the same name.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, PartialEq)]
pub struct CubicNurbsSegmentCache {
    /// Columns are the coefficients of t^3, t^2, t and 1.
    monomial_basis: [[f32; 4]; 4],

    length: f32,
    cumulative_length: f32,

    rmf_start_normal: [f32; 2],
    rmf_end_normal: [f32; 2],
    _padding: [f32; 2],
}

impl DataType for CubicNurbsSegmentCache {
    type ConfigurationOptions = ();

    const NAME: &'static str = "cubic_nurbs_segment";
    const WGSL: &'static str = "CubicNurbsSegmentCache";
    const SCALAR: Scalar = Scalar::F32;
    const LAYOUT: Layout = Layout::Struct(&[
        Field::new("monomial_basis", FieldType::Mat4),
        Field::new("length", FieldType::F32),
        Field::new("cumulative_length", FieldType::F32),
        Field::new("rmf_start_normal", FieldType::Vec2),
        Field::new("rmf_end_normal", FieldType::Vec2),
        Field::pad(2),
    ]);
}

describe_type! {
    /// A smooth curve, as the cubic segments it is made of. A group of
    /// segments is one curve.
    CubicNurbsSegmentCache { label: "Curve", hue: Amber }
}

/// The two vectors a normal is stored relative to. They only depend on the
/// tangent, so the GPU can rebuild them from the segment alone.
fn reference_basis(tangent: Vec3) -> (Vec3, Vec3) {
    let abs_t = tangent.abs();
    let ref_v = if abs_t.x < abs_t.y && abs_t.x < abs_t.z {
        Vec3::X
    } else if abs_t.y < abs_t.z {
        Vec3::Y
    } else {
        Vec3::Z
    };
    let n_ref = ref_v.cross(tangent).normalize_or_zero();
    let b_ref = tangent.cross(n_ref).normalize_or_zero();
    (n_ref, b_ref)
}

fn encode_normal(normal: Vec3, tangent: Vec3) -> [f32; 2] {
    let (n_ref, b_ref) = reference_basis(tangent);
    [normal.dot(n_ref), normal.dot(b_ref)]
}

fn decode_normal(normal: [f32; 2], tangent: Vec3) -> Vec3 {
    let (n_ref, b_ref) = reference_basis(tangent);
    n_ref * normal[0] + b_ref * normal[1]
}


impl CubicNurbsSegmentCache {
    fn from_basis(monomial_basis: Mat4) -> Self {
        Self {
            monomial_basis: monomial_basis.to_cols_array_2d(),
            ..Self::zeroed()
        }
    }

    pub fn length(&self) -> f32 {
        self.length
    }

    /// The length of the curve up to the end of this segment.
    pub fn cumulative_length(&self) -> f32 {
        self.cumulative_length
    }

    /// The same segment, moved by `offset`.
    pub fn translated(&self, offset: Vec3) -> Self {
        let mut moved = *self;
        // The columns are homogeneous, so the offset scales with the weight.
        for column in &mut moved.monomial_basis {
            let weight = column[3];
            for (value, offset) in column.iter_mut().zip(offset.to_array()) {
                *value += offset * weight;
            }
        }
        moved
    }

    /// The same segment turned by `rotation`, scaled by `scale` and then
    /// moved by `offset`, all about the origin. Its normals turn along, so
    /// its frames are the ones of before, turned. `rotation` has to be one,
    /// without any scale or mirroring of its own.
    pub fn placed(&self, rotation: Mat3, scale: f32, offset: Vec3) -> Self {
        let mut placed = *self;
        for column in &mut placed.monomial_basis {
            // Homogeneous, so the offset is scaled by the weight.
            let point = rotation * Vec3::new(column[0], column[1], column[2]) * scale + offset * column[3];
            column[..3].copy_from_slice(&point.to_array());
        }
        placed.length *= scale;
        placed.cumulative_length *= scale;
        let turn = |normal: [f32; 2], tangent: Vec3| {
            encode_normal(rotation * decode_normal(normal, tangent), rotation * tangent)
        };
        placed.rmf_start_normal = turn(self.rmf_start_normal, self.evaluate(0.0).2);
        placed.rmf_end_normal = turn(self.rmf_end_normal, self.evaluate(1.0).2);
        placed
    }

    /// Position, velocity and unit tangent at `u` in `0..=1`.
    pub fn evaluate(&self, u: f32) -> (Vec3, Vec3, Vec3) {
        let m = Mat4::from_cols_array_2d(&self.monomial_basis);
        let (a, b, c, d) = (m.col(0), m.col(1), m.col(2), m.col(3));

        let at = |u: f32| {
            let pos_hom = a
                .mul_add(Vec4::splat(u), b)
                .mul_add(Vec4::splat(u), c)
                .mul_add(Vec4::splat(u), d);
            let pos = pos_hom.xyz() / pos_hom.w;

            let dp_du_hom = a
                .mul_add(Vec4::splat(u * 3.0), b * 2.0)
                .mul_add(Vec4::splat(u), c);
            let velocity = (dp_du_hom.xyz() - dp_du_hom.w * pos) / pos_hom.w;
            (pos, velocity)
        };

        let (pos, velocity) = at(u);

        // Tangente mit robustem Fallback für verschwindende Ableitungen (z.B. geklemmte Ränder)
        let tangent = velocity.try_normalize().unwrap_or_else(|| {
            let u_eps = if u + 0.001 <= 1.0 {
                u + 0.001
            } else {
                u - 0.001
            };
            at(u_eps).1.normalize_or_zero()
        });

        (pos, velocity, tangent)
    }

    /// The length of the segment between `0` and `u`.
    pub fn length_until(&self, u: f32) -> f32 {
        if u <= 0.0 {
            return 0.0;
        }

        let mut integral = 0.0;
        for i in 0..5 {
            let sample = 0.5 * u * (GAUSS_NODES[i] + 1.0);
            integral += GAUSS_WEIGHTS[i] * self.evaluate(sample).1.length();
        }
        integral * 0.5 * u
    }

    /// The `u` at which the segment is `distance` long.
    pub fn u_at_distance(&self, distance: f32) -> f32 {
        if self.length <= 0.0 {
            return 0.0;
        }

        let mut u = (distance / self.length).clamp(0.0, 1.0);
        // The start is a guess which assumes the segment is as fast
        // everywhere. Where it is not, a few steps are needed.
        for _ in 0..NEWTON_STEPS {
            let speed = self.evaluate(u).1.length();
            if speed < 1e-5 {
                break;
            }

            let delta_u = (self.length_until(u) - distance) / speed;
            u = (u - delta_u).clamp(0.0, 1.0);

            if delta_u.abs() < 1e-5 {
                break;
            }
        }
        u
    }

    /// The rotation minimizing frame at `u` in `0..=1`.
    ///
    /// Only the normals at both ends of the segment are stored. Each one is
    /// carried to `u` by the shortest rotation between the tangents and the
    /// two results are blended, so the frame is defined for every `u` and
    /// never depends on how finely the segment gets tessellated.
    pub fn frame(&self, u: f32) -> MovingFrame {
        let (position, _, tangent) = self.evaluate(u);

        let carry = |normal: [f32; 2], from: Vec3| {
            Quat::from_rotation_arc(from, tangent) * decode_normal(normal, from)
        };
        let start = carry(self.rmf_start_normal, self.evaluate(0.0).2);
        let end = carry(self.rmf_end_normal, self.evaluate(1.0).2);
        let blended = start.lerp(end, u);

        let normal = (blended - tangent * blended.dot(tangent))
            .try_normalize()
            .unwrap_or_else(|| reference_basis(tangent).0);
        let binormal = tangent.cross(normal).normalize_or_zero();

        MovingFrame {
            position,
            tangent,
            normal,
            binormal,
        }
    }
}

/// The knots of a curve which starts at its first and ends at its last
/// point, with every segment in between covering the same parameter range.
pub fn clamped_uniform_knots(num_points: usize) -> Vec<f32> {
    let num_segments = num_points.saturating_sub(3).max(1);
    (0..num_points + 4)
        .map(|i| (i.saturating_sub(3).min(num_segments)) as f32 / num_segments as f32)
        .collect()
}

/// Turns the control points (with their weight in `w`) and knots of a cubic
/// NURBS into its segments. Returns nothing if they do not describe a curve.
pub fn extract_segments(points: &[Vec4], knots: &[f32]) -> Vec<CubicNurbsSegmentCache> {
    if points.len() < 4 || knots.len() != points.len() + 4 {
        return Vec::new();
    }

    let pieces: Vec<_> = spans(points.len(), knots)
        .map(|span| NurbsPiece::new(points, knots, span))
        .collect();
    join(&pieces)
}

/// What a segment is on its own, before it is joined with the segments
/// of its curve: its shape, its length, its tangents at both ends, and how
/// the rotation minimizing frame turns from its start to its end.
///
/// Every piece is made without looking at any other, so all of them can be
/// made at once. The GPU makes them in a pass of their own.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, PartialEq)]
pub struct NurbsPiece {
    monomial_basis: [[f32; 4]; 4],
    start_tangent: [f32; 3],
    length: f32,
    end_tangent: [f32; 3],
    _padding: f32,
    /// The columns of the matrix which carries a normal at the start to
    /// the one at the end: the double reflections of every step, one after
    /// the other.
    transport: [[f32; 4]; 3],
}

impl DataType for NurbsPiece {
    type ConfigurationOptions = ();

    const NAME: &'static str = "nurbs_piece";
    const WGSL: &'static str = "NurbsPiece";
    const SCALAR: Scalar = Scalar::F32;
    const LAYOUT: Layout = Layout::Struct(&[
        Field::new("monomial_basis", FieldType::Mat4),
        Field::new("start_tangent", FieldType::Vec3),
        Field::new("length", FieldType::F32),
        Field::new("end_tangent", FieldType::Vec3),
        Field::pad(1),
        Field::new("transport_x", FieldType::Vec3),
        Field::pad(1),
        Field::new("transport_y", FieldType::Vec3),
        Field::pad(1),
        Field::new("transport_z", FieldType::Vec3),
        Field::pad(1),
    ]);
}

impl NurbsPiece {
    /// The piece of the span which starts at knot `span`.
    pub fn new(points: &[Vec4], knots: &[f32], span: usize) -> Self {
        // Everything is measured on the segment relative to its origin,
        // where differences of small numbers keep their precision, and only
        // the shape is moved back to where it is.
        let (local, origin) = local_span_segment(points, knots, span);
        let segment = local.translated(origin);
        let at = |step: u32| local.evaluate(step as f32 / RMF_STEPS as f32);
        let mut transport = Mat3::IDENTITY;
        let (mut position, _, mut tangent) = at(0);
        for step in 1..=RMF_STEPS {
            let (next_position, _, next_tangent) = at(step);
            transport = double_reflection(position, tangent, next_position, next_tangent) * transport;
            (position, tangent) = (next_position, next_tangent);
        }
        let column = |v: Vec3| v.extend(0.0).to_array();
        Self {
            monomial_basis: segment.monomial_basis,
            start_tangent: local.evaluate(0.0).2.to_array(),
            length: local.length_until(1.0),
            end_tangent: tangent.to_array(),
            _padding: 0.0,
            transport: [
                column(transport.x_axis),
                column(transport.y_axis),
                column(transport.z_axis),
            ],
        }
    }

    fn transport(&self) -> Mat3 {
        let column = |c: [f32; 4]| Vec4::from_array(c).truncate();
        Mat3::from_cols(
            column(self.transport[0]),
            column(self.transport[1]),
            column(self.transport[2]),
        )
    }
}

/// The segments of a curve out of its pieces, in order: how long the curve
/// is up to the end of each, and the rotation minimizing frame at both of
/// its ends. The frame starts out from the reference of the first tangent,
/// turns with the tangent where segments meet at a corner, and is carried
/// across each segment by its transport.
pub fn join(pieces: &[NurbsPiece]) -> Vec<CubicNurbsSegmentCache> {
    let Some(first) = pieces.first() else {
        return Vec::new();
    };
    let mut tangent = Vec3::from_array(first.start_tangent);
    let mut normal = reference_basis(tangent).0;
    let mut total = 0.0;
    pieces
        .iter()
        .map(|piece| {
            let start_tangent = Vec3::from_array(piece.start_tangent);
            normal = turn(tangent, start_tangent, normal);
            let start_normal = encode_normal(normal, start_tangent);
            normal = piece.transport() * normal;
            tangent = Vec3::from_array(piece.end_tangent);
            total += piece.length;
            CubicNurbsSegmentCache {
                monomial_basis: piece.monomial_basis,
                length: piece.length,
                cumulative_length: total,
                rmf_start_normal: start_normal,
                rmf_end_normal: encode_normal(normal, tangent),
                _padding: [0.0; 2],
            }
        })
        .collect()
}

/// `normal` turned from where the tangent was `from` to where it is `to`,
/// and made a unit normal of `to` again. That keeps the error of carrying
/// it from adding up over many segments.
fn turn(from: Vec3, to: Vec3, normal: Vec3) -> Vec3 {
    let turned = Quat::from_rotation_arc(from, to) * normal;
    (turned - to * turned.dot(to))
        .try_normalize()
        .unwrap_or_else(|| reference_basis(to).0)
}

/// One step of the double reflection method as a matrix: carries a normal
/// from one sample of the curve to the next without twisting it around the
/// tangent. The same as on the GPU.
fn double_reflection(position: Vec3, tangent: Vec3, next_position: Vec3, next_tangent: Vec3) -> Mat3 {
    let v1 = next_position - position;
    let c1 = v1.length_squared();
    if c1 <= 1e-8 {
        return Mat3::IDENTITY;
    }
    let reflect = |v: Vec3, c: f32| {
        let k = 2.0 / c;
        Mat3::from_cols(
            Vec3::X - (k * v.x) * v,
            Vec3::Y - (k * v.y) * v,
            Vec3::Z - (k * v.z) * v,
        )
    };
    let first = reflect(v1, c1);
    let v2 = next_tangent - first * tangent;
    let c2 = v2.length_squared();
    if c2 <= 1e-8 {
        return first;
    }
    reflect(v2, c2) * first
}

/// The knot spans of a curve of `points` with these knots which are no
/// shorter than nothing: one segment of the curve each. A span is known by
/// the knot it starts at.
pub fn spans(points: usize, knots: &[f32]) -> impl Iterator<Item = usize> + '_ {
    (3..points).filter(move |span| knots[*span] < knots[span + 1])
}


/// Where a span is computed relative to: its first control point. A curve
/// can be far from the origin while its segments are small, and then the
/// numbers of a segment lose their precision in what all of them share.
fn span_origin(points: &[Vec4], span: usize) -> Vec3 {
    let first = points[span - 3];
    match first.w != 0.0 {
        true => first.xyz() / first.w,
        false => Vec3::ZERO,
    }
}

/// The segment of the span which starts at knot `span`, moved by minus
/// its origin, and that origin. Moved back by it, it is the segment.
fn local_span_segment(points: &[Vec4], knots: &[f32], span: usize) -> (CubicNurbsSegmentCache, Vec3) {
    let origin = span_origin(points, span);
    let (a, b) = (knots[span], knots[span + 1]);
    let bezier = [[a, a, a], [a, a, b], [a, b, b], [b, b, b]]
        .map(|at| blossom(points, knots, span, at, origin));
    let bezier_basis = Mat4::from_cols(
        Vec4::new(-1.0, 3.0, -3.0, 1.0),
        Vec4::new(3.0, -6.0, 3.0, 0.0),
        Vec4::new(-3.0, 3.0, 0.0, 0.0),
        Vec4::new(1.0, 0.0, 0.0, 0.0),
    );
    let points = Mat4::from_cols(bezier[0], bezier[1], bezier[2], bezier[3]);
    (CubicNurbsSegmentCache::from_basis(points * bezier_basis), origin)
}

/// The blossom of the span at the three parameters `at`: de Boor's
/// algorithm with another parameter on every level. With the ends of the
/// span as parameters it gives the Bézier points of the segment.
fn blossom(points: &[Vec4], knots: &[f32], span: usize, at: [f32; 3], origin: Vec3) -> Vec4 {
    // The points are homogeneous, so moving them scales with the weight.
    let moved = |point: Vec4| point - (origin * point.w).extend(0.0);
    let mut d = [
        moved(points[span - 3]),
        moved(points[span - 2]),
        moved(points[span - 1]),
        moved(points[span]),
    ];
    for level in 1..=3 {
        for j in (level..=3).rev() {
            let i = span - 3 + j;
            let width = knots[i + 4 - level] - knots[i];
            let alpha = if width > 0.0 { (at[level - 1] - knots[i]) / width } else { 0.0 };
            d[j] = d[j - 1] + (d[j] - d[j - 1]) * alpha;
        }
    }
    d[3]
}



/// Splits `t` in `0..=1` into the index of a segment and the `u` inside it.
/// Every segment covers the same share of `t`.
pub fn locate(num_segments: usize, t: f32) -> (usize, f32) {
    let scaled = t.clamp(0.0, 1.0) * num_segments as f32;
    let index = (scaled as usize).min(num_segments.saturating_sub(1));
    (index, scaled - index as f32)
}

/// A Cubic Nurbs curve that can be evaluated extremly efficiently on the CPU and GPU.
#[derive(Debug, Clone)]
pub struct CubicNurbs {
    /// Includes the weight of the point in the w coordinate.
    points: Vec<Vec4>,
    knots: Vec<f32>,
    segments: Vec<CubicNurbsSegmentCache>,
}

impl CubicNurbs {
    pub fn new(points: Vec<Vec4>, knots: Vec<f32>) -> Self {
        assert!(points.len() >= 4, "A cubic curve needs at least 4 points");
        assert_eq!(
            knots.len(),
            points.len() + 4,
            "Knots length must be equal to num_points + degree + 1"
        );

        let segments = extract_segments(&points, &knots);
        Self {
            points,
            knots,
            segments,
        }
    }

    pub fn segments(&self) -> &[CubicNurbsSegmentCache] {
        &self.segments
    }

    pub fn length(&self) -> f32 {
        self.segments
            .last()
            .map_or(0.0, |seg| seg.cumulative_length)
    }

    pub fn evaluate(&self, t: f32) -> Vec3 {
        let (index, u) = locate(self.segments.len(), t);
        self.segments[index].evaluate(u).0
    }

    pub fn frame(&self, t: f32) -> MovingFrame {
        let (index, u) = locate(self.segments.len(), t);
        self.segments[index].frame(u)
    }

    /// The `t` at which the curve is `distance` long.
    pub fn t_at_distance(&self, distance: f32) -> f32 {
        let distance = distance.clamp(0.0, self.length());
        let index = self
            .segments
            .partition_point(|seg| distance >= seg.cumulative_length)
            .min(self.segments.len() - 1);
        let segment = &self.segments[index];

        let local_distance = distance - (segment.cumulative_length - segment.length);
        (index as f32 + segment.u_at_distance(local_distance)) / self.segments.len() as f32
    }

    /// `count` frames with the same distance along the curve between them.
    pub fn sample_equidistant(&self, count: usize) -> Vec<MovingFrame> {
        let step = self.length() / count.saturating_sub(1).max(1) as f32;
        (0..count)
            .map(|i| self.frame(self.t_at_distance(i as f32 * step)))
            .collect()
    }

    //WIP
    fn to_gpu_matrices(&self) -> Vec<CubicNurbsSegmentCache> {
        let knots = &self.knots;
        let points = &self.points;

        let final_points_len = 3 * points.len() - 8;
        let final_knots_len = final_points_len + 4;

        let mut final_points = Vec::with_capacity(final_points_len);
        let mut final_knots = Vec::with_capacity(final_knots_len);

        final_knots.extend_from_slice(&knots[0..3]);
        final_points.push(points[0]);

        let mut point_cursor = 1;

        for i in 3..knots.len() - 5 {
            let k_0 = knots[i];
            let k_1 = knots[i + 1];
            let k_2 = knots[i + 2];

            let insertions = if k_0 == k_1 {
                if k_1 == k_2 { 0 } else { 1 }
            } else {
                2
            };

            for _ in 0..=insertions {
                final_knots.push(k_0);
            }

            match insertions {
                2 => {
                    let alpha1 = (k_0 - knots[i - 1]) / (knots[i + 2] - knots[i - 1]);
                    let alpha2 = (k_0 - knots[i]) / (knots[i + 3] - knots[i]);

                    let p_left = points[point_cursor - 1].lerp(points[point_cursor], alpha1);
                    let p_right = points[point_cursor].lerp(points[point_cursor + 1], alpha2);

                    final_points.push(p_left);
                    final_points.push(p_left.lerp(p_right, alpha2));
                }
                1 => {
                    let alpha = (k_0 - knots[i]) / (knots[i + 3] - knots[i]);
                    let p_interp = points[point_cursor].lerp(points[point_cursor + 1], alpha);
                    final_points.push(p_interp);
                }
                _ => {}
            }

            if point_cursor < points.len() {
                final_points.push(points[point_cursor]);
                point_cursor += 1;
            }
        }

        if let Some(&last_knot) = knots.last() {
            while final_knots.len() < final_knots_len {
                final_knots.push(last_knot);
            }
        }
        if let Some(&last_point) = points.last() {
            while final_points.len() < final_points_len {
                final_points.push(last_point);
            }
        }

        println!("{}, {final_points:?}", final_points.len());
        println!("{}, {final_knots:?}", final_knots.len());

        let bezier_basis = Mat4::from_cols(
            Vec4::new(-1.0, 3.0, -3.0, 1.0),
            Vec4::new(3.0, -6.0, 3.0, 0.0),
            Vec4::new(-3.0, 3.0, 0.0, 0.0),
            Vec4::new(1.0, 0.0, 0.0, 0.0),
        );

        let num_segments = (final_points.len() - 1) / 3;
        let mut gpu_matrices = Vec::with_capacity(num_segments);

        for s in 0..num_segments {
            let offset = s * 3;
            let p_matrix = Mat4::from_cols(
                final_points[offset],
                final_points[offset + 1],
                final_points[offset + 2],
                final_points[offset + 3],
            );

            gpu_matrices.push(CubicNurbsSegmentCache::from_basis(p_matrix * bezier_basis));
        }

        gpu_matrices
    }

    /* pub fn compute_rmf_frames(
        &self,
        count: usize,
        initial_normal: Option<Vec3>,
    ) -> Vec<MovingFrame> {
        if count < 2 {
            return Vec::new();
        }

        // 1. Get equidistant spatial distributions and clean tangent vectors
        let samples = self.sample_equidistant(count);
        let mut frames = Vec::with_capacity(count);

        // 2. Initialize the first frame
        let (p0, t0) = samples[0];
        let tangent0 = t0.normalize();

        // Establish an initial orthogonal baseline vector for the normal
        let normal0 = match initial_normal {
            Some(n) if n.cross(tangent0).length_squared() > 1e-5 => {
                // Gram-Schmidt orthogonalization
                (n - tangent0 * n.dot(tangent0)).normalize()
            }
            _ => {
                // Fallback default vector selection away from the tangent axis
                let abs_t = tangent0.abs();
                let ref_v = if abs_t.x < abs_t.y && abs_t.x < abs_t.z {
                    Vec3::X
                } else if abs_t.y < abs_t.z {
                    Vec3::Y
                } else {
                    Vec3::Z
                };
                ref_v.cross(tangent0).normalize()
            }
        };
        let binormal0 = tangent0.cross(normal0).normalize();

        frames.push(MovingFrame {
            position: p0,
            tangent: tangent0,
            normal: normal0,
            binormal: binormal0,
        });

        // 3. Propagate frames forward along the path using Double Reflection
        for i in 0..(count - 1) {
            let f_curr = &frames[i];
            let (p_next, t_next_raw) = samples[i + 1];
            let t_next = t_next_raw.normalize();

            let v1 = p_next - f_curr.position;
            let c1 = v1.length_squared();

            if c1 < 1e-8 {
                // Degenerate/Duplicate step; duplicate prior frame orientations
                frames.push(MovingFrame {
                    position: p_next,
                    tangent: t_next,
                    normal: f_curr.normal,
                    binormal: f_curr.binormal,
                });
                continue;
            }

            // First Reflection: maps f_curr.tangent to mirror space across v1
            let n_curr_reflected = f_curr.normal - (2.0 / c1) * v1.dot(f_curr.normal) * v1;
            let t_curr_reflected = f_curr.tangent - (2.0 / c1) * v1.dot(f_curr.tangent) * v1;

            // Second Reflection: maps mirrored frame onto t_next
            let v2 = t_next - t_curr_reflected;
            let c2 = v2.length_squared();

            let normal_next = if c2 > 1e-8 {
                n_curr_reflected - (2.0 / c2) * v2.dot(n_curr_reflected) * v2
            } else {
                n_curr_reflected
            };

            let binormal_next = t_next.cross(normal_next).normalize();
            let final_normal = binormal_next.cross(t_next).normalize(); // clean up numerical drift

            frames.push(MovingFrame {
                position: p_next,
                tangent: t_next,
                normal: final_normal,
                binormal: binormal_next,
            });
        }

        frames
    } */

    /* pub fn sweep_profile(
        &self,
        profile_vertices: &[Vec2], // Defined local X, Y (Z assumed 0)
        subdivisions: usize,
        is_closed_profile: bool,
    ) -> (Vec<Vec3>, Vec<u32>) {
        let frames = self.compute_rmf_frames(subdivisions, None);
        if frames.is_empty() || profile_vertices.is_empty() {
            return (Vec::new(), Vec::new());
        }

        let num_profile_pts = profile_vertices.len();
        let num_frames = frames.len();

        let mut out_vertices = Vec::with_capacity(num_frames * num_profile_pts);
        let mut out_indices = Vec::new();

        // 1. Generate transformed vertices for every Ring cross section
        for frame in &frames {
            for pt in profile_vertices {
                let local_3d = Vec3::new(pt.x, pt.y, 0.0);
                out_vertices.push(frame.transform_point(local_3d));
            }
        }

        // 2. Generate Triangles linking adjacent rings
        for ring in 0..(num_frames - 1) {
            let curr_ring_start = ring * num_profile_pts;
            let next_ring_start = (ring + 1) * num_profile_pts;

            let segments = if is_closed_profile {
                num_profile_pts
            } else {
                num_profile_pts - 1
            };

            for i in 0..segments {
                let i_next = (i + 1) % num_profile_pts;

                let v0 = (curr_ring_start + i) as u32;
                let v1 = (curr_ring_start + i_next) as u32;
                let v2 = (next_ring_start + i) as u32;
                let v3 = (next_ring_start + i_next) as u32;

                // Triangle 1
                out_indices.push(v0);
                out_indices.push(v1);
                out_indices.push(v2);

                // Triangle 2
                out_indices.push(v1);
                out_indices.push(v3);
                out_indices.push(v2);
            }
        }

        (out_vertices, out_indices)
    }

    pub fn sweep_profile_transformed<F>(
        &self,
        profile_vertices: &[Vec2],
        subdivisions: usize,
        is_closed_profile: bool,
        transform_fn: F,
    ) -> (Vec<Vec3>, Vec<u32>)
    where
        F: Fn(f32) -> TransformAtT,
    {
        // 1. Generate our underlying stable frame alignments
        let frames = self.compute_rmf_frames(subdivisions, None);
        if frames.is_empty() || profile_vertices.is_empty() {
            return (Vec::new(), Vec::new());
        }

        let num_profile_pts = profile_vertices.len();
        let num_frames = frames.len();

        let mut out_vertices = Vec::with_capacity(num_frames * num_profile_pts);
        let mut out_indices = Vec::new();

        // 2. Identify bounding time window domain to invert raw spacing to t coordinates
        let t_min = self.segments.first().unwrap().t_start;
        let t_max = self.segments.last().unwrap().t_end;
        let total_length = self.segments.last().unwrap().cumulative_length;
        let step_len = total_length / (subdivisions - 1) as f32;

        // 3. Process every ring frame along the curve
        for (idx, frame) in frames.iter().enumerate() {
            // Reconstruct the t parameter location of this specific frame index
            let target_s = idx as f32 * step_len;

            // Map the arc length `s` back to the approximate global time parameter `t`
            let mut t = t_min + (target_s / total_length) * (t_max - t_min);

            // Find the segment this specific slice falls within
            let current_seg_idx = self.find_segment_idx(t);
            let segment = &self.segments[current_seg_idx];

            let seg_start_len = if current_seg_idx == 0 {
                0.0
            } else {
                self.segments[current_seg_idx - 1].cumulative_length
            };
            let s_local = target_s - seg_start_len;

            // Refine t parameter estimation with Newton-Raphson iteration to match arc length
            for _ in 0..2 {
                let current_s_local = self.length_inside_segment(segment, t);
                let (_, tangent) = self.evaluate_tanget(t);
                let speed = tangent.length();

                if speed < 1e-5 {
                    break;
                }

                let delta_t = (current_s_local - s_local) / speed;
                t -= delta_t;
                t = t.clamp(segment.t_start, segment.t_end);

                if delta_t.abs() < 1e-5 {
                    break;
                }
            }

            // Fetch structural modifications at this specific parameter point
            let tx = transform_fn(t);

            // Compute an internal transformation matrix combining scaling and twist around the tangent
            let twist_quat = Quat::from_axis_angle(frame.tangent, tx.rotation_radians);
            let twist_matrix = Mat3::from_quat(twist_quat);

            // Construct and transform each profile point for this ring slice
            for pt in profile_vertices {
                // Apply 2D scale profile factor directly
                let scaled_local = Vec3::new(pt.x * tx.scale.x, pt.y * tx.scale.y, 0.0);

                // Align to the standard RMF orientation coordinate system
                let oriented_pt = (frame.normal * scaled_local.x)
                    + (frame.binormal * scaled_local.y)
                    + (frame.tangent * scaled_local.z);

                // Apply twist rotation around the path's core directional axis
                let transformed_pt = twist_matrix.mul_vec3(oriented_pt);

                // Translate out to global curve space positioning coordinates
                out_vertices.push(frame.position + transformed_pt);
            }
        }

        // 4. Thread structural indices to weave geometric topology mesh structures
        for ring in 0..(num_frames - 1) {
            let curr_ring_start = ring * num_profile_pts;
            let next_ring_start = (ring + 1) * num_profile_pts;

            let segments = if is_closed_profile {
                num_profile_pts
            } else {
                num_profile_pts - 1
            };

            for i in 0..segments {
                let i_next = (i + 1) % num_profile_pts;

                let v0 = (curr_ring_start + i) as u32;
                let v1 = (curr_ring_start + i_next) as u32;
                let v2 = (next_ring_start + i) as u32;
                let v3 = (next_ring_start + i_next) as u32;

                out_indices.push(v0);
                out_indices.push(v1);
                out_indices.push(v2);

                out_indices.push(v1);
                out_indices.push(v3);
                out_indices.push(v2);
            }
        }

        (out_vertices, out_indices)
    } */
}

/// A stable coordinate frame tracking a point along the NURBS spline.
#[derive(Debug, Clone, Copy)]
pub struct MovingFrame {
    pub position: Vec3,
    pub tangent: Vec3,
    pub normal: Vec3,
    pub binormal: Vec3,
}

impl MovingFrame {
    /// Transforms a local profile vertex (usually defined in the XY plane) into World Space.
    pub fn transform_point(&self, local_pt: Vec3) -> Vec3 {
        self.position
            + (self.normal * local_pt.x)
            + (self.binormal * local_pt.y)
            + (self.tangent * local_pt.z)
    }
}

/// Defines structural scales and twist angles mapping along parameter `t`
#[derive(Debug, Clone, Copy)]
pub struct TransformAtT {
    /// 2D Scaling factor for the profile's local (X, Y) coordinates.
    pub scale: Vec2,
    /// Twist angle rotation (in radians) around the curve's local tangent vector.
    pub rotation_radians: f32,
}


impl Curve<Vec3> for CubicNurbs {
    fn domain(&self) -> std::ops::Range<f32> {
        0.0..1.
    }

    fn sample_unchecked(&self, t: f32) -> Vec3 {
        self.evaluate(t)
    }

    fn sample(&self, t: f32) -> Vec3 {
        self.evaluate(t)
    }

    fn length(&self) -> f32 {
        CubicNurbs::length(self)
    }

    fn t_at_distance(&self, distance: f32) -> f32 {
        CubicNurbs::t_at_distance(self, distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points() -> Vec<Vec4> {
        vec![
            Vec4::new(0.0, 0.0, 0.0, 1.),
            Vec4::new(1.0, 2.0, 0.5, 1.),
            Vec4::new(2.0, -1.0, 1.0, 1.),
            Vec4::new(3.0, 3.0, 0.0, 1.),
            Vec4::new(4.0, 0.0, -1.0, 1.),
            Vec4::new(5.0, 2.0, 0.0, 1.),
            Vec4::new(6.0, 1.0, 2.0, 1.),
            Vec4::new(7.0, 4.0, 0.0, 1.),
        ]
    }

    fn curve() -> CubicNurbs {
        CubicNurbs::new(points(), clamped_uniform_knots(8))
    }

    /// The textbook definition of a B-Spline basis function.
    fn basis(i: usize, degree: usize, x: f32, knots: &[f32]) -> f32 {
        if degree == 0 {
            return (knots[i] <= x && x < knots[i + 1]) as u32 as f32;
        }
        let term = |numerator: f32, denominator: f32, basis: f32| {
            if denominator == 0.0 {
                0.0
            } else {
                numerator / denominator * basis
            }
        };
        term(
            x - knots[i],
            knots[i + degree] - knots[i],
            basis(i, degree - 1, x, knots),
        ) + term(
            knots[i + degree + 1] - x,
            knots[i + degree + 1] - knots[i + 1],
            basis(i + 1, degree - 1, x, knots),
        )
    }

    fn reference(points: &[Vec4], knots: &[f32], x: f32) -> Vec3 {
        let hom: Vec4 = points
            .iter()
            .enumerate()
            .map(|(i, point)| *point * basis(i, 3, x, knots))
            .sum();
        hom.xyz() / hom.w
    }

    #[test]
    fn knots_are_clamped_and_uniform() {
        assert_eq!(
            clamped_uniform_knots(6),
            [0., 0., 0., 0., 1. / 3., 2. / 3., 1., 1., 1., 1.]
        );
    }

    #[test]
    fn segments_match_the_basis_functions() {
        let knots = clamped_uniform_knots(8);
        let curve = curve();
        assert_eq!(curve.segments().len(), 5);

        for (index, segment) in curve.segments().iter().enumerate() {
            for step in 0..10 {
                let u = step as f32 / 10.0;
                let x = knots[3 + index] + u * (knots[4 + index] - knots[3 + index]);
                let expected = reference(&points(), &knots, x);
                let found = segment.evaluate(u).0;
                assert!(
                    found.distance(expected) < 1e-4,
                    "segment {index} at {u}: {found} != {expected}"
                );
            }
        }
    }

    #[test]
    fn weights_scale_out() {
        let heavy: Vec<_> = points().iter().map(|point| *point * 2.0).collect();
        let heavy = CubicNurbs::new(heavy, clamped_uniform_knots(8));
        let curve = curve();
        for step in 0..=20 {
            let t = step as f32 / 20.0;
            assert!(heavy.evaluate(t).distance(curve.evaluate(t)) < 1e-4);
        }
    }

    #[test]
    fn the_curve_starts_and_ends_at_its_outer_points() {
        let curve = curve();
        assert!(curve.evaluate(0.0).distance(Vec3::ZERO) < 1e-5);
        assert!(curve.evaluate(1.0).distance(Vec3::new(7.0, 4.0, 0.0)) < 1e-4);
    }

    #[test]
    fn lengths_of_a_straight_line() {
        let line = (0..6).map(|i| Vec4::new(i as f32 * 2.0, 0.0, 0.0, 1.0));
        let line = CubicNurbs::new(line.collect(), clamped_uniform_knots(6));

        assert!((line.length() - 10.0).abs() < 1e-3);
        let cumulative: Vec<_> = line
            .segments()
            .iter()
            .map(|segment| segment.cumulative_length())
            .collect();
        assert!(cumulative.is_sorted());

        let middle = line.evaluate(line.t_at_distance(5.0));
        assert!(middle.distance(Vec3::new(5.0, 0.0, 0.0)) < 1e-3);
    }

    #[test]
    fn equidistant_samples_are_equidistant_along_the_curve() {
        let curve = curve();
        let samples = curve.sample_equidistant(9);
        assert!(samples[0].position.distance(curve.evaluate(0.0)) < 1e-4);
        assert!(samples[8].position.distance(curve.evaluate(1.0)) < 1e-3);

        let step = curve.length() / 8.0;
        for i in 0..9 {
            let t = curve.t_at_distance(i as f32 * step);
            let (index, u) = locate(curve.segments().len(), t);
            let segment = &curve.segments()[index];
            let distance =
                segment.cumulative_length() - segment.length() + segment.length_until(u);
            assert!((distance - i as f32 * step).abs() < 1e-2 * curve.length());
        }
    }

    #[test]
    fn frames_are_orthonormal_and_continuous() {
        let curve = curve();
        for segment in curve.segments() {
            for step in 0..=16 {
                let frame = segment.frame(step as f32 / 16.0);
                assert!((frame.normal.length() - 1.0).abs() < 1e-4);
                assert!(frame.normal.dot(frame.tangent).abs() < 1e-4);
                assert!(frame.binormal.distance(frame.tangent.cross(frame.normal)) < 1e-4);
            }
        }
        for pair in curve.segments().windows(2) {
            let (end, start) = (pair[0].frame(1.0), pair[1].frame(0.0));
            assert!(end.position.distance(start.position) < 1e-4);
            assert!(end.normal.distance(start.normal) < 1e-3);
        }
    }

    #[test]
    fn interpolated_frames_stay_close_to_the_walked_ones() {
        let curve = curve();
        let mut worst: f32 = 0.0;
        for segment in curve.segments() {
            let (mut position, _, mut tangent) = segment.evaluate(0.0);
            let mut normal = segment.frame(0.0).normal;
            for step in 1..=RMF_STEPS {
                let u = step as f32 / RMF_STEPS as f32;
                let (next_position, _, next_tangent) = segment.evaluate(u);
                normal = double_reflection(position, tangent, next_position, next_tangent) * normal;
                (position, tangent) = (next_position, next_tangent);
                worst = worst.max(normal.angle_between(segment.frame(u).normal));
            }
        }
        assert!(worst.to_degrees() < 8.0, "{} degrees", worst.to_degrees());
    }

    #[test]
    fn a_flat_curve_keeps_its_normal_in_the_plane() {
        let flat: Vec<_> = points()
            .iter()
            .map(|point| Vec4::new(point.x, point.y, 0.0, 1.0))
            .collect();
        let flat = CubicNurbs::new(flat, clamped_uniform_knots(8));
        for step in 0..=50 {
            assert!(flat.frame(step as f32 / 50.0).normal.z.abs() < 1e-3);
        }
    }

    #[test]
    fn repeated_knots_make_a_corner() {
        // A knot of full multiplicity splits the curve into two Béziers.
        let knots = vec![0., 0., 0., 0., 0.5, 0.5, 0.5, 1., 1., 1., 1.];
        let corner = CubicNurbs::new(points()[..7].to_vec(), knots);
        assert_eq!(corner.segments().len(), 2);
        assert!(corner.evaluate(0.5).distance(points()[3].xyz()) < 1e-4);

        let (end, start) = (corner.segments()[0].frame(1.0), corner.segments()[1].frame(0.0));
        assert!(start.normal.dot(start.tangent).abs() < 1e-4);
        assert!(end.normal.dot(end.tangent).abs() < 1e-4);
    }

    #[test]
    fn a_moved_curve_is_the_same_curve_somewhere_else() {
        let offset = Vec3::new(1.0, -2.0, 3.0);
        let weighted: Vec<_> = points()
            .iter()
            .enumerate()
            .map(|(i, point)| *point * (1.0 + i as f32 * 0.3))
            .collect();
        let curve = CubicNurbs::new(weighted, clamped_uniform_knots(8));
        for segment in curve.segments() {
            let moved = segment.translated(offset);
            for step in 0..=8 {
                let u = step as f32 / 8.0;
                assert!((moved.evaluate(u).0 - segment.evaluate(u).0 - offset).length() < 1e-4);
                assert!(moved.frame(u).normal.distance(segment.frame(u).normal) < 1e-4);
            }
            assert_eq!(moved.length(), segment.length());
        }
    }

    #[test]
    fn nonsense_is_not_a_curve() {
        assert!(extract_segments(&points()[..3], &[0.0; 7]).is_empty());
        assert!(extract_segments(&points(), &[0.0; 5]).is_empty());
    }
}
