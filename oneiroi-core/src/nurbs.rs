use glam::{Mat4, Vec3, Vec4};

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
        for val in &mut alloc[0..(points.len() + 4) / 2] {
            val.write(0.);
        }

        for val in &mut alloc[(points.len() + 4) / 2..points.len() + 4] {
            val.write(1.);
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
    println!("Coeff Mat: {coeff_mat}");
    coeff_mat
}
