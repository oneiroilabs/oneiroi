//! The surface a profile leaves behind when it is moved along a curve.

use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{LazyLock, Mutex},
};

use glam::{Vec2, Vec3};
use oneiroi_core::{
    node::{GenericNode, GenericPort, Inputs, NodeDef, NodeRegistry, Outputs, PortDecl, Shape},
    program::{Program, ProgramBuilder},
};

use super::{
    nurbs::CubicNurbsSegmentCache,
    profile::{PROFILE, Profile},
};
use crate::primitive::CIRCLE;

/// Position and normal of the swept surface at `u` in `0..=1` along
/// `segment` and `v` in `0..=1` along the profile.
///
/// This is the whole definition of the surface. Meshing only decides at
/// which `u` and `v` it gets asked.
pub fn sweep_point<P: Profile>(
    segment: &CubicNurbsSegmentCache,
    u: f32,
    profile: &[P],
    v: f32,
) -> (Vec3, Vec3) {
    let frame = segment.frame(u);
    let (position, normal) = P::point(profile, v);
    (
        frame.position + frame.normal * position.x + frame.binormal * position.y,
        frame.normal * normal.x + frame.binormal * normal.y,
    )
}

/// A triangle mesh of the swept surface with `rings` steps along every
/// segment of the spine and `sides` steps along the profile. Those two are
/// the level of detail.
///
/// The profile has to run counter-clockwise for the surface to face
/// outwards.
pub static SWEEP: GenericNode = GenericNode {
    id: "curve.sweep",
    display_name: "Sweep",
    inputs: &[
        GenericPort::exact::<CubicNurbsSegmentCache>("spine"),
        GenericPort::implementing("profile", PROFILE),
        GenericPort::exact::<u32>("rings"),
        GenericPort::exact::<u32>("sides"),
    ],
    outputs: &[
        GenericPort::exact::<Vec3>("positions"),
        GenericPort::exact::<Vec3>("normals"),
        GenericPort::exact::<Vec2>("uvs"),
        GenericPort::exact::<u32>("indices"),
    ],
};

struct Ports<P>(PhantomData<P>);

impl<P: Profile> Ports<P> {
    const INPUTS: &'static [PortDecl] = &[
        PortDecl::new::<CubicNurbsSegmentCache>("spine"),
        PortDecl::new::<P>("profile"),
        PortDecl::new::<u32>("rings"),
        PortDecl::new::<u32>("sides"),
    ];
}

const OUTPUTS: &[PortDecl] = &[
    PortDecl::new::<Vec3>("positions"),
    PortDecl::new::<Vec3>("normals"),
    PortDecl::new::<Vec2>("uvs"),
    PortDecl::new::<u32>("indices"),
];

/// Makes `P` usable wherever a profile is asked for, by registering the
/// version of every node that takes one for it.
pub fn register_profile<P: Profile>(registry: &mut NodeRegistry) {
    // A version is made once and then lives as long as the program does,
    // like the nodes which are written out as statics.
    static VERSIONS: LazyLock<Mutex<HashMap<String, &'static NodeDef>>> =
        LazyLock::new(Mutex::default);

    let id = format!("{}<{}>", SWEEP.id, P::NAME);
    let mut versions = VERSIONS.lock().unwrap();
    let node = *versions.entry(id.clone()).or_insert_with(|| {
        Box::leak(Box::new(NodeDef {
            id: id.leak(),
            display_name: SWEEP.display_name,
            inputs: Ports::<P>::INPUTS,
            outputs: OUTPUTS,
            shape: Shape::Expand,
            cpu: sweep::<P>,
            gpu: &[],
        }))
    });
    registry.register_instance(&SWEEP, node);
}

struct RingVertex {
    v: f32,
    position: Vec2,
    normal: Vec2,
    /// Whether a quad spans from this vertex to the next one.
    connected: bool,
}

