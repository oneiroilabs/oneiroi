use bevy::{
    asset::RenderAssetUsages,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    color::palettes::css::*,
    prelude::*,
};
use oneiroi_core::nurbs::TransformAtT;
use std::{
    f32::consts::{PI, TAU},
    time::Instant,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FreeCameraPlugin))
        .init_gizmo_group::<MyRoundGizmos>()
        .add_systems(Startup, setup)
        .add_systems(Update, (draw_example_collection, update_config))
        .run();
}

// We can create our own gizmo config group!
#[derive(Default, Reflect, GizmoConfigGroup)]
struct MyRoundGizmos;

fn setup(
    mut commands: Commands,
    mut gizmo_assets: ResMut<Assets<GizmoAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut gizmo = GizmoAsset::new();

    // When drawing a lot of static lines a Gizmo component can have
    // far better performance than the Gizmos system parameter,
    // but the system parameter will perform better for smaller lines that update often.

    // A sphere made out of 30_000 lines!
    /*  gizmo
        .sphere(Isometry3d::IDENTITY, 0.5, CRIMSON)
        .resolution(30_000 / 3);

    commands.spawn((
        Gizmo {
            handle: gizmo_assets.add(gizmo),
            line_config: GizmoLineConfig {
                width: 5.,
                ..default()
            },
            ..default()
        },
        Transform::from_xyz(4., 1., 0.),
    )); */

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0., 1.5, 6.).looking_at(Vec3::ZERO, Vec3::Y),
        FreeCamera::default(),
    ));
    // plane

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

    /* let control_points = vec![
        Vec4::new(1.0, 0.0, 0.0, 1.),        // P0: Start East
        Vec4::new(1.0, 2.0, 0.0, 1. / 3.),   // P1: North-East corner
        Vec4::new(-1.0, 2.0, 0.0, 1. / 3.),  // P2: North-West corner
        Vec4::new(-1.0, 0.0, 0.0, 1.),       // P3: West
        Vec4::new(-1.0, -2.0, 0.0, 1. / 3.), // P4: South-West corner
        Vec4::new(1.0, -2.0, 0.0, 1. / 3.),  // P5: South-East corner
        Vec4::new(1.0, 0.0, 0.0, 1.),        // P6: East (Loop closure)
        Vec4::new(1.0, 2.0, 0.0, 1. / 3.),   // P7: Wrap-around phantom point for cubic continuity
        Vec4::new(-1.0, 2.0, 0.0, 1. / 3.),  // P8: Wrap-around phantom point for cubic continuity
    ];

    let knot_vec = vec![
        0.0, 0.0, 0.0, 0.0, // Clamped start
        1.0, 1.0, 1.0, // Quad 1 to Quad 2 boundary
        2.0, 2.0, 2.0, // Quad 2 to Quad 3 boundary
        3.0, 3.0, 3.0, // Clamped end matching parameter space bounds
    ]; */

    let curve = oneiroi_core::nurbs::CubicNurbs::new(control_points, knot_vec);

    let circle_profile: Vec<Vec2> = (0..16)
        .map(|i| {
            let angle = (i as f32 / 16.0) * std::f32::consts::TAU;
            Vec2::new(angle.cos(), angle.sin())
        })
        .collect();

    let half_w = 0.3 * 0.5;
    let half_h = 0.3 * 0.5;

    // Die Punkte werden im Uhrzeigersinn (oder Gegenuhrzeigersinn) definiert,
    // um eine saubere, nicht-selbstüberkreuzende Geometrie zu gewährleisten.
    let cubic_profile = vec![
        Vec2::new(-half_w, -half_h), // Unten Links
        Vec2::new(-half_w, half_h),  // Oben Links
        Vec2::new(half_w, half_h),   // Oben Rechts
        Vec2::new(half_w, -half_h),  // Unten Rechts
    ];

    let (vertices, indices) = curve.sweep_profile_transformed(
        &circle_profile,
        100,  // Smooth longitudinal subdivision count
        true, // Closed circle cross section
        |t| {
            // Normalize time parameter to a 0.0 -> 1.0 range factor
            let factor = t;

            TransformAtT {
                // Linearly taper from a scale factor of 2.0 down to a sharp 0.1 near the tip
                scale: Vec2::splat(0.3 * (1.0 - factor) + 0.1),
                // Rotate the profile a full revolution over the course of the spine path
                rotation_radians: factor * TAU,
            }
        },
    );

    println!("Number Vertices: {}", vertices.len());

    let mut mesh = Mesh::new(
        wgpu::PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_indices(bevy::mesh::Indices::U32(indices));
    mesh.duplicate_vertices();
    let mesh = mesh.with_computed_flat_normals();

    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 1.0, 1.0))),
    ));
    // cube
    /* commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    )); */
    // light
    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // example instructions
    /* commands.spawn((
        Text::new(
            "Press 'T' to toggle drawing gizmos on top of everything else in the scene\n\
            Press 'P' to toggle perspective for line gizmos\n\
            Hold 'Left' or 'Right' to change the line width of straight gizmos\n\
            Hold 'Up' or 'Down' to change the line width of round gizmos\n\
            Press '1' or '2' to toggle the visibility of straight gizmos or round gizmos\n\
            Press 'B' to show all AABB boxes\n\
            Press 'U' or 'I' to cycle through line styles for straight or round gizmos\n\
            Press 'J' or 'K' to cycle through line joins for straight or round gizmos\n\
            Press 'Spacebar' to toggle pause",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    )); */
}

