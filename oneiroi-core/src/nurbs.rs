use glam::{Mat4, Vec3, Vec4, Vec4Swizzles};

mod gpu_cached;

pub struct CubicNurbs {
    /// The control points of the Curve
    points: Box<[Vec3]>,
    /// The numer of knots of a nurbs curve is always:
    /// num_knots = degree (cubic=3) + num_ctrl_pts + 1
    /// That means in this case we have 4 + points.len() knots.
    /// That means the weights start at that index.
    knots_followed_by_weights: Box<[f32]>,

    mardsen_cache: Box<[Mat4]>,
}

impl CubicNurbs {
    pub fn cubic_bezier(points: Vec<Vec3>) -> Self {
        let num_knots_and_weigths = points.len() * 2 + 4;

        let mut alloc = Box::new_uninit_slice(num_knots_and_weigths);

        // A standard uniform knot vector requires 4 zeroes at the start and 4 ones at the end...
        for i in 0..4 {
            alloc[i].write(0.0);
        }
        for i in points.len()..points.len() + 4 {
            alloc[i].write(1.0);
        }
        // The rest of the inbetween knot vector scalars are distributed equidistantly apart from each other.
        let num_interior_segments = points.len() - 3;
        for i in 4..points.len() {
            let interior_t = (i - 3) as f32 / num_interior_segments as f32;
            alloc[i].write(interior_t);
        }

        for val in &mut alloc[points.len() + 4..num_knots_and_weigths] {
            val.write(1.);
        }

        // # Safety: All initialized above.
        let knots_followed_by_weights = unsafe { alloc.assume_init() };

        let mut mardsen_cache = Box::new_uninit_slice(points.len() - 3);

        for (idx, val) in mardsen_cache.iter_mut().enumerate() {
            val.write(compute_nurbs_coefficient_matrix(
                &knots_followed_by_weights,
                idx + 3,
            ));
        }

        let mardsen_cache = unsafe { mardsen_cache.assume_init() };

        Self {
            points: points.into_boxed_slice(),
            knots_followed_by_weights,
            mardsen_cache,
        }
    }

    fn span(&self, t: f32) -> usize {
        let n = self.points.len();
        let knots = &self.knots_followed_by_weights;
        // T is at the end of the curve.
        if t >= knots[n] {
            return n - 1;
        }
        if t <= knots[3] {
            return 3;
        }

        let mut low = 3;
        let mut high = n;
        let mut mid = (low + high) / 2;

        while t < knots[mid] || t >= knots[mid + 1] {
            if t < knots[mid] {
                high = mid;
            } else {
                low = mid;
            }
            mid = (low + high) / 2;
        }
        mid
    }

    pub fn evaluate(&self, t: f32) -> Vec3 {
        let knots = &self.knots_followed_by_weights;
        let weights = &self.knots_followed_by_weights[self.points.len() + 4..];
        let points = &self.points;
        let r = self.span(t);

        let t_r = knots[r];
        let t_r1 = knots[r + 1];
        let u = (t - t_r) / (t_r1 - t_r);

        // Berechne die explizite Koeffizientenmatrix
        //let a_matrix = compute_nurbs_coefficient_matrix(knots, r);
        let a_matrix = self.mardsen_cache[r - 3];

        // Extrahiere die 4 relevanten Kontrollpunkte in homogenen Koordinaten (Vec4)
        let mut p_h = [Vec4::ZERO; 4];
        for (idx, i) in (r - 3..=r).enumerate() {
            let w = weights[i];
            p_h[idx] = Vec4::new(points[i].x * w, points[i].y * w, points[i].z * w, w);
        }

        //println!("PH: {p_h:?}");

        // Berechne die transformierten Kontrollkoeffizienten (C = P^h * A)
        // Entspricht der Multiplikation der Kontrollpunkte mit der expliziten Matrix
        let mut c = [Vec4::ZERO; 4];
        for j in 0..4 {
            let row = a_matrix.col(j);
            c[j] = p_h[0] * row.x + p_h[1] * row.y + p_h[2] * row.z + p_h[3] * row.w;
        }

        //println!("C: {c:?}");

        // Effiziente Polynomevaluation mittels Horner-Schema auf der Power-Basis
        let horner_eval = c[0] + u * (c[1] + u * (c[2] + u * c[3]));
        //println!("{evaluated_homogeneous}");
        // Dehomogenisierung: Zurückrechnen in den 3D-Raum (Projektion)
        Vec3::new(
            horner_eval.x / horner_eval.w,
            horner_eval.y / horner_eval.w,
            horner_eval.z / horner_eval.w,
        )
    }

