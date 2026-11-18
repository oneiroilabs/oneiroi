//! The surface a profile leaves behind when it is moved along a curve.

use std::{f32::consts::TAU, marker::PhantomData};

use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3, Vec3Swizzles};
use oneiroi_core::{
    node::{Caps, GenericPort, GpuKernel, GpuOutput, Inputs, NodeDef, NodeRegistry, Outputs, PortDecl},
    program::{Program, ProgramBuilder},
    types::{DataType, Field, FieldType, Layout, Scalar},
};

use super::{
    nurbs::CubicNurbsSegmentCache,
    profile::{PROFILE, Profile},
};
use crate::{
    generic::version,
    primitive::CIRCLE,
    surface::{self, SURFACE, Surface, register_surface},
};

/// A circle swept along one segment of a curve.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct TubePatch {
    segment: CubicNurbsSegmentCache,
    radius: f32,
    /// Where along the whole curve the segment starts and ends.
    along: [f32; 2],
    _padding: f32,
}

impl TubePatch {
    pub fn new(segment: CubicNurbsSegmentCache, radius: f32, along: [f32; 2]) -> Self {
        Self {
            segment,
            radius,
            along,
            _padding: 0.0,
        }
    }
}

impl DataType for TubePatch {
    type ConfigurationOptions = ();

    const NAME: &'static str = "tube_patch";
    const WGSL: &'static str = "TubePatch";
    const SCALAR: Scalar = Scalar::F32;
    const LAYOUT: Layout = Layout::Struct(&[
        Field::nested("segment", CubicNurbsSegmentCache::INFO),
        Field::new("radius", FieldType::F32),
        Field::new("along", FieldType::Vec2),
        Field::pad(1),
    ]);
}

describe_type! {
    /// A tube around a piece of a curve, what a circle sweeps out.
    TubePatch { label: "Tube", hue: Emerald }
}

impl Surface for TubePatch {
    fn point(&self, u: f32, v: f32) -> (Vec3, Vec3) {
        let frame = self.segment.frame(u);
        let around = Vec2::from_angle(v * TAU);
        let normal = frame.normal * around.x + frame.binormal * around.y;
        (frame.position + normal * self.radius, normal)
    }

    fn uv(&self, u: f32, v: f32) -> Vec2 {
        Vec2::new(v, self.along[0] + (self.along[1] - self.along[0]) * u)
    }

    const WGSL: &'static str = concat!(include_str!("curve.wgsl"), include_str!("tube.wgsl"));
}

/// One segment of a profile swept along one segment of a curve.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct SweptPatch {
    spine: CubicNurbsSegmentCache,
    profile: CubicNurbsSegmentCache,
    /// Where along the whole curve the segment of the spine starts and ends.
    along: [f32; 2],
    /// The same for the segment of the profile.
    around: [f32; 2],
}

impl SweptPatch {
    pub fn new(
        spine: CubicNurbsSegmentCache,
        profile: CubicNurbsSegmentCache,
        along: [f32; 2],
        around: [f32; 2],
    ) -> Self {
        Self {
            spine,
            profile,
            along,
            around,
        }
    }
}

impl DataType for SweptPatch {
    type ConfigurationOptions = ();

    const NAME: &'static str = "swept_patch";
    const WGSL: &'static str = "SweptPatch";
    const SCALAR: Scalar = Scalar::F32;
    const LAYOUT: Layout = Layout::Struct(&[
        Field::nested("spine", CubicNurbsSegmentCache::INFO),
        Field::nested("profile", CubicNurbsSegmentCache::INFO),
        Field::new("along", FieldType::Vec2),
        Field::new("around", FieldType::Vec2),
    ]);
}

describe_type! {
    /// A piece of the surface a curve sweeps out along another one.
    SweptPatch { label: "Swept surface", hue: Teal }
}