/// The vertices of one ring. A corner of the profile gets two of them, one
/// with the normal of either side.
fn ring<P: Profile>(profile: &[P], sides: usize) -> Vec<RingVertex> {
    const INSET: f32 = 1e-4;

    let mut bounds = vec![0.0];
    bounds.extend(P::breaks(profile).into_iter().filter(|v| *v > 0.0 && *v < 1.0));
    bounds.push(1.0);

    let mut vertices = Vec::new();
    for (index, span) in bounds.windows(2).enumerate() {
        let (start, end) = (span[0], span[1]);
        let steps = ((sides as f32 * (end - start)).round() as usize).max(1);
        for step in 0..=steps {
            let v = start + (end - start) * step as f32 / steps as f32;
            // Next to a corner the normal is the one of this side of it.
            let mut inside = v;
            if index > 0 {
                inside = inside.max(start + INSET);
            }
            if index + 2 < bounds.len() {
                inside = inside.min(end - INSET);
            }
            vertices.push(RingVertex {
                v,
                position: P::point(profile, v).0,
                normal: P::point(profile, inside).1,
                connected: step < steps,
            });
        }
    }
    vertices
}

fn sweep<P: Profile>(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let segments = inputs.get::<CubicNurbsSegmentCache>(0);
    let profile = inputs.get::<P>(1);
    let rings = inputs.value::<u32>(2).max(1) as usize;
    let sides = inputs.value::<u32>(3).max(3) as usize;
    let Some(last) = segments.last() else {
        return;
    };
    if profile.is_empty() {
        return;
    }
    let total_length = last.cumulative_length().max(f32::MIN_POSITIVE);

    // Neighbouring segments share the ring between them.
    let ring = ring(profile, sides);
    let num_rings = segments.len() * rings + 1;

    let mut positions = Vec::with_capacity(num_rings * ring.len());
    let mut normals = Vec::with_capacity(positions.capacity());
    let mut uvs = Vec::with_capacity(positions.capacity());
    for index in 0..num_rings {
        let segment_index = (index / rings).min(segments.len() - 1);
        let segment = &segments[segment_index];
        let u = (index - segment_index * rings) as f32 / rings as f32;
        let distance = segment.cumulative_length() - segment.length() + segment.length_until(u);
        let frame = segment.frame(u);

        for vertex in &ring {
            positions.push(
                frame.position
                    + frame.normal * vertex.position.x
                    + frame.binormal * vertex.position.y,
            );
            normals.push(frame.normal * vertex.normal.x + frame.binormal * vertex.normal.y);
            uvs.push(Vec2::new(vertex.v, distance / total_length));
        }
    }

    let mut indices = Vec::new();
    for index in 0..num_rings - 1 {
        for (side, vertex) in ring.iter().enumerate() {
            if !vertex.connected {
                continue;
            }
            let a = (index * ring.len() + side) as u32;
            let b = a + 1;
            let c = a + ring.len() as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }

    outputs.set(0, &positions);
    outputs.set(1, &normals);
    outputs.set(2, &uvs);
    outputs.set(3, &indices);
}

/// A tube is not a node of its own but this graph: a circle swept along
/// the `spine`. Its parameters are `spine`, `radius`, `rings` and `sides`,
/// its outputs the ones of [`SWEEP`].
pub fn tube(registry: &NodeRegistry) -> Program {
    let mut b = ProgramBuilder::new("tube");
    let spine = b.param::<CubicNurbsSegmentCache>("spine", &[]);
    let radius = b.param::<f32>("radius", &[0.2]);
    let rings = b.param::<u32>("rings", &[16]);
    let sides = b.param::<u32>("sides", &[24]);

    let circle = b.node(&CIRCLE, &[radius]).expect("a radius is a f32")[0];
    let mesh = b
        .generic(registry, &SWEEP, &[spine, circle, rings, sides])
        .expect("circles are registered as profiles");
    for (port, slot) in SWEEP.outputs.iter().zip(mesh) {
        b.output(port.name, slot);
    }
    b.finish()
}

#[cfg(test)]
mod tests {
    use glam::Vec4;
    use oneiroi_core::{
        exec::{Engine, Instance},
        program::BuildError,
    };

    use super::*;
    use crate::curve::{NURBS, nurbs::clamped_uniform_knots};

    const BENT: [[f32; 4]; 5] = [
        [0., 0., 0., 1.],
        [1., 2., 0., 1.],
        [2., 0., 1., 1.],
        [3., 2., 0., 1.],
        [4., 0., -1., 1.],
    ];

    fn registry() -> NodeRegistry {
        let mut registry = NodeRegistry::new();
        crate::register(&mut registry);
        registry
    }

    /// A spine with the tube graph called on it.
    fn tube_on(spine: &[[f32; 4]]) -> (Engine, Instance) {
        let mut engine = Engine::default();
        let tube = engine.add_program(tube(&registry()));

        let mut b = ProgramBuilder::new("test");
        let points = b.constant(spine);
        let knots = b.constant(&clamped_uniform_knots(spine.len()));
        let radius = b.param::<f32>("radius", &[0.25]);
        let rings = b.constant::<u32>(&[4]);
        let sides = b.param::<u32>("sides", &[6]);
        let segments = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let mesh = b
            .call(
                engine.programs(),
                tube,
                &[Some(segments), Some(radius), Some(rings), Some(sides)],
            )
            .unwrap();
        for (port, slot) in SWEEP.outputs.iter().zip(mesh) {
            b.output(port.name, slot);
        }

        let id = engine.add_program(b.finish());
        let mut instance = engine.instantiate(id);
        engine.evaluate([&mut instance]);
        (engine, instance)
    }

    /// A square with sharp corners as a NURBS curve, counter-clockwise.
    fn square() -> (Vec<[f32; 4]>, Vec<f32>) {
        // Small enough to fit through the bends of the spine.
        let corners = [(0.2, -0.2), (0.2, 0.2), (-0.2, 0.2), (-0.2, -0.2), (0.2, -0.2)];
        let mut points = vec![[0.2, -0.2, 0., 1.]];
        for pair in corners.windows(2) {
            let (a, b) = (Vec2::from(pair[0]), Vec2::from(pair[1]));
            for step in 1..=3 {
                let point = a.lerp(b, step as f32 / 3.0);
                points.push([point.x, point.y, 0., 1.]);
            }
        }
        let mut knots = vec![0.0; 4];
        for knot in [0.25, 0.5, 0.75] {
            knots.extend([knot; 3]);
        }
        knots.extend([1.0; 4]);
        (points, knots)
    }

    fn assert_faces_outwards(engine: &Engine, instance: &Instance) {
        let positions = engine.read::<Vec3>(instance, "positions").unwrap();
        let normals = engine.read::<Vec3>(instance, "normals").unwrap();
        let indices = engine.read::<u32>(instance, "indices").unwrap();
        assert!(!indices.is_empty());

        for triangle in indices.chunks_exact(3) {
            let [a, b, c] = [0, 1, 2].map(|i| triangle[i] as usize);
            let face = (positions[b] - positions[a]).cross(positions[c] - positions[a]);
            assert!(face.dot(normals[a] + normals[b] + normals[c]) > 0.0);
        }
    }

    #[test]
    fn a_tube_is_a_closed_grid() {
        let (engine, instance) = tube_on(&BENT);
        let positions = engine.read::<Vec3>(&instance, "positions").unwrap();
        let uvs = engine.read::<Vec2>(&instance, "uvs").unwrap();
        let indices = engine.read::<u32>(&instance, "indices").unwrap();

        // 2 segments with 4 steps each share a ring: 9 rings of 6 + 1.
        assert_eq!(positions.len(), 9 * 7);
        assert_eq!(indices.len(), 8 * 6 * 6);
        assert!(indices.iter().all(|i| (*i as usize) < positions.len()));

        // The seam closes and the texture runs once along and around.
        for ring in positions.chunks_exact(7) {
            assert!(ring[0].distance(ring[6]) < 1e-4);
        }
        assert_eq!(uvs[0], Vec2::ZERO);
        assert!(uvs[uvs.len() - 1].distance(Vec2::ONE) < 1e-4);

        assert_faces_outwards(&engine, &instance);
    }

    #[test]
    fn a_tube_keeps_its_radius() {
        let line: Vec<_> = (0..5).map(|i| [i as f32, 0., 0., 1.]).collect();
        let (engine, instance) = tube_on(&line);
        for position in engine.read::<Vec3>(&instance, "positions").unwrap() {
            let on_axis = Vec4::new(position.x, 0., 0., 1.).truncate();
            assert!((position.distance(on_axis) - 0.25).abs() < 1e-4);
        }
    }

    #[test]
    fn the_radius_only_touches_the_circle_and_the_mesh() {
        let (mut engine, mut instance) = tube_on(&BENT);
        engine.set_param(&mut instance, "radius", &[0.5f32]).unwrap();
        assert_eq!(engine.evaluate([&mut instance]), 2);

        engine.set_param(&mut instance, "sides", &[12u32]).unwrap();
        assert_eq!(engine.evaluate([&mut instance]), 1);
        assert_eq!(
            engine.read::<Vec3>(&instance, "positions").unwrap().len(),
            9 * 13
        );
    }

    #[test]
    fn a_curve_is_a_profile_too() {
        let registry = registry();
        let (square_points, square_knots) = square();

        let mut b = ProgramBuilder::new("beam");
        let points = b.constant(&BENT);
        let knots = b.constant(&clamped_uniform_knots(5));
        let spine = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let points = b.constant(&square_points);
        let knots = b.constant(&square_knots);
        let profile = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let rings = b.constant::<u32>(&[4]);
        let sides = b.constant::<u32>(&[8]);
        let mesh = b
            .generic(&registry, &SWEEP, &[spine, profile, rings, sides])
            .unwrap();
        for (port, slot) in SWEEP.outputs.iter().zip(mesh) {
            b.output(port.name, slot);
        }

        let mut engine = Engine::default();
        let id = engine.add_program(b.finish());
        let mut instance = engine.instantiate(id);
        engine.evaluate([&mut instance]);

        // Each of the 4 edges gets 2 steps and its own two corner vertices.
        let positions = engine.read::<Vec3>(&instance, "positions").unwrap();
        let normals = engine.read::<Vec3>(&instance, "normals").unwrap();
        assert_eq!(positions.len(), 9 * 4 * 3);
        assert_eq!(engine.read::<u32>(&instance, "indices").unwrap().len(), 8 * 8 * 6);

        // A corner is one position with the normals of both edges.
        assert!(positions[2].distance(positions[3]) < 1e-4);
        assert!(normals[2].dot(normals[3]).abs() < 1e-3);
        for edge in normals[..12].chunks_exact(3) {
            assert!(edge[0].distance(edge[1]) < 1e-3 && edge[1].distance(edge[2]) < 1e-3);
        }

        assert_faces_outwards(&engine, &instance);
    }

    #[test]
    fn only_profiles_can_be_swept() {
        let registry = registry();
        let mut b = ProgramBuilder::new("broken");
        let spine = b.param::<CubicNurbsSegmentCache>("spine", &[]);
        let not_a_profile = b.constant(&[Vec3::ONE]);
        let count = b.constant::<u32>(&[4]);
        assert_eq!(
            b.generic(&registry, &SWEEP, &[spine, not_a_profile, count, count]),
            Err(BuildError::Trait {
                target: "curve.sweep".into(),
                port: "profile".into(),
                expected: "profile",
                found: "vec3",
            })
        );

        let mut names: Vec<_> = registry
            .implementors(PROFILE)
            .iter()
            .map(|ty| ty.name)
            .collect();
        names.sort();
        assert_eq!(names, ["circle", "cubic_nurbs_segment"]);
    }
}
