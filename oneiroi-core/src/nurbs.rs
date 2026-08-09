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
/// - Caching the coefficient Matrix for the Segment.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CubicNurbsSegmentCache {
    monomial_basis: Mat4,

    length: f32,
    cumulative_length: f32,

    rmf_start_normal: Vec2,
}

/// A Cubic Nurbs curve that can be evaluated extremly efficiently on the CPU and GPU.
/// To achieve this it uses the Marsden Identity.
pub struct CubicNurbs {
    /// Includes the weight of the point in the w coordinate.
    points: Vec<Vec4>,
    knots: Vec<f32>,
    pub segments: Box<[CubicNurbsSegmentCache]>,
}

impl CubicNurbs {
    pub fn new(points: Vec<Vec4>, knots: Vec<f32>) -> Self {
        let num_points = points.len();

        assert_eq!(
            knots.len(),
            num_points + 4,
            "Knots length must be equal to num_points + degree + 1"
        );

        let mut curve = Self {
            points,
            knots,
            segments: Box::new([]), //segments_cache.into_boxed_slice(),
        };

        curve.segments = curve.to_gpu_matrices();

        //curve.recompute_lengths();
        curve.precompute_segment_rmf_starts();

        println!("{:#?}", curve.segments);

        curve
    }

    pub fn to_gpu_matrices(&self) -> Box<[CubicNurbsSegmentCache]> {
        let p = 3; // Cubic degree
        let mut w_knots = self.knots.clone();
        let mut w_points = self.points.clone();

        // 1. Standard Boehm's Knot Insertion to isolate Bezier control points
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

        // Constant Cubic Bezier Basis Matrix (Transposed for glam column-major order alignment)
        // Row 0 coefficient for t^3, Row 1 for t^2, Row 2 for t, Row 3 for 1
        let bezier_basis = Mat4::from_cols(
            Vec4::new(-1.0, 3.0, -3.0, 1.0), // Coeffs for P0
            Vec4::new(3.0, -6.0, 3.0, 0.0),  // Coeffs for P1
            Vec4::new(-3.0, 3.0, 0.0, 0.0),  // Coeffs for P2
            Vec4::new(1.0, 0.0, 0.0, 0.0),   // Coeffs for P3
        );

        let mut gpu_matrices = Vec::new();
        let num_segments = (w_points.len() - 1) / p;

        // 2. Combine Basis Matrix with Control Points into a single Coefficient Matrix
        for s in 0..num_segments {
            let offset = s * p;

            // Build a geometry matrix where columns are the 4 control points
            let p_matrix = Mat4::from_cols(
                w_points[offset],
                w_points[offset + 1],
                w_points[offset + 2],
                w_points[offset + 3],
            );

            // Multiply geometry by the basis.
            // In glam, p_matrix * bezier_basis creates a coefficient matrix where:
            // Column 0 = A, Column 1 = B, Column 2 = C, Column 3 = D
            // for the equation: P(t) = A*t^3 + B*t^2 + C*t + D
            let coeff_matrix = p_matrix * bezier_basis;
            gpu_matrices.push(CubicNurbsSegmentCache {
                length: 0.,
                cumulative_length: 0.,
                monomial_basis: coeff_matrix,
                rmf_start_normal: Vec2::ZERO,
            });
        }

        gpu_matrices.into_boxed_slice()
    }