    pub fn evaluate_derivatives(&self, t: f32) -> (Vec3, Vec3, Vec3) {
        let knots = &self.knots_followed_by_weights;
        let weights = &self.knots_followed_by_weights[self.points.len() + 4..];
        let points = &self.points;
        let r = self.span(t);

        let t_r = knots[r];
        let t_r1 = knots[r + 1];
        let dt = t_r1 - t_r;
        let u = (t - t_r) / dt;

        let a_matrix = self.mardsen_cache[r - 3];

        // 1. Extract the 4 relevant control points in homogeneous coordinates
        let mut p_h = [Vec4::ZERO; 4];
        for (idx, i) in (r - 3..=r).enumerate() {
            let w = weights[i];
            p_h[idx] = Vec4::new(points[i].x * w, points[i].y * w, points[i].z * w, w);
        }

        // 2. Compute polynomial coefficients via Marsden matrix
        let mut c = [Vec4::ZERO; 4];
        for j in 0..4 {
            let row = a_matrix.col(j);
            c[j] = p_h[0] * row.x + p_h[1] * row.y + p_h[2] * row.z + p_h[3] * row.w;
        }

        // 3. Evaluate homogeneous position and its derivatives w.r.t local parameter u
        let p_hom = c[0] + u * (c[1] + u * (c[2] + u * c[3]));
        let dp_du = c[1] + u * (2.0 * c[2] + 3.0 * u * c[3]);
        let d2p_du2 = 2.0 * c[2] + 6.0 * u * c[3];

        // 4. Transform derivatives to global parameter t using chain rule
        let inv_dt = 1.0 / dt;
        let inv_dt2 = inv_dt * inv_dt;

        let a = p_hom.xyz();
        let w = p_hom.w;

        let da = dp_du.xyz() * inv_dt;
        let dw = dp_du.w * inv_dt;

        let d2a = d2p_du2.xyz() * inv_dt2;
        let d2w = d2p_du2.w * inv_dt2;

        // 5. Dehomogenization via the Rational Quotient Rule
        let c_pos = a / w;
        let c_vel = (da - dw * c_pos) / w;
        let c_acc = (d2a - 2.0 * dw * c_vel - d2w * c_pos) / w;

        (c_pos, c_vel, c_acc)
    }

    pub fn curvature(&self, t: f32) -> f32 {
        let (_, tangent, second_deriv) = self.evaluate_derivatives(t);

        let numerator = tangent.cross(second_deriv).length();
        let denominator = tangent.length().powi(3);

        if denominator.abs() < 1e-6 {
            0.0 // Handles flat lines or linear cusps safely
        } else {
            numerator / denominator
        }
    }

