//! Surfaces which are kept as what they are instead of as triangles.

use std::marker::PhantomData;

use glam::{Vec2, Vec3};
use oneiroi_core::{
    node::{
        Drawable, GenericNode, GenericPort, Inputs, NodeDef, NodeRegistry, Outputs, PortDecl, Topology,
    },
    program::{BuildError, ProgramBuilder, SlotId},
    types::{DataType, PortDefault, Trait},
};

use crate::generic::version;

/// Everything which can be turned into a mesh implements this trait.
pub const SURFACE: Trait = Trait { name: "surface" };

/// One patch of a surface: a rectangle of parameters `u` and `v`, both in
/// `0..=1`, with a point of the surface for each of them.
///
/// A surface is a slice of patches like any other value. It is small and
/// exact, and how fine it gets meshed is only decided where it is drawn
/// or turned into a [mesh](MESH).
pub trait Surface: DataType {
    /// Position and normal at `u` along and `v` across the patch.
    fn point(&self, u: f32, v: f32) -> (Vec3, Vec3);

    /// The texture coordinate at the same place.
    fn uv(&self, u: f32, v: f32) -> Vec2 {
        Vec2::new(v, u)
    }

    /// [`Surface::point`] for the GPU, which draws a surface without it
    /// ever becoming a mesh:
    /// `fn surface_point(piece: Self, uv: vec2<f32>) -> SurfacePoint`,
    /// where a `SurfacePoint` is a `position` and a `normal`. Both have to
    /// compute the same thing.
    const WGSL: &'static str;
}

/// A sphere around the patches in `words`, found by looking at a few
/// points of each of them.
fn bounds<S: Surface>(words: &[u32]) -> Option<([f32; 3], f32)> {
    const STEPS: usize = 4;

    let patches: &[S] = bytemuck::try_cast_slice(words).ok()?;
    let points = patches.iter().flat_map(|patch| {
        (0..=STEPS).flat_map(move |ring| {
            (0..=STEPS).map(move |side| {
                patch
                    .point(ring as f32 / STEPS as f32, side as f32 / STEPS as f32)
                    .0
            })
        })
    });
    let (min, max) = points.fold((Vec3::MAX, Vec3::MIN), |(min, max), point| {
        (min.min(point), max.max(point))
    });
    (min.x <= max.x).then(|| (((min + max) * 0.5).to_array(), (max - min).length() * 0.5))
}

/// How far the triangles of a mesh may be from the surface it is made of,
/// unless told otherwise.
pub const TOLERANCE: f32 = 0.001;

/// No more steps than these along or across a patch, however small the
/// tolerance.
pub const MAX_STEPS: usize = 128;

/// A triangle mesh of a surface, as fine as it has to be for no triangle to
/// be further than `tolerance` from the surface.
///
/// This is no node an artist places. Surfaces are drawn as they are, and
/// what needs triangles, like a host which exports or renders meshes, puts
/// it in front of itself with [`mesh`]. Each group of patches is a surface
/// of its own, and all of its patches get the same detail so that where
/// they meet their edges line up.
pub static MESH: GenericNode = GenericNode {
    id: "surface.mesh",
    display_name: "Mesh",
    inputs: &[
        GenericPort::implementing("surface", SURFACE).grouped(),
        GenericPort::exact::<f32>("tolerance")
            .with_default(PortDefault::F32(&[TOLERANCE]))
            .one(),
    ],
    outputs: &[
        GenericPort::exact::<Vec3>("positions"),
        GenericPort::exact::<Vec3>("normals"),
        GenericPort::exact::<Vec2>("uvs"),
        GenericPort::exact::<u32>("indices"),
    ],
};

/// The outputs of [`MESH`] of `surface`, under the names of its ports. What
/// a host calls which needs triangles of a surface.
pub fn mesh(
    builder: &mut ProgramBuilder,
    registry: &NodeRegistry,
    surface: SlotId,
    tolerance: SlotId,
) -> Result<Vec<(&'static str, SlotId)>, BuildError> {
    let slots = builder.generic(registry, &MESH, &[surface, tolerance])?;
    Ok(MESH.outputs.iter().map(|port| port.name).zip(slots).collect())
}

struct Ports<S>(PhantomData<S>);

