//! The surface swept by a circle along a curve.

use std::f32::consts::TAU;

use glam::{Vec2, Vec3};
use oneiroi_core::node::{Inputs, NodeDef, Outputs, PortDecl, Shape};

use super::nurbs::CubicNurbsSegmentCache;

/// Position and normal of the tube around `segment` at `u` in `0..=1` along
/// it and `angle` around it.
///
/// This is the whole definition of the surface. Meshing only decides at
/// which `u` and `angle` it gets asked.
pub fn tube_point(
    segment: &CubicNurbsSegmentCache,
    u: f32,
    angle: f32,
    radius: f32,
) -> (Vec3, Vec3) {
    let frame = segment.frame(u);
    let (sin, cos) = angle.sin_cos();
    let normal = frame.normal * cos + frame.binormal * sin;
    (frame.position + normal * radius, normal)
}

/// A triangle mesh of the tube with `rings` steps along every segment and
/// `sides` steps around it. Those two are the level of detail.
pub static TUBE: NodeDef = NodeDef {
    id: "curve.tube",
    display_name: "Tube",
    inputs: &[
        PortDecl::new::<CubicNurbsSegmentCache>("segments"),
        PortDecl::new::<f32>("radius"),
        PortDecl::new::<u32>("rings"),
        PortDecl::new::<u32>("sides"),
    ],
    outputs: &[
        PortDecl::new::<Vec3>("positions"),
        PortDecl::new::<Vec3>("normals"),
        PortDecl::new::<Vec2>("uvs"),
        PortDecl::new::<u32>("indices"),
    ],
    shape: Shape::Expand,
    cpu: tube,
    gpu: &[],
};

fn tube(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let segments = inputs.get::<CubicNurbsSegmentCache>(0);
    let radius = inputs.value::<f32>(1);
    let rings = inputs.value::<u32>(2).max(1) as usize;
    let sides = inputs.value::<u32>(3).max(3) as usize;
    let Some(last) = segments.last() else {
        return;
    };
    let total_length = last.cumulative_length().max(f32::MIN_POSITIVE);

    // Neighbouring segments share the ring between them, and every ring
    // repeats its first vertex so the texture can wrap around.
    let num_rings = segments.len() * rings + 1;
    let ring_vertices = sides + 1;

    let mut positions = Vec::with_capacity(num_rings * ring_vertices);
    let mut normals = Vec::with_capacity(positions.capacity());
    let mut uvs = Vec::with_capacity(positions.capacity());
    for ring in 0..num_rings {
        let index = (ring / rings).min(segments.len() - 1);
        let segment = &segments[index];
        let u = (ring - index * rings) as f32 / rings as f32;
        let distance = segment.cumulative_length() - segment.length() + segment.length_until(u);

        for side in 0..ring_vertices {
            let around = side as f32 / sides as f32;
            let (position, normal) = tube_point(segment, u, around * TAU, radius);
            positions.push(position);
            normals.push(normal);
            uvs.push(Vec2::new(around, distance / total_length));
        }
    }

    let mut indices = Vec::with_capacity((num_rings - 1) * sides * 6);
    for ring in 0..num_rings - 1 {
        for side in 0..sides {
            let a = (ring * ring_vertices + side) as u32;
            let b = a + 1;
            let c = a + ring_vertices as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }

    outputs.set(0, &positions);
    outputs.set(1, &normals);
    outputs.set(2, &uvs);
    outputs.set(3, &indices);
}

#[cfg(test)]
mod tests {
    use glam::Vec4;
    use oneiroi_core::{exec::Engine, program::ProgramBuilder};

    use super::*;
    use crate::curve::{NURBS, nurbs::clamped_uniform_knots};

    fn engine(points: &[[f32; 4]]) -> (Engine, oneiroi_core::exec::Instance) {
        let mut b = ProgramBuilder::new("tube");
        let points = b.constant(points);
        let knots = b.constant(&clamped_uniform_knots(5));
        let radius = b.param::<f32>("radius", &[0.25]);
        let rings = b.param::<u32>("rings", &[4]);
        let sides = b.param::<u32>("sides", &[6]);
        let segments = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let mesh = b.node(&TUBE, &[segments, radius, rings, sides]).unwrap();
        for (name, slot) in ["positions", "normals", "uvs", "indices"].iter().zip(mesh) {
            b.output(*name, slot);
        }

        let mut engine = Engine::default();
        let id = engine.add_program(b.finish());
        let mut instance = engine.instantiate(id);
        engine.evaluate([&mut instance]);
        (engine, instance)
    }

    const BENT: [[f32; 4]; 5] = [
        [0., 0., 0., 1.],
        [1., 2., 0., 1.],
        [2., 0., 1., 1.],
        [3., 2., 0., 1.],
        [4., 0., -1., 1.],
    ];

    #[test]
    fn the_mesh_is_a_closed_grid() {
        let (engine, instance) = engine(&BENT);
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
    }

    #[test]
    fn triangles_face_outwards() {
        let (engine, instance) = engine(&BENT);
        let positions = engine.read::<Vec3>(&instance, "positions").unwrap();
        let normals = engine.read::<Vec3>(&instance, "normals").unwrap();
        let indices = engine.read::<u32>(&instance, "indices").unwrap();

        for triangle in indices.chunks_exact(3) {
            let [a, b, c] = [0, 1, 2].map(|i| triangle[i] as usize);
            let face = (positions[b] - positions[a]).cross(positions[c] - positions[a]);
            assert!(face.dot(normals[a] + normals[b] + normals[c]) > 0.0);
        }
    }

    #[test]
    fn the_surface_keeps_its_radius() {
        let line: Vec<_> = (0..5).map(|i| [i as f32, 0., 0., 1.]).collect();
        let (engine, instance) = engine(&line);
        let positions = engine.read::<Vec3>(&instance, "positions").unwrap();
        for position in positions {
            let on_axis = Vec4::new(position.x, 0., 0., 1.).truncate();
            assert!((position.distance(on_axis) - 0.25).abs() < 1e-4);
        }
    }

    #[test]
    fn detail_only_remeshes() {
        let (mut engine, mut instance) = engine(&BENT);
        engine.set_param(&mut instance, "sides", &[12u32]).unwrap();
        assert_eq!(engine.evaluate([&mut instance]), 1);
        assert_eq!(
            engine.read::<Vec3>(&instance, "positions").unwrap().len(),
            9 * 13
        );
    }
}