    pub fn length(&self) -> f32 {
        let knots = &self.knots_followed_by_weights;
        let n = self.points.len();

        // 5-point Gauss-Legendre constants (abscissae and weights on interval [-1, 1])
        const GAUSS_X: [f32; 5] = [
            -0.90617985,
            -0.5384693,
            0.0,
            0.5384693,
            0.090617985, // Corrected pair mirror sequence order below:
        ];
        // Exact 5-point weights and nodes matches:
        let nodes: [f32; 5] = [
            0.0,
            -0.5384693101,
            0.5384693101,
            -0.9061798459,
            0.9061798459,
        ];
        let weights: [f32; 5] = [
            0.5688888889,
            0.4786286705,
            0.4786286705,
            0.2369268851,
            0.2369268851,
        ];

        let mut total_length = 0.0;

        // Iterate through all possible valid knot spans [t_r, t_{r+1})
        // For a cubic curve, valid evaluation spans are from index 3 up to n
        for r in 3..n {
            let t_r = knots[r];
            let t_r1 = knots[r + 1];
            let dt = t_r1 - t_r;

            // Skip zero-length knot intervals safely
            if dt.abs() < 1e-6 {
                continue;
            }

            // Perform numerical integration across this specific knot span
            let mut span_integral = 0.0;
            for i in 0..5 {
                // Map the standard Gauss node from [-1, 1] to our local span [t_r, t_{r+1}]
                let t = 0.5 * ((t_r1 - t_r) * nodes[i] + (t_r1 + t_r));

                // Get the velocity vector C'(t) at this point
                let (_, tangent, _) = self.evaluate_derivatives(t);

                // Add the speed (magnitude of velocity) scaled by the Gauss weight
                span_integral += weights[i] * tangent.length();
            }

            // Scale by the half-width of the transformation interval (Jacobian determinant)
            total_length += span_integral * 0.5 * dt;
        }

        total_length
    }

    pub fn time_at_distance(&self, target_s: f32, total_length: f32) -> f32 {
        let knots = &self.knots_followed_by_weights;
        let n = self.points.len();
        let t_start = knots[3];
        let t_end = knots[n];

        // Clamp boundary conditions safely
        if target_s <= 0.0 {
            return t_start;
        }
        if target_s >= total_length {
            return t_end;
        }

        // 1. Initial Guess via proportional interpolation
        let mut t = t_start + (target_s / total_length) * (t_end - t_start);

        // 2. Newton-Raphson Iteration loop
        // f(t) = current_distance(t) - target_s = 0
        // Update formula: t_next = t - f(t) / f'(t)
        // Note that f'(t) is simply the magnitude of the velocity vector: ||C'(t)||
        for _ in 0..8 {
            let current_s = self.length_up_to(t);
            let (_, tangent, _) = self.evaluate_derivatives(t);
            let speed = tangent.length();

            // Guard against division by zero if the curve stops or forms a cusp
            if speed < 1e-5 {
                break;
            }

            let delta_t = (current_s - target_s) / speed;
            t -= delta_t;

            // Enforce bounds constraints during optimization
            t = t.clamp(t_start, t_end);

            if delta_t.abs() < 1e-5 {
                break; // Converged safely
            }
        }

        t
    }

    /// Helper that calculates the cumulative arc length from t_start up to parameter `t_cutoff`.
    fn length_up_to(&self, t_cutoff: f32) -> f32 {
        let knots = &self.knots_followed_by_weights;
        let n = self.points.len();

        let nodes: [f32; 5] = [0.0, -0.5384693, 0.5384693, -0.90617985, 0.90617985];
        let weights: [f32; 5] = [0.5688889, 0.47862867, 0.47862867, 0.23692689, 0.23692689];

        let mut current_length = 0.0;

        for r in 3..n {
            let t_r = knots[r];
            let t_r1 = knots[r + 1];

            // If this segment is fully behind our cutoff, integrate the full span
            // If we are inside the cut-off segment, trim the upper integration bound
            let upper_bound = if t_cutoff < t_r1 { t_cutoff } else { t_r1 };

            let dt = upper_bound - t_r;
            if dt <= 1e-6 {
                continue; // This or subsequent spans are out of scope
            }

            let mut span_integral = 0.0;
            for i in 0..5 {
                let t = 0.5 * (dt * nodes[i] + (upper_bound + t_r));
                let (_, tangent, _) = self.evaluate_derivatives(t);
                span_integral += weights[i] * tangent.length();
            }
            current_length += span_integral * 0.5 * dt;

            if t_cutoff < t_r1 {
                break; // Optimization: We passed our upper limit, stop scanning segments
            }
        }

        current_length
    }

