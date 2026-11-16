//! A row of tubes swept along a NURBS curve.
//!
//! The three tubes in the middle have the same parameters, so they are one
//! computation and one mesh. The left one changes its radius, which only
//! meshes the tube again. The right one moves its control points, which
//! rebuilds its curve as well.

use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    prelude::*,
};
use oneiroi_bevy::{INDICES, NORMALS, Oneiroi, OneiroiInstance, OneiroiPlugin, POSITIONS, UVS};
use oneiroi_core::program::{Program, ProgramBuilder};
use oneiroi_nodes::curve::{NURBS, TUBE, nurbs::clamped_uniform_knots};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FreeCameraPlugin, OneiroiPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (pulse, wave, report))
        .run();
}

fn points(time: f32) -> Vec<[f32; 4]> {
    (0..8)
        .map(|i| {
            let along = i as f32;
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            [
                side * (0.6 + 0.4 * (time + along).sin()),
                along,
                0.5 * (time * 0.7 + along * 1.3).cos(),
                1.0,
            ]
        })
        .collect()
}

/// The control points and the look of the tube are exposed, the knots of
/// the curve are not and therefore the same for every instance.
fn tube() -> Program {
    let mut b = ProgramBuilder::new("tube");
    let points = b.param("points", &points(0.0));
    let radius = b.param::<f32>("radius", &[0.2]);
    let rings = b.param::<u32>("rings", &[16]);
    let sides = b.param::<u32>("sides", &[24]);
    let knots = b.constant(&clamped_uniform_knots(8));

    let segments = b.node(&NURBS, &[points, knots]).unwrap()[0];
    let mesh = b.node(&TUBE, &[segments, radius, rings, sides]).unwrap();
    for (name, slot) in [POSITIONS, NORMALS, UVS, INDICES].iter().zip(mesh) {
        b.output(*name, slot);
    }
    b.finish()
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
    let program = oneiroi.add_program(tube());
    let material = materials.add(Color::srgb(0.8, 0.7, 0.6));

    for column in 0..5 {
        let mut tube = commands.spawn((
            oneiroi.instantiate(program),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz((column as f32 - 2.0) * 3.0, 0.0, 0.0),
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
        Transform::from_xyz(0.0, 3.5, 14.0).looking_at(Vec3::new(0.0, 3.5, 0.0), Vec3::Y),
    ));
}

fn pulse(
    time: Res<Time>,
    oneiroi: Res<Oneiroi>,
    mut tubes: Query<&mut OneiroiInstance, With<Pulse>>,
) {
    let radius = 0.2 + 0.1 * (time.elapsed_secs() * 2.0).sin();
    for mut tube in &mut tubes {
        oneiroi.set(&mut tube, "radius", &[radius]).unwrap();
    }
}

fn wave(time: Res<Time>, oneiroi: Res<Oneiroi>, mut tubes: Query<&mut OneiroiInstance, With<Wave>>) {
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
