use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec4, Vec4Swizzles};

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
/// - Caching the Marsden Identity for the Segment
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct NurbsSegmentCache {
    coefficients: Mat4,

    // The Knot start and end value for the given segment to avoid knot vector upload.
    t_start: f32,
    t_end: f32,

    length: f32,
    cumulative_length: f32,
}

/// A Cubic Nurbs curve that can be evaluated extremly efficiently on the CPU and GPU.
/// To achieve this it uses the Marsden Identity.
pub struct CubicNurbs {
    /// Includes the weight of the point in the w coordinate.
    points: Box<[Vec4]>,
    knots: Box<[f32]>,
    segments: Box<[NurbsSegmentCache]>,
}

impl CubicNurbs {
    pub fn new(control_points: Vec<Vec4>, knots: Vec<f32>) -> Self {
        let num_points = control_points.len();

        assert_eq!(
            knots.len(),
            num_points + 4,
            "Knots length must be equal to num_points + degree + 1"
        );

        let points = control_points.into_boxed_slice();
        let knot_vec = knots.into_boxed_slice();

        // 2. Generate segment caches using your exact Marsden method
        let num_interior_segments = num_points - 3;
        let mut segments_cache = Vec::with_capacity(num_interior_segments);

        for idx in 0..num_interior_segments {
            let r = idx + 3;
            let t_start = knot_vec[r];
            let t_end = knot_vec[r + 1];

            // Skip zero-length knot spans (used for sharp kinks/multiplicity in NURBS)
            if (t_end - t_start).abs() < 1e-6 {
                continue;
            }

            let marsden_identity = compute_nurbs_coefficient_matrix(&knot_vec, r);

            // Gather the 4 active homogenous control points for this specific span
            let p0 = points[idx];
            let p1 = points[idx + 1];
            let p2 = points[idx + 2];
            let p3 = points[idx + 3];
            let p_mat = Mat4::from_cols(p0, p1, p2, p3);

            // Compute the composite matrix transformation
            let monom = p_mat.mul_mat4(&marsden_identity);

            segments_cache.push(NurbsSegmentCache {
                coefficients: monom,
                t_start,
                t_end,
                length: 0.,
                cumulative_length: 0.,
            });
        }

        let mut curve = Self {
            points,
            knots: knot_vec,
            segments: segments_cache.into_boxed_slice(),
        };

        curve.recompute_lengths();
        curve
    }