impl Surface for SweptPatch {
    fn point(&self, u: f32, v: f32) -> (Vec3, Vec3) {
        let frame = self.spine.frame(u);
        let (outline, _, tangent) = self.profile.evaluate(v);
        let tangent = tangent.xy().normalize_or_zero();
        // To the right of the direction the profile runs in.
        let outwards = Vec2::new(tangent.y, -tangent.x);
        (
            frame.position + frame.normal * outline.x + frame.binormal * outline.y,
            frame.normal * outwards.x + frame.binormal * outwards.y,
        )
    }

    fn uv(&self, u: f32, v: f32) -> Vec2 {
        Vec2::new(
            self.around[0] + (self.around[1] - self.around[0]) * v,
            self.along[0] + (self.along[1] - self.along[0]) * u,
        )
    }

    const WGSL: &'static str = concat!(include_str!("curve.wgsl"), include_str!("swept.wgsl"));
}

generic_node! {
    /// The surface a profile leaves behind when it is moved along a curve.
    ///
    /// The profile stays upright to the curve and twists as little as it can.
    /// It has to run counter-clockwise for the surface to face outwards.
    pub static SWEEP = "curve.sweep" {
        label: "Sweep",
        category: "Curve",
        inputs {
            /// The curves to move the profile along, a surface for each.
            port GenericPort::exact::<CubicNurbsSegmentCache>("spine").grouped();
            /// The cross-section, in the plane of `x` and `y`: `x` points along the
            /// normal of the curve, `y` along its binormal.
            port GenericPort::implementing("profile", PROFILE);
        }
        outputs {
            /// The swept surfaces, one per curve of the spine.
            port GenericPort::implementing("surface", SURFACE).grouped();
        }
    }
}

struct Ports<P>(PhantomData<P>);

impl<P: Profile> Ports<P> {
    const INPUTS: &'static [PortDecl] = &[
        PortDecl::new::<CubicNurbsSegmentCache>("spine").grouped(),
        PortDecl::new::<P>("profile"),
    ];
    const OUTPUTS: &'static [PortDecl] = &[PortDecl::new::<P::Patch>("surface").grouped()];
    const GPU: &'static [GpuKernel] = &[GpuKernel {
        wgsl: P::SWEEP_WGSL,
        outputs: &[
            GpuOutput {
                entry: "sweep_patch",
                len: |inputs| P::patches(inputs.len(0), inputs.len(1)),
                words: None,
            },
            GpuOutput {
                entry: "sweep_offset",
                len: |inputs| inputs.group_count(0) + 1,
                // From where the curves of the spine start, which the CPU
                // has unless the spine came of a kernel which did not say.
                words: Some(|inputs| {
                    let mut offsets = vec![0];
                    for curve in inputs.groups(0) {
                        let patches = P::patches(curve.len(), inputs.len(1)) as u32;
                        offsets.push(offsets.last().unwrap() + patches);
                    }
                    offsets
                }),
            },
        ],
        requires: Caps::NONE,
        // Where the curves of the spine start.
        reads: &[2],
        accepts: None,
        examples: P::SWEEP_EXAMPLES,
        stages: &[],
    }];
}

/// Makes `P` usable wherever a profile is asked for, by registering the
/// version of every node that takes one for it. The surface it sweeps out
/// is registered along with it.
pub fn register_profile<P: Profile>(registry: &mut NodeRegistry) {
    let node = version(&SWEEP, P::NAME, |id| NodeDef {
        id,
        inputs: Ports::<P>::INPUTS,
        outputs: Ports::<P>::OUTPUTS,
        version: 1,
        cpu: sweep::<P>,
        gpu: Ports::<P>::GPU,
    });
    registry.register_instance(&SWEEP, node);
    register_surface::<P::Patch>(registry);
}

/// Every curve of the spine is swept on its own, into a group of patches.
fn sweep<P: Profile>(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let spine = inputs.get::<CubicNurbsSegmentCache>(0);
    let profile = inputs.get::<P>(1);
    let (patches, offsets) = crate::parallel::per_group(inputs.groups(0), |group, curve| {
        P::sweep(&spine[curve], P::select(profile, group))
    });
    outputs.set(0, &patches);
    outputs.set_groups(0, &offsets);
}

