use glam::Vec3;
use oneiroi_core::nurbs::CubicNurbs;

fn main() {
    /* let control_points = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 2.0, 0.0),
        Vec3::new(2.0, -1.0, 0.0),
        Vec3::new(3.0, 3.0, 0.0),
        Vec3::new(4.0, 0.0, 0.0),
        Vec3::new(5.0, 2.0, 0.0),
        Vec3::new(6.0, 1.0, 0.0),
        Vec3::new(7.0, 4.0, 0.0),
    ]; */
    let control_points = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 3.0, 0.0),
        Vec3::new(4.0, 3.0, 0.0),
        Vec3::new(5.0, 0.0, 0.0),
    ];
    let curve = CubicNurbs::cubic_bezier(control_points);

    // 2. Schnelle Auswertung zur Laufzeit
    let steps = 10;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let pt = curve.evaluate(t);
        println!("t = {:.2} -> Punkt: {:?}", t, pt);
    }
}
