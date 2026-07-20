// Test matrix based nurbs implementation
use glam::{Mat4, Vec3, Vec4};

/// Berechnet die elementaren symmetrischen Funktionen (Sym_j) für 3 Knotenschritte.
/// Wird für die Konstruktion der Delta-Matrix basierend auf der Marsden-Identität benötigt.
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
    delta_inv.mul_mat4(&m_u)
}

/// Evaluiert einen Punkt auf der NURBS-Kurve im Intervall [t_r, t_{r+1}) mittels Horner-Schema.
pub fn evaluate_nurbs(
    control_points: &[Vec3],
    weights: &[f32],
    knots: &[f32],
    r: usize,
    t: f32,
) -> Vec3 {
    let t_r = knots[r];
    let t_r1 = knots[r + 1];
    let u = (t - t_r) / (t_r1 - t_r);

    // Berechne die explizite Koeffizientenmatrix
    let a_matrix = compute_nurbs_coefficient_matrix(knots, r);

    // Extrahiere die 4 relevanten Kontrollpunkte in homogenen Koordinaten (Vec4)
    let mut p_h = [Vec4::ZERO; 4];
    for (idx, i) in (r - 3..=r).enumerate() {
        let w = weights[i];
        p_h[idx] = Vec4::new(
            control_points[i].x * w,
            control_points[i].y * w,
            control_points[i].z * w,
            w,
        );
    }

    // Berechne die transformierten Kontrollkoeffizienten (C = P^h * A)
    // Entspricht der Multiplikation der Kontrollpunkte mit der expliziten Matrix
    let mut c = [Vec4::ZERO; 4];
    for j in 0..4 {
        let row = a_matrix.row(j);
        c[j] = p_h[0] * row.x + p_h[1] * row.y + p_h[2] * row.z + p_h[3] * row.w;
    }

    // Effiziente Polynomevaluation mittels Horner-Schema auf der Power-Basis
    let evaluated_homogeneous = c[0] + u * (c[1] + u * (c[2] + u * c[3]));

    // Dehomogenisierung: Zurückrechnen in den 3D-Raum (Projektion)
    Vec3::new(
        evaluated_homogeneous.x / evaluated_homogeneous.w,
        evaluated_homogeneous.y / evaluated_homogeneous.w,
        evaluated_homogeneous.z / evaluated_homogeneous.w,
    )
}

fn main() {
    // Beispiel-Setup für eine kubische offene NURBS-Kurve (Knotenanzahl = Punkte + Ordnung)
    let control_points = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 3.0, 0.0),
        Vec3::new(4.0, 3.0, 0.0),
        Vec3::new(5.0, 0.0, 0.0),
    ];
    let weights = vec![1.0, 1.0, 1.0, 1.0];
    let knots = vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0]; // Standard geklemmter Clamped-Knotenvektor

    // Wir evaluieren im gültigen Intervall r = 3 (entspricht [knots[3], knots[4]) -> [0.0, 1.0))
    let r = 3;
    let t = 0.5;

    let point = evaluate_nurbs(&control_points, &weights, &knots, r, t);
    println!("Evaluierter Punkt bei t={}: {:?}", t, point);
}
