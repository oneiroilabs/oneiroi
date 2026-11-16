//! Runs oneiroi graphs inside bevy.
//!
//! Add the [`OneiroiPlugin`], put programs into the [`Oneiroi`] resource and
//! spawn [`OneiroiInstance`]s. Every instance whose program has the outputs
//! named by [`POSITIONS`] and [`INDICES`] is given a [`Mesh3d`] which follows
//! its parameters.

use std::collections::{HashMap, HashSet};

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use oneiroi_core::{
    exec::{Engine, Instance, ParamError},
    node::NodeRegistry,
    program::{Program, ProgramId, ProgramSet},
    store::ContentKey,
    types::DataType,
};

mod global;

/// `vec3` output with one position per vertex.
pub const POSITIONS: &str = "positions";
/// `u32` output with three indices per triangle.
pub const INDICES: &str = "indices";
/// Optional `vec3` output with one normal per vertex.
pub const NORMALS: &str = "normals";
/// Optional `vec2` output with one texture coordinate per vertex.
pub const UVS: &str = "uvs";

pub struct OneiroiPlugin;

impl Plugin for OneiroiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Oneiroi>()
            .add_systems(PostUpdate, evaluate_instances);
    }
}

/// The keys of everything a mesh is made of. Equal keys mean equal meshes.
type MeshKey = [Option<ContentKey>; 4];

/// What the last frame cost.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OneiroiStats {
    pub instances: usize,
    /// How many nodes had to run. Zero while nothing changes.
    pub nodes_run: usize,
    /// How many different meshes all instances share.
    pub meshes: usize,
}

#[derive(Resource)]
pub struct Oneiroi {
    engine: Engine,
    registry: NodeRegistry,
    meshes: HashMap<MeshKey, Handle<Mesh>>,
    stats: OneiroiStats,
    /// How many 32-bit words of results are kept around before the ones
    /// nobody uses anymore are dropped.
    pub budget_words: u64,
}

impl Default for Oneiroi {
    fn default() -> Self {
        let mut registry = NodeRegistry::new();
        oneiroi_nodes::register(&mut registry);
        Self {
            engine: Engine::default(),
            registry,
            meshes: HashMap::new(),
            stats: OneiroiStats::default(),
            // 256 MiB
            budget_words: 1 << 26,
        }
    }
}

impl Oneiroi {
    /// Every node programs can be built from. The ones of `oneiroi-nodes`
    /// are in there from the start.
    pub fn registry(&self) -> &NodeRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut NodeRegistry {
        &mut self.registry
    }

    pub fn programs(&self) -> &ProgramSet {
        self.engine.programs()
    }

    pub fn add_program(&mut self, program: Program) -> ProgramId {
        self.engine.add_program(program)
    }

    /// A new instance with every parameter at its default.
    pub fn instantiate(&self, program: ProgramId) -> OneiroiInstance {
        OneiroiInstance {
            instance: self.engine.instantiate(program),
            shown: None,
        }
    }

    pub fn set<T: DataType>(
        &self,
        instance: &mut OneiroiInstance,
        name: &str,
        value: &[T],
    ) -> Result<(), ParamError> {
        self.engine.set_param(&mut instance.instance, name, value)
    }

    /// Any output of an instance, as of the last frame.
    pub fn read<T: DataType>(&self, instance: &OneiroiInstance, name: &str) -> Option<&[T]> {
        self.engine.read(&instance.instance, name)
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn stats(&self) -> OneiroiStats {
        self.stats
    }
}

#[derive(Component)]
pub struct OneiroiInstance {
    instance: Instance,
    /// The mesh the entity currently shows.
    shown: Option<MeshKey>,
}

impl OneiroiInstance {
    pub fn program(&self) -> ProgramId {
        self.instance.program()
    }
}

fn evaluate_instances(
    mut oneiroi: ResMut<Oneiroi>,
    mut assets: ResMut<Assets<Mesh>>,
    mut instances: Query<(Entity, &mut OneiroiInstance)>,
    mut commands: Commands,
) {
    let oneiroi = &mut *oneiroi;

    // Evaluating is bookkeeping and not a change somebody else should see.
    let mut all: Vec<_> = instances.iter_mut().collect();
    let nodes_run = oneiroi.engine.evaluate(
        all.iter_mut()
            .map(|(_, instance)| &mut instance.bypass_change_detection().instance),
    );

    let mut used = HashSet::new();
    for (entity, instance) in &mut all {
        let key = [POSITIONS, INDICES, NORMALS, UVS]
            .map(|name| oneiroi.engine.output_key(&instance.instance, name));
        used.insert(key);
        if instance.shown == Some(key) {
            continue;
        }
        instance.bypass_change_detection().shown = Some(key);

        // Instances with equal results show the very same mesh.
        let handle = match oneiroi.meshes.get(&key) {
            Some(handle) => Some(handle.clone()),
            None => build_mesh(&oneiroi.engine, &instance.instance).map(|mesh| {
                let handle = assets.add(mesh);
                oneiroi.meshes.insert(key, handle.clone());
                handle
            }),
        };
        match handle {
            Some(handle) => commands.entity(*entity).insert(Mesh3d(handle)),
            None => commands.entity(*entity).remove::<Mesh3d>(),
        };
    }

    oneiroi.meshes.retain(|key, _| used.contains(key));
    oneiroi.stats = OneiroiStats {
        instances: all.len(),
        nodes_run,
        meshes: oneiroi.meshes.len(),
    };
    oneiroi.engine.end_frame(oneiroi.budget_words);
}

fn build_mesh(engine: &Engine, instance: &Instance) -> Option<Mesh> {
    let arrays = |name| {
        engine
            .read::<glam::Vec3>(instance, name)
            .map(|values| values.iter().map(|v| v.to_array()).collect::<Vec<_>>())
    };
    let positions = arrays(POSITIONS)?;
    let indices = engine.read::<u32>(instance, INDICES)?.to_vec();
    let vertices = positions.len();

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_indices(Indices::U32(indices));

    if let Some(normals) = arrays(NORMALS).filter(|normals| normals.len() == vertices) {
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    }
    let uvs = engine.read::<glam::Vec2>(instance, UVS);
    if let Some(uvs) = uvs.filter(|uvs| uvs.len() == vertices) {
        let uvs: Vec<_> = uvs.iter().map(|uv| uv.to_array()).collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    }
    Some(mesh)
}

#[cfg(test)]
mod tests {
    use oneiroi_core::program::ProgramBuilder;
    use oneiroi_nodes::{
        curve::{NURBS, nurbs::clamped_uniform_knots},
        surface::TESSELLATE,
    };

