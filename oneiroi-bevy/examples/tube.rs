//! Profiles swept along a NURBS curve.
//!
//! The three tubes in the middle have the same parameters, so they are one
//! computation and one mesh. The left one changes its radius, which makes
//! a new circle and sweeps it along the same curve. The one right of them
//! moves its control points, which rebuilds its curve as well.
//!
//! The beam on the far right is the same sweep node with a curve instead
//! of a circle as its profile.

use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    prelude::*,
};
use oneiroi_bevy::{Oneiroi, OneiroiInstance, OneiroiPlugin};
use oneiroi_core::program::{ProgramBuilder, ProgramId};
use oneiroi_nodes::{
    curve::{ControlPoint, NURBS, nurbs::clamped_uniform_knots, swept, tube},
    surface,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FreeCameraPlugin, OneiroiPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (pulse, wave, report))
        .run();
}

fn points(time: f32) -> Vec<ControlPoint> {
    (0..8)
        .map(|i| {
            let along = i as f32;
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            // Bevy has a glam of its own, so the position is plain numbers.
            ControlPoint::from([
                side * (0.6 + 0.4 * (time + along).sin()),
                along,
                0.5 * (time * 0.7 + along * 1.3).cos(),
                1.0,
            ])
        })
        .collect()
}

/// A tube along a curve. The control points and the look of the tube are
/// exposed, the knots of the curve are not and therefore the same for
/// every instance.
fn tube_along_curve(oneiroi: &mut Oneiroi) -> ProgramId {
    let tube = oneiroi.add_program(tube(oneiroi.registry()));

    let mut b = ProgramBuilder::new("tube along curve");
    let points = b.param("points", &points(0.0));
    let radius = b.param::<f32>("radius", &[0.2]);
    let knots = b.constant(&clamped_uniform_knots(8));
    let spine = b.node(&NURBS, &[points, knots]).unwrap()[0];
    // The mesh keeps the default tolerance of the tube.
    let outputs = b
        .call(oneiroi.programs(), tube, &[Some(spine), Some(radius), None])
        .unwrap();
    let names: Vec<_> = oneiroi.programs().get(tube).outputs.iter().map(|o| o.name.clone()).collect();
    for (name, slot) in names.into_iter().zip(outputs) {
        b.output(name, slot);
    }
    oneiroi.add_program(b.finish())
}

/// The same curve with a square swept along it. The square is a NURBS
/// curve whose knots repeat, which gives it straight edges and corners.
fn beam_along_curve(oneiroi: &mut Oneiroi) -> ProgramId {
    let corners = [(0.2, -0.2), (0.2, 0.2), (-0.2, 0.2), (-0.2, -0.2), (0.2, -0.2)];
    let mut square = vec![ControlPoint::from([0.2, -0.2, 0.0, 1.0])];
    for pair in corners.windows(2) {
        let (from, to) = (Vec2::from(pair[0]), Vec2::from(pair[1]));
        for step in 1..=3 {
            let point = from.lerp(to, step as f32 / 3.0);
            square.push(ControlPoint::from([point.x, point.y, 0.0, 1.0]));
        }
    }
    let mut square_knots = vec![0.0; 4];
    for knot in [0.25, 0.5, 0.75] {
        square_knots.extend([knot; 3]);
    }
    square_knots.extend([1.0; 4]);

    let mut b = ProgramBuilder::new("beam along curve");
    let points = b.param("points", &points(0.0));
    let knots = b.constant(&clamped_uniform_knots(8));
    let spine = b.node(&NURBS, &[points, knots]).unwrap()[0];
    let points = b.constant(&square);
    let knots = b.constant(&square_knots);
    let profile = b.node(&NURBS, &[points, knots]).unwrap()[0];
    let surface = swept(&mut b, oneiroi.registry(), spine, profile).unwrap();
    // Each edge of the square is flat, so the mesh takes a single step
    // across it by itself.
    let tolerance = b.constant::<f32>(&[surface::TOLERANCE]);
    for (name, slot) in surface::mesh(&mut b, oneiroi.registry(), surface, tolerance).unwrap() {
        b.output(name, slot);
    }
    oneiroi.add_program(b.finish())
}

#[derive(Component)]
struct Pulse;

#[derive(Component)]
struct Wave;

fn setup(
    mut commands: Commands,
    mut oneiroi: ResMut<Oneiroi>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let tube = tube_along_curve(&mut oneiroi);
    let beam = beam_along_curve(&mut oneiroi);
    let material = materials.add(Color::srgb(0.8, 0.7, 0.6));

    for column in 0..6 {
        let program = if column == 5 { beam } else { tube };
        let mut tube = commands.spawn((
            oneiroi.instantiate(program),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz((column as f32 - 2.5) * 3.0, 0.0, 0.0),
        ));
        match column {
            0 => tube.insert(Pulse),
            4 => tube.insert(Wave),
            _ => &mut tube,
        };
    }

    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 10.0, 8.0),
    ));
    commands.spawn((
        Camera3d::default(),
        FreeCamera::default(),
        Transform::from_xyz(0.0, 3.5, 16.0).looking_at(Vec3::new(0.0, 3.5, 0.0), Vec3::Y),
    ));
}

fn pulse(
    time: Res<Time>,
    mut oneiroi: ResMut<Oneiroi>,
    mut tubes: Query<&mut OneiroiInstance, With<Pulse>>,
) {
    let radius = 0.2 + 0.1 * (time.elapsed_secs() * 2.0).sin();
    for mut tube in &mut tubes {
        oneiroi.set(&mut tube, "radius", &[radius]).unwrap();
    }
}

fn wave(time: Res<Time>, mut oneiroi: ResMut<Oneiroi>, mut tubes: Query<&mut OneiroiInstance, With<Wave>>) {
    for mut tube in &mut tubes {
        oneiroi
            .set(&mut tube, "points", &points(time.elapsed_secs()))
            .unwrap();
    }
}

fn report(time: Res<Time>, oneiroi: Res<Oneiroi>, mut next: Local<f32>) {
    if time.elapsed_secs() >= *next {
        *next = time.elapsed_secs() + 1.0;
        info!("{:?}", oneiroi.stats());
    }
}
