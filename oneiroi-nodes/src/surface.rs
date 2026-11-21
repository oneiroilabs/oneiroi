//! Surfaces which are kept as what they are instead of as triangles.

use std::marker::PhantomData;

use glam::{Vec2, Vec3};
use oneiroi_core::{
    node::{
        Drawable, GenericPort, Inputs, NodeDef, NodeRegistry, Outputs, PortDecl, Topology,
    },
    program::{BuildError, ProgramBuilder, SlotId},
    types::DataType,
};

use rayon::prelude::*;

use crate::generic::version;

describe_trait! {
    /// A surface kept exact instead of as triangles. It gets as fine as it
    /// has to be where it is drawn or turned into a mesh.
    pub const SURFACE = "surface" { label: "Surface", hue: Green }
}

/// One patch of a surface: a rectangle of parameters `u` and `v`, both in
/// `0..=1`, with a point of the surface for each of them.
///
/// A surface is a slice of patches like any other value. It is small and
/// exact, and how fine it gets meshed is only decided where it is drawn
/// or turned into a [mesh](MESH).
pub trait Surface: DataType + Send + Sync {
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
fn bounds<S: Surface>(words: &[u32]) -> Option<(Vec3, f32)> {
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
    (min.x <= max.x).then(|| ((min + max) * 0.5, (max - min).length() * 0.5))
}

/// How far the triangles of a mesh may be from the surface it is made of,
/// unless told otherwise.
pub const TOLERANCE: f32 = 0.001;

/// No more steps than these along or across a patch, however small the
/// tolerance.
pub const MAX_STEPS: usize = 128;

generic_node! {
    /// A triangle mesh of a surface, as fine as it has to be for no triangle to
    /// be further than `tolerance` from the surface.
    ///
    /// Surfaces are drawn as they are, so this is only needed where triangles
    /// are, like for an export. Each group of patches is a surface of its own,
    /// and all of its patches get the same detail so that where they meet their
    /// edges line up.
    pub static MESH = "surface.mesh" {
        label: "Mesh Surface",
        category: "Surface",
        inputs {
            /// The surfaces to mesh.
            port GenericPort::implementing("surface", SURFACE).grouped(), "Surfaces";
            /// How far a triangle may be from the surface at most. Smaller makes more triangles.
            port GenericPort::exact::<f32>("tolerance")
                .with_default(&[TOLERANCE])
                .one();
        }
        outputs {
            /// A point for each corner of the triangles.
            port GenericPort::exact::<Vec3>("positions");
            /// The normal of the surface at each point.
            port GenericPort::exact::<Vec3>("normals");
            /// Where each point is on the surface, `u` along it and `v` across.
            port GenericPort::exact::<Vec2>("uvs"), "UVs";
            /// Three points for each triangle, counter-clockwise seen from outside.
            port GenericPort::exact::<u32>("indices");
        }
    }
}

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
            .with_default(&[TOLERANCE])
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
    let at = |step: usize| step as f32 / PROBES as f32;
    // The largest sag along and across of every patch, the patches on
    // every core. The largest is the same in any order.
    let (along, across) = patches
        .par_iter()
        .map(|patch| {
            let (mut along, mut across) = (0.0f32, 0.0f32);
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
            (along, across)
        })
        .reduce(|| (0.0, 0.0), |a, b| (a.0.max(b.0), a.1.max(b.1)));
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

/// One patch as a grid of `rings` by `sides` cells, its vertices numbered
/// from `first` on, written into room for exactly as many.
#[allow(clippy::too_many_arguments)]
fn grid<S: Surface>(
    patch: &S,
    rings: usize,
    sides: usize,
    first: usize,
    positions: &mut [Vec3],
    normals: &mut [Vec3],
    uvs: &mut [Vec2],
    indices: &mut [u32],
) {
    let mut vertex = 0;
    for ring in 0..=rings {
        let u = ring as f32 / rings as f32;
        for side in 0..=sides {
            let v = side as f32 / sides as f32;
            (positions[vertex], normals[vertex]) = patch.point(u, v);
            uvs[vertex] = patch.uv(u, v);
            vertex += 1;
        }
    }
    let mut corners = indices.chunks_exact_mut(6);
    for ring in 0..rings {
        for side in 0..sides {
            let a = (first + ring * (sides + 1) + side) as u32;
            let b = a + 1;
            let c = a + sides as u32 + 1;
            let d = c + 1;
            corners.next().unwrap().copy_from_slice(&[a, b, c, b, d, c]);
        }
    }
}

/// Every patch becomes a grid of its own. Patches which meet share their
/// positions there but not their vertices, so an edge between two of
/// them stays sharp when their normals differ.
fn tessellate<S: Surface>(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let patches = inputs.get::<S>(0);
    let tolerance = inputs.value::<f32>(1);

    // Every patch of a surface has as many vertices, so how many there are
    // in all is known before any is made.
    let surfaces: Vec<_> = inputs
        .groups(0)
        .into_iter()
        .map(|surface| {
            let (rings, sides) = detail(&patches[surface.clone()], tolerance);
            (surface, rings, sides)
        })
        .collect();
    let (vertices, corners) = surfaces.iter().fold((0, 0), |(vertices, corners), (surface, rings, sides)| {
        let patches = surface.len();
        (vertices + patches * (rings + 1) * (sides + 1), corners + patches * rings * sides * 6)
    });

    // Each patch writes its grid right where it is kept, on every core.
    let mut positions = outputs.alloc_unset::<Vec3>(0, vertices);
    let mut normals = outputs.alloc_unset::<Vec3>(1, vertices);
    let mut uvs = outputs.alloc_unset::<Vec2>(2, vertices);
    let mut indices = outputs.alloc_unset::<u32>(3, corners);
    let mut first = 0;
    for (surface, rings, sides) in surfaces {
        let per_patch = (rings + 1) * (sides + 1);
        let per_patch_corners = rings * sides * 6;
        let count = surface.len();
        let (p, rest) = std::mem::take(&mut positions).split_at_mut(count * per_patch);
        positions = rest;
        let (n, rest) = std::mem::take(&mut normals).split_at_mut(count * per_patch);
        normals = rest;
        let (u, rest) = std::mem::take(&mut uvs).split_at_mut(count * per_patch);
        uvs = rest;
        let (i, rest) = std::mem::take(&mut indices).split_at_mut(count * per_patch_corners);
        indices = rest;
        p.par_chunks_mut(per_patch)
            .zip(n.par_chunks_mut(per_patch))
            .zip(u.par_chunks_mut(per_patch))
            .zip(i.par_chunks_mut(per_patch_corners))
            .zip(&patches[surface])
            .enumerate()
            .for_each(|(index, ((((p, n), u), i), patch))| {
                grid(patch, rings, sides, first + index * per_patch, p, n, u, i)
            });
        first += count * per_patch;
    }
}