    /// Evaluates `count` points spaced perfectly equidistant along the curve.
    pub fn sample_equidistant(&self, count: usize) -> Vec<(Vec3, Vec3, Vec3)> {
        if count == 0 {
            return Vec::new();
        }
        if count == 1 {
            return vec![self.evaluate_derivatives(self.knots_followed_by_weights[3])];
        }

        let total_length = self.length();
        let step = total_length / (count - 1) as f32;
        let mut points = Vec::with_capacity(count);

        for i in 0..count {
            let target_s = i as f32 * step;
            let t = self.time_at_distance(target_s, total_length);
            points.push(self.evaluate_derivatives(t));
        }

        points
    }
}

fn symmetric_functions(a: f32, b: f32, c: f32) -> [f32; 4] {
    [
        1.0,                   // Sym_0
        a + b + c,             // Sym_1
        a * b + b * c + a * c, // Sym_2
        a * b * c,             // Sym_3
    ]
}

/// Berechnet die Koeffizientenmatrix A_{r,k,T} für ein bestimmtes Knotenintervall [t_r, t_{r+1}).
/// Verwendet den Ansatz der Marsden-Identität (Sektion 4 des Papers).
pub fn compute_nurbs_coefficient_matrix(knots: &[f32], r: usize) -> Mat4 {
    let t_r = knots[r];
    let t_r1 = knots[r + 1];
    let dt = t_r1 - t_r;

    // 1. Konstruktion der Matrix Delta (D) nach Gleichung (22) & (24)
    // Die Spalten entsprechen den Basis-Indizes i = r-3, r-2, r-1, r
    let mut delta_cols = [Vec4::ZERO; 4];

    for (col_idx, i) in (r - 3..=r).enumerate() {
        // Die 3 relevanten Knoten für das jeweilige B-Spline-Segment: t_{i+1}, t_{i+2}, t_{i+3}
        let sym = symmetric_functions(knots[i + 1], knots[i + 2], knots[i + 3]);

        // Elemente gewichtet mit 1 / binom(k-1, j) für k=4 (Binomialkoeffizienten: 1, 3, 3, 1)
        delta_cols[col_idx] = Vec4::new(sym[0] / 1.0, sym[1] / 3.0, sym[2] / 3.0, sym[3] / 1.0);
    }

    // Mat4::from_cols erwartet die Spaltenvektoren
    let delta = Mat4::from_cols(delta_cols[0], delta_cols[1], delta_cols[2], delta_cols[3]);
    let delta_inv = delta.inverse();

    // 2. Transformation von der globalen Zeit 't' auf die lokale Variable u in [0, 1)
    // t = t_r + u * dt. Wir bauen die untere Dreiecksmatrix M_u für die Monombasis-Wandlung:
    // t^0 = 1
    // t^1 = t_r + dt * u
    // t^2 = t_r^2 + 2*t_r*dt * u + dt^2 * u^2
    // t^3 = t_r^3 + 3*t_r^2*dt * u + 3*t_r*dt^2 * u^2 + dt^3 * u^3
    let m_u = Mat4::from_cols(
        Vec4::new(1.0, t_r, t_r.powi(2), t_r.powi(3)),
        Vec4::new(0.0, dt, 2.0 * t_r * dt, 3.0 * t_r.powi(2) * dt),
        Vec4::new(0.0, 0.0, dt.powi(2), 3.0 * t_r * dt.powi(2)),
        Vec4::new(0.0, 0.0, 0.0, dt.powi(3)),
    );

    // Gemäß Sektion 4 gilt: A = Delta^-1 * M_u
    // Da glam Spaltenvektoren nutzt und Matrixmultiplikationen von rechts nach links liest,
    // transformieren wir die Zeilen-/Spaltenlogik passend zur Paper-Gleichung (4).
    let coeff_mat = delta_inv.mul_mat4(&m_u);
    //println!("Coeff Mat: {coeff_mat}");
    coeff_mat
}