fn draw_example_collection(
    mut gizmos: Gizmos,
    mut my_gizmos: Gizmos<MyRoundGizmos>,
    time: Res<Time>,
) {
    /* let control_points = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 3.0, 0.0),
        Vec3::new(4.0, 3.0, 0.0),
        Vec3::new(5.0, 0.0, 0.0),
    ]; */
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

    /* let control_points = vec![
        Vec4::new(1.0, 0.0, 0.0, 1.),        // P0: Start East
        Vec4::new(1.0, 2.0, 0.0, 1. / 3.),   // P1: North-East corner
        Vec4::new(-1.0, 2.0, 0.0, 1. / 3.),  // P2: North-West corner
        Vec4::new(-1.0, 0.0, 0.0, 1.),       // P3: West
        Vec4::new(-1.0, -2.0, 0.0, 1. / 3.), // P4: South-West corner
        Vec4::new(1.0, -2.0, 0.0, 1. / 3.),  // P5: South-East corner
        Vec4::new(1.0, 0.0, 0.0, 1.),        // P6: East (Loop closure)
        Vec4::new(1.0, 2.0, 0.0, 1. / 3.),   // P7: Wrap-around phantom point for cubic continuity
        Vec4::new(-1.0, 2.0, 0.0, 1. / 3.),  // P8: Wrap-around phantom point for cubic continuity
    ];

    let knot_vec = vec![
        0.0, 0.0, 0.0, 0.0, // Clamped start
        1.0, 1.0, 1.0, // Quad 1 to Quad 2 boundary
        2.0, 2.0, 2.0, // Quad 2 to Quad 3 boundary
        3.0, 3.0, 3.0, // Clamped end matching parameter space bounds
    ]; */

    let curve = oneiroi_core::nurbs::CubicNurbs::new(control_points, knot_vec);

    let mut points = vec![];
    let mut tangents = vec![];
    let mut curvature = vec![];

    // 2. Schnelle Auswertung zur Laufzeit
    let steps = 100;
    let instant = Instant::now();
    let uniform_samples = curve.sample_equidistant(steps);
    //println!("Evaluation of {steps} steps took: {:?}", instant.elapsed());
    for step in 0..steps {
        let t = step as f32 / steps as f32;
        let pt = curve.evaluate(t);
        let (_, tangent, _) = curve.evaluate_derivatives(t);
        let cv = curve.curvature(t);
        //println!("t = {:.2} -> Punkt: {:?}, Curvature: {cv}", t, pt);
        points.push(pt);
        tangents.push(tangent);
        curvature.push(cv);
    }

    for points in points.windows(2) {
        gizmos.line(points[0], points[1], TEAL);
    }

    for (point, tangent) in uniform_samples.into_iter() {
        gizmos.arrow(point, point.move_towards(tangent, 1.), Color::BLACK);
    }

    let circle_profile: Vec<Vec2> = (0..16)
        .map(|i| {
            let angle = (i as f32 / 16.0) * std::f32::consts::TAU;
            Vec2::new(angle.cos(), angle.sin())
        })
        .collect();

    let (vertices, indices) = curve.sweep_profile_transformed(
        &circle_profile,
        100,  // Smooth longitudinal subdivision count
        true, // Closed circle cross section
        |t| {
            // Normalize time parameter to a 0.0 -> 1.0 range factor
            let factor = t;

            TransformAtT {
                // Linearly taper from a scale factor of 2.0 down to a sharp 0.1 near the tip
                scale: Vec2::splat(2.0 * (1.0 - factor) + 0.1),
                // Rotate the profile a full revolution over the course of the spine path
                rotation_radians: factor * std::f32::consts::TAU,
            }
        },
    );
}

