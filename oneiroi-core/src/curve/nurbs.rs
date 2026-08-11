use std::time::Instant;

use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec4, Vec4Swizzles};

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

/// GPU-Friendly structure accelerating the evaluation by:
/// - Caching the monomial basis via Bezier Extraction.
/// - Caching the start normal in two dimensional space.
/// - Caching the length of the preceeding and currect segment.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct CubicNurbsSegmentCache {
    monomial_basis: Mat4,

    length: f32,
    cumulative_length: f32,

    rmf_start_normal: Vec2,
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
        let instant = Instant::now();

        let num_points = points.len();

        assert_eq!(
            knots.len(),
            num_points + 4,
            "Knots length must be equal to num_points + degree + 1"
        );

        let mut curve = Self {
            points,
            knots,
            segments: Vec::new(),
        };

        curve.segments = curve.to_gpu_matrices_old();
        curve.precompute_segment_rmf_starts();

        println!("{:#?}", instant.elapsed());

        curve
    }

    pub fn segments(&self) -> &[CubicNurbsSegmentCache] {
        &self.segments
    }

    fn evaluate_monomial(&self, seg_idx: usize, u: f32) -> (Vec3, Vec3, Vec3) {
        let m = &self.segments[seg_idx].monomial_basis;
        let a = m.col(0); // t^3
        let b = m.col(1); // t^2
        let c = m.col(2); // t
        let d = m.col(3); // 1

        let pos_hom = a
            .mul_add(Vec4::splat(u), b)
            .mul_add(Vec4::splat(u), c)
            .mul_add(Vec4::splat(u), d);
        let pos = pos_hom.xyz() / pos_hom.w;

        let dp_du_hom = a
            .mul_add(Vec4::splat(u * 3.0), b * 2.0)
            .mul_add(Vec4::splat(u), c);
        let velocity = (dp_du_hom.xyz() - dp_du_hom.w * pos) / pos_hom.w;

        // Tangente mit robustem Fallback für verschwindende Ableitungen (z.B. geklemmte Ränder)
        let tangent = velocity.try_normalize().unwrap_or_else(|| {
            let u_eps = if u + 0.001 <= 1.0 {
                u + 0.001
            } else {
                u - 0.001
            };
            let dp_du_eps = a
                .mul_add(Vec4::splat(u_eps * 3.0), b * 2.0)
                .mul_add(Vec4::splat(u_eps), c);
            let pos_eps_hom = a
                .mul_add(Vec4::splat(u_eps), b)
                .mul_add(Vec4::splat(u_eps), c)
                .mul_add(Vec4::splat(u_eps), d);
            let pos_eps = pos_eps_hom.xyz() / pos_eps_hom.w;
            let vel_eps = (dp_du_eps.xyz() - dp_du_eps.w * pos_eps) / pos_eps_hom.w;
            vel_eps.normalize()
        });

        (pos, velocity, tangent)
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

            gpu_matrices.push(CubicNurbsSegmentCache {
                length: 0.,
                cumulative_length: 0.,
                monomial_basis: p_matrix * bezier_basis,
                rmf_start_normal: Vec2::ZERO,
            });
        }

        gpu_matrices
    }

    fn to_gpu_matrices_old(&self) -> Vec<CubicNurbsSegmentCache> {
        let p = 3;
        let mut w_knots = self.knots.clone();
        let mut w_points = self.points.clone();

        let mut i = w_knots.len() - p - 2;
        while i > p {
            let knot_val = w_knots[i];
            let mut count = 0;
            while w_knots[i - count] == knot_val {
                count += 1;
            }
            let start_idx = i - count + 1;
            let num_insertions = p - count;

            for _ in 0..num_insertions {
                let k = start_idx;
                w_knots.insert(k, knot_val);

                let mut new_points = Vec::with_capacity(w_points.len() + 1);
                new_points.extend_from_slice(&w_points[..k - p]);

                for j in (k - p)..k {
                    let alpha = (knot_val - w_knots[j]) / (w_knots[j + p + 1] - w_knots[j]);
                    let new_pt = w_points[j - 1].lerp(w_points[j], alpha);
                    new_points.push(new_pt);
                }

                new_points.extend_from_slice(&w_points[k - 1..]);
                w_points = new_points;
            }
            i -= count;
        }

        println!("{}, {w_points:?}", w_points.len());
        println!("{}, {w_knots:?}", w_knots.len());

        let bezier_basis = Mat4::from_cols(
            Vec4::new(-1.0, 3.0, -3.0, 1.0),
            Vec4::new(3.0, -6.0, 3.0, 0.0),
            Vec4::new(-3.0, 3.0, 0.0, 0.0),
            Vec4::new(1.0, 0.0, 0.0, 0.0),
        );

        let mut gpu_matrices = Vec::new();
        let num_segments = (w_points.len() - 1) / p;

        for s in 0..num_segments {
            let offset = s * p;
            let p_matrix = Mat4::from_cols(
                w_points[offset],
                w_points[offset + 1],
                w_points[offset + 2],
                w_points[offset + 3],
            );

            gpu_matrices.push(CubicNurbsSegmentCache {
                length: 0.,
                cumulative_length: 0.,
                monomial_basis: p_matrix * bezier_basis,
                rmf_start_normal: Vec2::ZERO,
            });
        }

        gpu_matrices
    }

    fn precompute_segment_rmf_starts(&mut self) {
        let num_segments = self.segments.len();
        if num_segments == 0 {
            return;
        }

        let (mut current_pos, _, mut current_tangent) = self.evaluate_monomial(0, 0.0);

        let abs_t = current_tangent.abs();
        let ref_v = if abs_t.x < abs_t.y && abs_t.x < abs_t.z {
            Vec3::X
        } else if abs_t.y < abs_t.z {
            Vec3::Y
        } else {
            Vec3::Z
        };
        let mut current_normal = ref_v.cross(current_tangent).normalize();

        self.segments[0].rmf_start_normal = Vec2::X;

        for idx in 0..num_segments {
            let (next_pos, _, next_tangent) = self.evaluate_monomial(idx, 1.0);

            let v1 = next_pos - current_pos;
            let c1 = v1.length_squared();

            if c1 > 1e-8 {
                let n_curr_reflected = current_normal - (2.0 / c1) * v1.dot(current_normal) * v1;
                let t_curr_reflected = current_tangent - (2.0 / c1) * v1.dot(current_tangent) * v1;

                let v2 = next_tangent - t_curr_reflected;
                let c2 = v2.length_squared();

                if c2 > 1e-8 {
                    current_normal = n_curr_reflected - (2.0 / c2) * v2.dot(n_curr_reflected) * v2;
                } else {
                    current_normal = n_curr_reflected;
                }

                current_normal = next_tangent
                    .cross(current_normal)
                    .normalize()
                    .cross(next_tangent)
                    .normalize();
            }

            current_pos = next_pos;
            current_tangent = next_tangent;

            if idx + 1 < num_segments {
                let next_abs_t = current_tangent.abs();
                let next_ref_v = if next_abs_t.x < next_abs_t.y && next_abs_t.x < next_abs_t.z {
                    Vec3::X
                } else if next_abs_t.y < next_abs_t.z {
                    Vec3::Y
                } else {
                    Vec3::Z
                };

                let n_ref = next_ref_v.cross(current_tangent).normalize();
                let b_ref = current_tangent.cross(n_ref).normalize();

                self.segments[idx + 1].rmf_start_normal =
                    Vec2::new(current_normal.dot(n_ref), current_normal.dot(b_ref));
            }
        }
    }

    /* fn recompute_lengths(&mut self) {
        let num_segments = self.segments.len();
        let mut total_length = 0.0;

        for idx in 0..num_segments {
            let segment = &self.segments[idx];

            let seg_len = self.length_inside_segment(segment, segment.t_end);

            total_length += seg_len;

            let segment_mut = &mut self.segments[idx];
            segment_mut.length = seg_len;
            segment_mut.cumulative_length = total_length;
        }
    }

    fn find_segment_idx(&self, t: f32) -> usize {
        if self.segments.is_empty() {
            return 0;
        }

        if t <= self.segments[0].t_start {
            return 0;
        }
        if t >= self.segments.last().unwrap().t_end {
            return self.segments.len() - 1;
        }

        self.segments.partition_point(|seg| t >= seg.t_start) - 1
    }

    pub fn evaluate(&self, t: f32) -> Vec3 {
        let segment_idx = self.find_segment_idx(t);
        let segment = &self.segments[segment_idx];

        let u = (t - segment.t_start) / (segment.t_end - segment.t_start);

        let u_splat = Vec4::splat(u);
        let c0 = segment.monomial_basis.col(0);
        let c1 = segment.monomial_basis.col(1);
        let c2 = segment.monomial_basis.col(2);
        let c3 = segment.monomial_basis.col(3);

        let horner_eval = c3
            .mul_add(u_splat, c2)
            .mul_add(u_splat, c1)
            .mul_add(u_splat, c0);

        Vec3::new(
            horner_eval.x / horner_eval.w,
            horner_eval.y / horner_eval.w,
            horner_eval.z / horner_eval.w,
        )
    }

    pub fn evaluate_derivatives(&self, t: f32) -> (Vec3, Vec3, Vec3) {
        let segment_idx = self.find_segment_idx(t);
        let segment = &self.segments[segment_idx];

        let dt = segment.t_end - segment.t_start;
        let u = (t - segment.t_start) / dt;
        let u_splat = Vec4::splat(u);

        let c0 = segment.monomial_basis.col(0);
        let c1 = segment.monomial_basis.col(1);
        let c2 = segment.monomial_basis.col(2);
        let c3 = segment.monomial_basis.col(3);

        let horner_eval = c3
            .mul_add(u_splat, c2)
            .mul_add(u_splat, c1)
            .mul_add(u_splat, c0);

        let d3 = c3 * 3.0;
        let d2 = c2 * 2.0;
        let dp_du = d3.mul_add(u_splat, d2).mul_add(u_splat, c1);

        let d6 = c3 * 6.0;
        let d2p_du2 = d6.mul_add(u_splat, d2);

        let inv_dt = 1.0 / dt;
        let inv_dt2 = inv_dt * inv_dt;

        let a_xyz = horner_eval.xyz();
        let w = horner_eval.w;

        let da = dp_du.xyz() * inv_dt;
        let dw = dp_du.w * inv_dt;

        let d2a = d2p_du2.xyz() * inv_dt2;
        let d2w = d2p_du2.w * inv_dt2;

        let c_pos = a_xyz / w;
        let c_vel = (da - dw * c_pos) / w;
        let c_acc = (d2a - 2.0 * dw * c_vel - d2w * c_pos) / w;

        (c_pos, c_vel, c_acc)
    }

    pub fn evaluate_tanget(&self, t: f32) -> (Vec3, Vec3) {
        let segment_idx = self.find_segment_idx(t);
        let segment = &self.segments[segment_idx];

        let dt = segment.t_end - segment.t_start;
        let u = (t - segment.t_start) / dt;

        let u_splat = Vec4::splat(u);
        let c0 = segment.monomial_basis.col(0);
        let c1 = segment.monomial_basis.col(1);
        let c2 = segment.monomial_basis.col(2);
        let c3 = segment.monomial_basis.col(3);

        let horner_eval = c3
            .mul_add(u_splat, c2)
            .mul_add(u_splat, c1)
            .mul_add(u_splat, c0);

        let d3 = c3 * 3.0;
        let d2 = c2 * 2.0;
        let dp_du = d3.mul_add(u_splat, d2).mul_add(u_splat, c1);

        let inv_dt = 1.0 / dt;
        let a_xyz = horner_eval.xyz();
        let w = horner_eval.w;

        let da = dp_du.xyz() * inv_dt;
        let dw = dp_du.w * inv_dt;

        let c_pos = a_xyz / w;
        let c_vel = (da - dw * c_pos) / w;

        (c_pos, c_vel)
    }

    pub fn curvature(&self, t: f32) -> f32 {
        let (_, tangent, second_deriv) = self.evaluate_derivatives(t);
        let numerator = tangent.cross(second_deriv).length();
        let denominator = tangent.length().powi(3);

        if denominator.abs() < 1e-6 {
            0.0
        } else {
            numerator / denominator
        }
    }

    fn length_inside_segment(&self, segment: &CubicNurbsSegmentCache, t_cutoff: f32) -> f32 {
        let dt = t_cutoff - segment.t_start;
        if dt <= 1e-6 {
            return 0.0;
        }

        let mut span_integral = 0.0;
        for i in 0..5 {
            let t = 0.5 * (dt * GAUSS_NODES[i] + (t_cutoff + segment.t_start));
            let (_, tangent) = self.evaluate_tanget(t);
            span_integral += GAUSS_WEIGHTS[i] * tangent.length();
        }
        span_integral * 0.5 * dt
    }

    pub fn length(&self) -> f32 {
        self.segments
            .last()
            .map_or(0.0, |seg| seg.cumulative_length)
    }

    pub fn sample_equidistant(&self, count: usize) -> Vec<(Vec3, Vec3)> {
        if count == 0 {
            return Vec::new();
        }
        if count == 1 {
            return vec![self.evaluate_tanget(self.segments[0].t_start)];
        }

        let total_length = self.segments.last().unwrap().cumulative_length;
        let step = total_length / (count - 1) as f32;
        let mut points = Vec::with_capacity(count);

        points.push(self.evaluate_tanget(self.segments[0].t_start));

        for i in 1..(count - 1) {
            let target_s = i as f32 * step;

            let t = self.t_at_distance(target_s).unwrap();

            points.push(self.evaluate_tanget(t));
        }

        points.push(self.evaluate_tanget(self.segments.last().unwrap().t_end));
        points
    }

    fn t_at_distance(&self, distance: f32) -> Option<f32> {
        if self.segments.is_empty() {
            return None;
        }

        let current_seg_idx = self
            .segments
            .partition_point(|seg| distance >= seg.cumulative_length);
        let segment = &self.segments[current_seg_idx];
        let seg_start_len = if current_seg_idx == 0 {
            0.0
        } else {
            self.segments[current_seg_idx - 1].cumulative_length
        };
        let local_distance = distance - seg_start_len;

        let mut t =
            segment.t_start + (local_distance / segment.length) * (segment.t_end - segment.t_start);

        // Because our start t should be extremely accurate we only do one loop for now.
        for _ in 0..2 {
            let current_local_distance = self.length_inside_segment(segment, t);
            let (_, tangent) = self.evaluate_tanget(t);
            let speed = tangent.length();

            if speed < 1e-5 {
                break;
            }

            let delta_t = (current_local_distance - local_distance) / speed;
            t -= delta_t;
            t = t.clamp(segment.t_start, segment.t_end);

            if delta_t.abs() < 1e-5 {
                break;
            }
        }
        Some(t)
    } */

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
        todo!()
    }

    fn sample(&self, t: f32) -> Vec3 {
        //self.evaluate(t)
        panic!()
    }

    fn length(&self) -> f32 {
        self.length()
    }

    fn t_at_distance(&self, distance: f32) -> f32 {
        //self.t_at_distance(distance).unwrap()
        panic!()
    }
}