    use super::*;

    /// A fixed curve with the tube graph called on it.
    fn tube(oneiroi: &mut Oneiroi) -> ProgramId {
        let tube = oneiroi.add_program(oneiroi_nodes::curve::tube(oneiroi.registry()));

        let mut b = ProgramBuilder::new("test");
        let points = b.constant::<[f32; 4]>(&[
            [0., 0., 0., 1.],
            [1., 2., 0., 1.],
            [2., 0., 1., 1.],
            [3., 2., 0., 1.],
        ]);
        let knots = b.constant(&clamped_uniform_knots(4));
        let radius = b.param::<f32>("radius", &[0.25]);
        let rings = b.constant::<u32>(&[4]);
        let sides = b.constant::<u32>(&[6]);
        let segments = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let mesh = b
            .call(
                oneiroi.programs(),
                tube,
                &[Some(segments), Some(radius), Some(rings), Some(sides)],
            )
            .unwrap();
        for (port, slot) in TESSELLATE.outputs.iter().zip(mesh) {
            b.output(port.name, slot);
        }
        oneiroi.add_program(b.finish())
    }

    fn app() -> (App, [Entity; 3]) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), OneiroiPlugin))
            .init_asset::<Mesh>();

        let mut oneiroi = app.world_mut().resource_mut::<Oneiroi>();
        let program = tube(&mut oneiroi);
        let instances = [(); 3].map(|_| oneiroi.instantiate(program));
        let entities = instances.map(|instance| app.world_mut().spawn(instance).id());
        (app, entities)
    }

    fn mesh(app: &App, entity: Entity) -> Handle<Mesh> {
        app.world().get::<Mesh3d>(entity).unwrap().0.clone()
    }

    fn set_radius(app: &mut App, entity: Entity, radius: f32) {
        let world = app.world_mut();
        let mut instance = world.entity_mut(entity).take::<OneiroiInstance>().unwrap();
        world
            .resource::<Oneiroi>()
            .set(&mut instance, "radius", &[radius])
            .unwrap();
        world.entity_mut(entity).insert(instance);
    }

    #[test]
    fn equal_instances_share_one_mesh() {
        let (mut app, [a, b, c]) = app();
        app.update();

        let stats = app.world().resource::<Oneiroi>().stats();
        assert_eq!(
            stats,
            OneiroiStats {
                instances: 3,
                nodes_run: 4,
                meshes: 1
            }
        );
        assert_eq!(mesh(&app, a), mesh(&app, b));
        assert_eq!(mesh(&app, a), mesh(&app, c));

        let meshes = app.world().resource::<Assets<Mesh>>();
        let tube = meshes.get(&mesh(&app, a)).unwrap();
        assert_eq!(tube.count_vertices(), 5 * 7);
        assert!(tube.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
        assert!(tube.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
    }

    #[test]
    fn a_changed_instance_gets_its_own_mesh() {
        let (mut app, [a, b, _]) = app();
        app.update();
        app.update();
        assert_eq!(app.world().resource::<Oneiroi>().stats().nodes_run, 0);

        set_radius(&mut app, b, 0.5);
        app.update();
        let stats = app.world().resource::<Oneiroi>().stats();
        // A new circle is swept and meshed, the curve is still the same.
        assert_eq!((stats.nodes_run, stats.meshes), (3, 2));
        assert_ne!(mesh(&app, a), mesh(&app, b));

        // Once nobody shows a mesh anymore it is dropped again.
        set_radius(&mut app, b, 0.25);
        app.update();
        let stats = app.world().resource::<Oneiroi>().stats();
        assert_eq!((stats.nodes_run, stats.meshes), (0, 1));
        assert_eq!(mesh(&app, a), mesh(&app, b));
    }
}