fn update_config(
    mut config_store: ResMut<GizmoConfigStore>,
    keyboard: Res<ButtonInput<KeyCode>>,
    real_time: Res<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) {
        for (_, config, _) in config_store.iter_mut() {
            config.depth_bias = if config.depth_bias == 0. { -1. } else { 0. };
        }
    }
    if keyboard.just_pressed(KeyCode::KeyP) {
        for (_, config, _) in config_store.iter_mut() {
            // Toggle line perspective
            config.line.perspective ^= true;
            // Increase the line width when line perspective is on
            config.line.width *= if config.line.perspective { 5. } else { 1. / 5. };
        }
    }

    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    if keyboard.pressed(KeyCode::ArrowRight) {
        config.line.width += 5. * real_time.delta_secs();
        config.line.width = config.line.width.clamp(0., 50.);
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        config.line.width -= 5. * real_time.delta_secs();
        config.line.width = config.line.width.clamp(0., 50.);
    }
    if keyboard.just_pressed(KeyCode::Digit1) {
        config.enabled ^= true;
    }
    if keyboard.just_pressed(KeyCode::KeyU) {
        config.line.style = match config.line.style {
            GizmoLineStyle::Solid => GizmoLineStyle::Dotted,
            GizmoLineStyle::Dotted => GizmoLineStyle::Dashed {
                gap_scale: 3.0,
                line_scale: 5.0,
            },
            _ => GizmoLineStyle::Solid,
        };
    }
    if keyboard.just_pressed(KeyCode::KeyJ) {
        config.line.joints = match config.line.joints {
            GizmoLineJoint::Bevel => GizmoLineJoint::Miter,
            GizmoLineJoint::Miter => GizmoLineJoint::Round(4),
            GizmoLineJoint::Round(_) => GizmoLineJoint::None,
            GizmoLineJoint::None => GizmoLineJoint::Bevel,
        };
    }

    let (my_config, _) = config_store.config_mut::<MyRoundGizmos>();
    if keyboard.pressed(KeyCode::ArrowUp) {
        my_config.line.width += 5. * real_time.delta_secs();
        my_config.line.width = my_config.line.width.clamp(0., 50.);
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        my_config.line.width -= 5. * real_time.delta_secs();
        my_config.line.width = my_config.line.width.clamp(0., 50.);
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        my_config.enabled ^= true;
    }
    if keyboard.just_pressed(KeyCode::KeyI) {
        my_config.line.style = match my_config.line.style {
            GizmoLineStyle::Solid => GizmoLineStyle::Dotted,
            GizmoLineStyle::Dotted => GizmoLineStyle::Dashed {
                gap_scale: 3.0,
                line_scale: 5.0,
            },
            _ => GizmoLineStyle::Solid,
        };
    }
    if keyboard.just_pressed(KeyCode::KeyK) {
        my_config.line.joints = match my_config.line.joints {
            GizmoLineJoint::Bevel => GizmoLineJoint::Miter,
            GizmoLineJoint::Miter => GizmoLineJoint::Round(4),
            GizmoLineJoint::Round(_) => GizmoLineJoint::None,
            GizmoLineJoint::None => GizmoLineJoint::Bevel,
        };
    }

    if keyboard.just_pressed(KeyCode::KeyB) {
        // AABB gizmos are normally only drawn on entities with a ShowAabbGizmo component
        // We can change this behavior in the configuration of AabbGizmoGroup
        config_store.config_mut::<AabbGizmoConfigGroup>().1.draw_all ^= true;
    }
    if keyboard.just_pressed(KeyCode::Space) {
        virtual_time.toggle();
    }
}