impl<S: Surface> Ports<S> {
    const INPUTS: &'static [PortDecl] = &[
        PortDecl::new::<S>("surface").grouped(),
        PortDecl::new::<f32>("tolerance")
            .with_default(PortDefault::F32(&[TOLERANCE]))
            .one(),
    ];
}

const OUTPUTS: &[PortDecl] = &[
    PortDecl::new::<Vec3>("positions"),
    PortDecl::new::<Vec3>("normals"),
    PortDecl::new::<Vec2>("uvs"),
    PortDecl::new::<u32>("indices"),
];

/// Makes `S` usable wherever a surface is asked for, by registering the
/// version of every node that takes one for it.
pub fn register_surface<S: Surface>(registry: &mut NodeRegistry) {
    let node = version(&MESH, S::NAME, |id| NodeDef {
        id,
        display_name: MESH.display_name,
        inputs: Ports::<S>::INPUTS,
        outputs: OUTPUTS,
        version: 2,
        cpu: tessellate::<S>,
        gpu: &[],
    });
    registry.register_instance(&MESH, node);
    registry.hide(MESH.id);
    registry.register_drawable(Drawable {
        ty: S::INFO,
        topology: Topology::Surface,
        wgsl: <S as Surface>::WGSL,
        bounds: bounds::<S>,
    });
}

/// How many steps along `u` and across `v` the patches of a surface need
/// for the mesh to be no further than `tolerance` from it.
///
/// Every patch is looked at in [`PROBES`] steps both ways. How far the
/// middle of a step is from the straight line between its ends says how
/// much the surface bends there. That distance shrinks with the square
/// of the step, which tells how short the steps have to be. It is only
/// as good as the probes, so a bend much smaller than a probe step can go
/// unseen.
pub fn detail<S: Surface>(patches: &[S], tolerance: f32) -> (usize, usize) {
    let (mut along, mut across) = (0.0f32, 0.0f32);
    let at = |step: usize| step as f32 / PROBES as f32;
    for patch in patches {
        for line in 0..=PROBES {
            for step in 0..PROBES {
                let (from, to) = (at(step), at(step + 1));
                let middle = (from + to) * 0.5;
                let sag = |point: &dyn Fn(f32) -> Vec3| {
                    point(middle).distance((point(from) + point(to)) * 0.5)
                };
                along = along.max(sag(&|u| patch.point(u, at(line)).0));
                across = across.max(sag(&|v| patch.point(at(line), v).0));
            }
        }
    }
    // Where the surface bends both ways, the middle of a quad is off by
    // both, so each way gets half of it.
    let tolerance = (tolerance * 0.5).max(f32::EPSILON);
    let steps = |sag: f32| {
        let steps = (PROBES as f32 * (sag / tolerance).sqrt()).ceil();
        (steps as usize).clamp(1, MAX_STEPS)
    };
    (steps(along), steps(across))
}

/// How many steps [`detail`] looks at a patch in, both ways.
pub const PROBES: usize = 8;

/// Every patch becomes a grid of its own. Patches which meet share their
/// positions there but not their vertices, so an edge between two of
/// them stays sharp when their normals differ.
fn tessellate<S: Surface>(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let patches = inputs.get::<S>(0);
    let tolerance = inputs.value::<f32>(1);

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    for surface in inputs.groups(0) {
        let surface = &patches[surface];
        let (rings, sides) = detail(surface, tolerance);
        for patch in surface {
            let first = positions.len();
            for ring in 0..=rings {
                let u = ring as f32 / rings as f32;
                for side in 0..=sides {
                    let v = side as f32 / sides as f32;
                    let (position, normal) = patch.point(u, v);
                    positions.push(position);
                    normals.push(normal);
                    uvs.push(patch.uv(u, v));
                }
            }
            for ring in 0..rings {
                for side in 0..sides {
                    let a = (first + ring * (sides + 1) + side) as u32;
                    let b = a + 1;
                    let c = a + sides as u32 + 1;
                    let d = c + 1;
                    indices.extend_from_slice(&[a, b, c, b, d, c]);
                }
            }
        }
    }

    outputs.set(0, &positions);
    outputs.set(1, &normals);
    outputs.set(2, &uvs);
    outputs.set(3, &indices);
}