    fn recompute_lengths(&mut self) {
        let num_segments = self.segments.len();
        let mut total_length = 0.0;

        for idx in 0..num_segments {
            // Wir holen uns eine unmutable Referenz für die Auswertung
            let segment = &self.segments[idx];

            // Wenn t_cutoff == segment.t_end, berechnet die Funktion die volle Segmentlänge
            let seg_len = self.length_inside_segment(segment, segment.t_end);

            total_length += seg_len;

            // Mutable Zuweisung erst ganz am Ende, um den Borrow-Checker zu bedienen
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

        // Lineare Suche nach dem passenden Zeitfenster
        self.segments.partition_point(|seg| t >= seg.t_start) - 1
    }

    pub fn evaluate(&self, t: f32) -> Vec3 {
        let segment_idx = self.find_segment_idx(t);
        let segment = &self.segments[segment_idx];

        let u = (t - segment.t_start) / (segment.t_end - segment.t_start);

        let u_splat = Vec4::splat(u);
        let mat = segment.coefficients;

        let horner_eval = mat
            .col(3)
            .mul_add(u_splat, mat.col(2))
            .mul_add(u_splat, mat.col(1))
            .mul_add(u_splat, mat.col(0));

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

        // Columns are already fully baked [A, B, C, D]
        let a = segment.coefficients.col(0);
        let b = segment.coefficients.col(1);
        let c = segment.coefficients.col(2);
        let d = segment.coefficients.col(3);

        // Position (4D): u * (u * (u * D + C) + B) + A
        let p_hom = d
            .mul_add(u_splat, c)
            .mul_add(u_splat, b)
            .mul_add(u_splat, a);

        // First Derivative (4D) w.r.t u: u * (3*D * u + 2*C) + B
        let d3 = d * 3.0;
        let d2 = c * 2.0;
        let dp_du = d3.mul_add(u_splat, d2).mul_add(u_splat, b);

        // Second Derivative (4D) w.r.t u: 6*D * u + 2*C
        let d6 = d * 6.0;
        let d2p_du2 = d6.mul_add(u_splat, d2);

        let inv_dt = 1.0 / dt;
        let inv_dt2 = inv_dt * inv_dt;

        let a_xyz = p_hom.xyz();
        let w = p_hom.w;

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
        let a = segment.coefficients.col(0);
        let b = segment.coefficients.col(1);
        let c = segment.coefficients.col(2);
        let d = segment.coefficients.col(3);

        // Position: u * (u * (u * D + C) + B) + A
        let p_hom = d
            .mul_add(u_splat, c)
            .mul_add(u_splat, b)
            .mul_add(u_splat, a);

        // Derivative with respect to u: u * (3*D * u + 2*C) + B
        let d3 = d * 3.0;
        let d2 = c * 2.0;
        let dp_du = d3.mul_add(u_splat, d2).mul_add(u_splat, b);

        let inv_dt = 1.0 / dt;
        let a_xyz = p_hom.xyz();
        let w = p_hom.w;

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

    fn length_inside_segment(&self, segment: &NurbsSegmentCache, t_cutoff: f32) -> f32 {
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
        let mut current_seg_idx = 0;

        for i in 1..(count - 1) {
            let target_s = i as f32 * step;

            while current_seg_idx < self.segments.len() - 1
                && self.segments[current_seg_idx].cumulative_length < target_s
            {
                current_seg_idx += 1;
            }

            let segment = &self.segments[current_seg_idx];
            let seg_start_len = if current_seg_idx == 0 {
                0.0
            } else {
                self.segments[current_seg_idx - 1].cumulative_length
            };
            let s_local = target_s - seg_start_len;

            let mut t =
                segment.t_start + (s_local / segment.length) * (segment.t_end - segment.t_start);

            // Because our start t should be extremely accurate we only do one loop for now.
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

            points.push(self.evaluate_tanget(t));
        }

        points.push(self.evaluate_tanget(self.segments.last().unwrap().t_end));
        points
    }

    pub fn compute_rmf_frames(
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
    }

    /// Sweeps a 2D profile down the length of the NURBS spline curve.
    /// Returns an aligned vertex buffer and a triangle-strip index buffer for rendering.
    pub fn sweep_profile(
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
    }
}

fn symmetric_functions(a: f32, b: f32, c: f32) -> [f32; 4] {
    [1.0, a + b + c, a * b + b * c + a * c, a * b * c]
}

pub fn compute_nurbs_coefficient_matrix(knots: &[f32], r: usize) -> Mat4 {
    let t_r = knots[r];
    let t_r1 = knots[r + 1];
    let dt = t_r1 - t_r;

    let mut delta_cols = [Vec4::ZERO; 4];

    for (col_idx, i) in (r - 3..=r).enumerate() {
        let sym = symmetric_functions(knots[i + 1], knots[i + 2], knots[i + 3]);
        delta_cols[col_idx] = Vec4::new(sym[0] / 1.0, sym[1] / 3.0, sym[2] / 3.0, sym[3] / 1.0);
    }

    let delta = Mat4::from_cols(delta_cols[0], delta_cols[1], delta_cols[2], delta_cols[3]);
    let delta_inv = delta.inverse();

    let m_u = Mat4::from_cols(
        Vec4::new(1.0, t_r, t_r.powi(2), t_r.powi(3)),
        Vec4::new(0.0, dt, 2.0 * t_r * dt, 3.0 * t_r.powi(2) * dt),
        Vec4::new(0.0, 0.0, dt.powi(2), 3.0 * t_r * dt.powi(2)),
        Vec4::new(0.0, 0.0, 0.0, dt.powi(3)),
    );
    delta_inv.mul_mat4(&m_u)
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
