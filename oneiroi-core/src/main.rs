use std::time::Instant;

use glam::Vec4;

fn main() {
    let control_points = vec![
        Vec4::new(0.0, 0.0, 0.0, 1.),
        Vec4::new(1.0, 2.0, 0.0, 1.),
        Vec4::new(2.0, -1.0, 0.0, 1.),
        Vec4::new(3.0, 3.0, 0.0, 1.),
        Vec4::new(4.0, 0.0, 0.0, 1.),
        Vec4::new(5.0, 2.0, 0.0, 1.),
        Vec4::new(6.0, 1.0, 0.0, 1.),
        Vec4::new(7.0, 4.0, 0.0, 1.),
    ];
    let num_points = control_points.len();
    let num_knots = num_points + 4;

    let mut knot_vec = vec![0.0; num_knots];
    for i in num_points..num_knots {
        knot_vec[i] = 1.0;
    }
    let num_interior_segments = num_points - 3;
    for i in 4..num_points {
        let interior_t = (i - 3) as f32 / num_interior_segments as f32;
        knot_vec[i] = interior_t;
    }
    let curve = oneiroi_core::nurbs::CubicNurbs::new(control_points, knot_vec);

    // 2. Schnelle Auswertung zur Laufzeit
    let steps = 1000;

    /* let instant = Instant::now();
    let uniform_samples = curve.sample_equidistant(steps);
    println!("Evaluation of {steps} steps took: {:?}", instant.elapsed()); */

    let instant = Instant::now();
    let uniform_samples = curve.sample_equidistant(steps);
    println!("Evaluation of {steps} steps took: {:?}", instant.elapsed());

    for step in 0..steps {
        let t = step as f32 / steps as f32;
        let pt = curve.evaluate(t);
        let cv = curve.curvature(t);
        println!("t = {:.2} -> Point: {:?}, Curvature: {cv}", t, pt);
        println!("Uniform Sample at point: {:?}", uniform_samples[step])
    }
    println!("Curve Length is: {}", curve.length());
}
