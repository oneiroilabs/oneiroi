//! Surfaces which are kept as what they are instead of as triangles.

use std::marker::PhantomData;

use glam::{Vec2, Vec3};
use oneiroi_core::{
    node::{
        Drawable, GenericNode, GenericPort, Inputs, NodeDef, NodeRegistry, Outputs, PortDecl, Topology,
    },
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
/// or [tessellated](TESSELLATE).
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

const RINGS: PortDefault = PortDefault::U32(&[16]);
const SIDES: PortDefault = PortDefault::U32(&[24]);

/// A triangle mesh of a surface, with `rings` steps along and `sides`
/// steps across every patch. Those two are the level of detail.
pub static TESSELLATE: GenericNode = GenericNode {
    id: "surface.tessellate",
    display_name: "Tessellate",
    inputs: &[
        GenericPort::implementing("surface", SURFACE),
        GenericPort::exact::<u32>("rings").with_default(RINGS).one(),
        GenericPort::exact::<u32>("sides").with_default(SIDES).one(),
    ],
    outputs: &[
        GenericPort::exact::<Vec3>("positions"),
        GenericPort::exact::<Vec3>("normals"),
        GenericPort::exact::<Vec2>("uvs"),
        GenericPort::exact::<u32>("indices"),
    ],
};

struct Ports<S>(PhantomData<S>);

impl<S: Surface> Ports<S> {
    const INPUTS: &'static [PortDecl] = &[
        PortDecl::new::<S>("surface"),
        PortDecl::new::<u32>("rings").with_default(RINGS).one(),
        PortDecl::new::<u32>("sides").with_default(SIDES).one(),
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
    let node = version(&TESSELLATE, S::NAME, |id| NodeDef {
        id,
        display_name: TESSELLATE.display_name,
        inputs: Ports::<S>::INPUTS,
        outputs: OUTPUTS,
        version: 1,
        cpu: tessellate::<S>,
        gpu: &[],
    });
    registry.register_instance(&TESSELLATE, node);
    registry.register_drawable(Drawable {
        ty: S::INFO,
        topology: Topology::Surface,
        wgsl: <S as Surface>::WGSL,
        bounds: bounds::<S>,
    });
}

/// Every patch becomes a grid of its own. Patches which meet share their
/// positions there but not their vertices, so an edge between two of
/// them stays sharp when their normals differ.
fn tessellate<S: Surface>(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let patches = inputs.get::<S>(0);
    let rings = inputs.value::<u32>(1).max(1) as usize;
    let sides = inputs.value::<u32>(2).max(1) as usize;

    let per_patch = (rings + 1) * (sides + 1);
    let mut positions = Vec::with_capacity(patches.len() * per_patch);
    let mut normals = Vec::with_capacity(positions.capacity());
    let mut uvs = Vec::with_capacity(positions.capacity());
    let mut indices = Vec::with_capacity(patches.len() * rings * sides * 6);

    for (index, patch) in patches.iter().enumerate() {
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

        let first = index * per_patch;
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

    outputs.set(0, &positions);
    outputs.set(1, &normals);
    outputs.set(2, &uvs);
    outputs.set(3, &indices);
}