/// A tube is not a node of its own but this graph: a circle swept along
/// the `spine`, and a mesh of it for whoever needs triangles. Its
/// parameters are `spine`, `radius` and the `tolerance` of the mesh, its
/// outputs the `surface` and the ones of [`MESH`](surface::MESH).
pub fn tube(registry: &NodeRegistry) -> Program {
    let mut b = ProgramBuilder::new("tube");
    let spine = b.param::<CubicNurbsSegmentCache>("spine", &[]);
    let radius = b.param::<f32>("radius", &[0.2]);
    let tolerance = b.param_one_of::<f32>("tolerance", surface::TOLERANCE);

    let circle = b.node(&CIRCLE, &[radius]).expect("a radius is a f32")[0];
    let swept = b
        .generic(registry, &SWEEP, &[spine, circle])
        .expect("circles are registered as profiles")[0];
    let mesh = surface::mesh(&mut b, registry, swept, tolerance)
        .expect("tubes are registered as surfaces");
    b.output("surface", swept);
    for (name, slot) in mesh {
        b.output(name, slot);
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

    use oneiroi_core::types::DataType;

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
        let tolerance = b.param_one_of::<f32>("tolerance", 0.01);
        let segments = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let outputs = b
            .call(
                engine.programs(),
                tube,
                &[Some(segments), Some(radius), Some(tolerance)],
            )
            .unwrap();
        let names = engine.programs().get(tube).outputs.iter().map(|o| o.name.clone());
        for (name, slot) in names.collect::<Vec<_>>().into_iter().zip(outputs) {
            b.output(name, slot);
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

    /// How fine the mesh of the tube is.
    fn tube_detail(engine: &Engine, instance: &Instance) -> (usize, usize) {
        let patches = engine.read::<TubePatch>(instance, "surface").unwrap();
        let tolerance = engine.read::<f32>(instance, "tolerance");
        surface::detail(patches, tolerance.map_or(0.01, |t| t[0]))
    }

    #[test]
    fn a_tube_is_a_grid_per_segment() {
        let (engine, instance) = tube_on(&BENT);
        let positions = engine.read::<Vec3>(&instance, "positions").unwrap();
        let normals = engine.read::<Vec3>(&instance, "normals").unwrap();
        let uvs = engine.read::<Vec2>(&instance, "uvs").unwrap();
        let indices = engine.read::<u32>(&instance, "indices").unwrap();

        // 2 segments, each with rings + 1 rings of sides + 1 vertices.
        let (rings, sides) = tube_detail(&engine, &instance);
        let ring = sides + 1;
        assert!(rings > 1 && sides > 8, "{rings} {sides}");
        assert_eq!(positions.len(), 2 * (rings + 1) * ring);
        assert_eq!(indices.len(), 2 * rings * sides * 6);
        assert!(indices.iter().all(|i| (*i as usize) < positions.len()));

        // The seam closes and the texture runs once along and around.
        for ring in positions.chunks_exact(ring) {
            assert!(ring[0].distance(ring[sides]) < 1e-4);
        }
        assert_eq!(uvs[0], Vec2::ZERO);
        assert!(uvs[uvs.len() - 1].distance(Vec2::ONE) < 1e-4);

        // Where the two segments meet they agree on everything, so the
        // tube is smooth there although they share no vertices.
        for side in 0..ring {
            let (end, start) = (rings * ring + side, (rings + 1) * ring + side);
            assert!(positions[end].distance(positions[start]) < 1e-4);
            assert!(normals[end].distance(normals[start]) < 1e-3);
            assert!(uvs[end].distance(uvs[start]) < 1e-4);
        }

        assert_faces_outwards(&engine, &instance);
    }

    #[test]
    fn the_mesh_stays_within_the_tolerance() {
        for tolerance in [0.05, 0.01, 0.002] {
            let (mut engine, mut instance) = tube_on(&BENT);
            engine.set_param(&mut instance, "tolerance", &[tolerance]).unwrap();
            engine.evaluate([&mut instance]);
            let patches = engine.read::<TubePatch>(&instance, "surface").unwrap();
            let (rings, sides) = surface::detail(patches, tolerance);
            let positions = engine.read::<Vec3>(&instance, "positions").unwrap();

            // The middle of every quad of the mesh against the surface.
            let mut worst = 0.0f32;
            for (index, patch) in patches.iter().enumerate() {
                let grid = &positions[index * (rings + 1) * (sides + 1)..];
                for ring in 0..rings {
                    for side in 0..sides {
                        let at = |r: usize, s: usize| grid[(ring + r) * (sides + 1) + side + s];
                        // Both triangles share the diagonal between these.
                        let diagonal = (at(0, 1) + at(1, 0)) * 0.5;
                        let u = (ring as f32 + 0.5) / rings as f32;
                        let v = (side as f32 + 0.5) / sides as f32;
                        worst = worst.max(diagonal.distance(patch.point(u, v).0));
                    }
                }
            }
            assert!(worst <= tolerance, "{worst} is further than {tolerance}");
        }
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
    fn detail_only_meshes_again() {
        let (mut engine, mut instance) = tube_on(&BENT);
        // A new circle, a new surface and a new mesh, but the same curve.
        engine.set_param(&mut instance, "radius", &[0.5f32]).unwrap();
        assert_eq!(engine.evaluate([&mut instance]), 3);

        // The surface is what it was, only its mesh gets finer.
        let coarse = engine.read::<Vec3>(&instance, "positions").unwrap().len();
        engine.set_param(&mut instance, "tolerance", &[0.001f32]).unwrap();
        assert_eq!(engine.evaluate([&mut instance]), 1);
        assert!(engine.read::<Vec3>(&instance, "positions").unwrap().len() > coarse);
    }

    #[test]
    fn a_surface_is_small() {
        let registry = registry();
        let mut b = ProgramBuilder::new("surface");
        let points = b.constant(&BENT);
        let knots = b.constant(&clamped_uniform_knots(5));
        let radius = b.constant::<f32>(&[0.25]);
        let spine = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let circle = b.node(&CIRCLE, &[radius]).unwrap()[0];
        let surface = b.generic(&registry, &SWEEP, &[spine, circle]).unwrap()[0];
        b.output("surface", surface);

        let mut engine = Engine::default();
        let id = engine.add_program(b.finish());
        let mut instance = engine.instantiate(id);
        engine.evaluate([&mut instance]);

        // One patch per segment of the curve, whatever it is drawn with.
        let patches = engine.read::<TubePatch>(&instance, "surface").unwrap();
        assert_eq!(patches.len(), 2);
        assert_eq!(size_of::<TubePatch>(), 28 * 4);
        let (position, normal) = patches[0].point(0.0, 0.25);
        assert!(position.distance(Vec3::ZERO) - 0.25 < 1e-4);
        assert!((normal.length() - 1.0).abs() < 1e-4);
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
        let surface = b.generic(&registry, &SWEEP, &[spine, profile]).unwrap()[0];
        let tolerance = b.constant::<f32>(&[0.01]);
        b.output("surface", surface);
        for (name, slot) in surface::mesh(&mut b, &registry, surface, tolerance).unwrap() {
            b.output(name, slot);
        }

        let mut engine = Engine::default();
        let id = engine.add_program(b.finish());
        let mut instance = engine.instantiate(id);
        engine.evaluate([&mut instance]);

        // A patch for each of the 4 edges along each of the 2 segments.
        assert_eq!(engine.read::<SweptPatch>(&instance, "surface").unwrap().len(), 8);
        let patches = engine.read::<SweptPatch>(&instance, "surface").unwrap();
        let (rings, sides) = surface::detail(patches, 0.01);
        let positions = engine.read::<Vec3>(&instance, "positions").unwrap();
        let normals = engine.read::<Vec3>(&instance, "normals").unwrap();
        // The edges are straight, so across them a single step is enough.
        assert_eq!(sides, 1);
        let per = (rings + 1) * (sides + 1);
        assert_eq!(positions.len(), 8 * per);
        assert_eq!(engine.read::<u32>(&instance, "indices").unwrap().len(), 8 * rings * 6);

        // A corner is one position with the normals of both edges: the
        // end of the first edge and the start of the second one.
        assert!(positions[sides].distance(positions[per]) < 1e-4);
        assert!(normals[sides].dot(normals[per]).abs() < 1e-3);
        // Along an edge the normal does not change.
        assert!(normals[0].distance(normals[sides]) < 1e-3);

        assert_faces_outwards(&engine, &instance);
    }

    #[test]
    fn every_curve_is_swept_on_its_own() {
        // Two curves in one list of points, told apart by their groups:
        // a straight one along x, and the bent one far away from it.
        let mut points: Vec<[f32; 4]> = (0..4).map(|i| [i as f32, 0., 0., 1.]).collect();
        points.extend(BENT.iter().map(|[x, y, z, w]| [*x, y + 10.0, *z, *w]));

        let mut b = ProgramBuilder::new("two curves");
        let points = b.param_grouped("points", <[f32; 4]>::INFO, bytemuck::cast_slice(&points));
        let knots = b.constant::<f32>(&[]);
        let spine = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let radius = b.constant::<f32>(&[0.25, 0.5]);
        let circle = b.node(&CIRCLE, &[radius]).unwrap()[0];
        let surface = b.generic(&registry(), &SWEEP, &[spine, circle]).unwrap()[0];
        b.output("spine", spine);
        b.output("surface", surface);

        let mut engine = Engine::default();
        let id = engine.add_program(b.finish());
        let mut instance = engine.instantiate(id);
        engine.set_param(&mut instance, "points#groups", &[0u32, 4, 9]).unwrap();
        engine.evaluate([&mut instance]);

        // A segment for the first curve and two for the second, each
        // curve a group of its own all the way through.
        assert_eq!(engine.groups(&instance, "spine"), Some(&[0, 1, 3][..]));
        assert_eq!(engine.groups(&instance, "surface"), Some(&[0, 1, 3][..]));
        let patches = engine.read::<TubePatch>(&instance, "surface").unwrap();
        assert_eq!(patches.len(), 3);

        // Each curve gets its own circle, and the texture runs along each
        // curve from its start to its end.
        let radius = |patch: &TubePatch| {
            let (position, _) = patch.point(0.5, 0.0);
            position.distance(patch.point(0.5, 0.5).0) / 2.0
        };
        assert!((radius(&patches[0]) - 0.25).abs() < 1e-3);
        assert!((radius(&patches[1]) - 0.5).abs() < 1e-3);
        assert_eq!(patches[0].uv(1.0, 0.0).y, 1.0);
        assert_eq!(patches[1].uv(0.0, 0.0).y, 0.0);
        assert!((patches[2].uv(1.0, 0.0).y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn only_profiles_can_be_swept() {
        let registry = registry();
        let mut b = ProgramBuilder::new("broken");
        let spine = b.param::<CubicNurbsSegmentCache>("spine", &[]);
        let not_a_profile = b.constant(&[Vec3::ONE]);
        assert_eq!(
            b.generic(&registry, &SWEEP, &[spine, not_a_profile]),
            Err(BuildError::Trait {
                target: "curve.sweep".into(),
                port: "profile".into(),
                expected: "profile",
                found: "vec3",
            })
        );
        // And only surfaces can be turned into a mesh.
        let tolerance = b.constant::<f32>(&[0.01]);
        assert!(matches!(
            surface::mesh(&mut b, &registry, spine, tolerance),
            Err(BuildError::Trait { .. })
        ));

        let names = |tr| {
            let mut names: Vec<_> = registry.implementors(tr).iter().map(|ty| ty.name).collect();
            names.sort();
            names
        };
        assert_eq!(names(PROFILE), ["circle", "cubic_nurbs_segment"]);
        assert_eq!(names(SURFACE), ["swept_patch", "tube_patch"]);
    }
}