    fn precompute_segment_rmf_starts(&mut self) {
        let num_segments = self.segments.len();
        if num_segments == 0 {
            return;
        }

        // --- 1. INITIALISIERUNG AM ABSOLUTEN ANFANG (Segment 0, u = 0.0) ---
        let segment_0 = &self.segments[0];

        // Bei u = 0.0 entspricht die Position direkt dem konstanten Vektor D (Spalte 3)
        let pos_0_hom = segment_0.monomial_basis.col(3);
        let mut current_pos = pos_0_hom.xyz() / pos_0_hom.w;

        // Bei u = 0.0 entspricht die Ableitung dP/du exakt dem Vektor C (Spalte 2)
        let dp_du_0 = segment_0.monomial_basis.col(2);
        let current_velocity = (dp_du_0.xyz() - dp_du_0.w * current_pos) / pos_0_hom.w;

        // Bestimme die Tangente (mit Fallback für geklemmte Ränder)
        let mut current_tangent = current_velocity.try_normalize().unwrap_or_else(|| {
            // Fallback: Ein winziges Stück (u = 0.001) ins Segment hineingehen
            let u_eps = 0.001;
            let u_splat = Vec4::splat(u_eps);
            let c0 = segment_0.monomial_basis.col(0); // A
            let c1 = segment_0.monomial_basis.col(1); // B
            let c2 = segment_0.monomial_basis.col(2); // C

            let dp_du_eps = c0.mul_add(u_splat * 3.0, c1 * 2.0).mul_add(u_splat, c2);
            dp_du_eps.xyz().normalize()
        });

        // Generiere die allererste stabile 3D-Startnormale (Gram-Schmidt)
        let abs_t = current_tangent.abs();
        let ref_v = if abs_t.x < abs_t.y && abs_t.x < abs_t.z {
            Vec3::X
        } else if abs_t.y < abs_t.z {
            Vec3::Y
        } else {
            Vec3::Z
        };
        let mut current_normal = ref_v.cross(current_tangent).normalize();

        // Jedes Segment braucht ein eigenes Referenzsystem für die Kompression.
        // Für Segment 0 ist die Start-Tangente exakt 'current_tangent'.
        let n_ref_0 = ref_v.cross(current_tangent).normalize();
        let b_ref_0 = current_tangent.cross(n_ref_0).normalize();

        // Da 'current_normal' hier identisch mit 'n_ref_0' generiert wurde,
        // ist die Projektion mathematisch exakt Vec2(1.0, 0.0)
        self.segments[0].rmf_start_normal =
            Vec2::new(current_normal.dot(n_ref_0), current_normal.dot(b_ref_0));

        // --- 2. PROPAGATION-LOOP (u = 1.0) ÜBER ALLE INTERVALLE ---
        for idx in 0..num_segments {
            let seg = &self.segments[idx];

            // Evaluiere das Ende des aktuellen Segments (u = 1.0)
            // Position = A + B + C + D
            let pos_1_hom = seg.monomial_basis.col(0)
                + seg.monomial_basis.col(1)
                + seg.monomial_basis.col(2)
                + seg.monomial_basis.col(3);
            let next_pos = pos_1_hom.xyz() / pos_1_hom.w;

            // Ableitung am Ende (u = 1.0) -> dP/du = 3A + 2B + C
            let dp_du_1 = seg.monomial_basis.col(0) * 3.0
                + seg.monomial_basis.col(1) * 2.0
                + seg.monomial_basis.col(2);
            let next_velocity = (dp_du_1.xyz() - dp_du_1.w * next_pos) / pos_1_hom.w;
            let next_tangent = next_velocity.normalize();

            // Wang's Doppel-Reflexion (Double Reflection Method)
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

                // Drift bereinigen
                current_normal = next_tangent
                    .cross(current_normal)
                    .normalize()
                    .cross(next_tangent)
                    .normalize();
            }

            // Werte für den nächsten Schritt sichern
            current_pos = next_pos;
            current_tangent = next_tangent;

            // --- 3. KOMPRESSION FÜR DAS NÄCHSTE SEGMENT ---
            if idx + 1 < num_segments {
                let next_seg = &mut self.segments[idx + 1];

                // Die Start-Tangente des NÄCHSTEN Segments ist exakt 'current_tangent' (da C-Spalte bei u=0.0)
                let tangent_next_start = current_tangent;

                // Generiere das deterministische 2D-Referenzsystem für das nächste Segment
                let abs_t_next = tangent_next_start.abs();
                let ref_v_next = if abs_t_next.x < abs_t_next.y && abs_t_next.x < abs_t_next.z {
                    Vec3::X
                } else if abs_t_next.y < abs_t_next.z {
                    Vec3::Y
                } else {
                    Vec3::Z
                };

                let n_ref_next = ref_v_next.cross(tangent_next_start).normalize();
                let b_ref_next = tangent_next_start.cross(n_ref_next).normalize();

                // Projiziere die fortgepflanzte 3D-Normale in die lokale 2D-Ebene des neuen Segments
                next_seg.rmf_start_normal = Vec2::new(
                    current_normal.dot(n_ref_next),
                    current_normal.dot(b_ref_next),
                );
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
