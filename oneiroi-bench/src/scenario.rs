//! What is measured: small graphs with an input which every edit changes,
//! the way an artist changes a value and waits for the result.

use bytemuck::Pod;
use glam::{Vec2, Vec3};
use oneiroi_core::{
    node::NodeRegistry,
    program::{Program, ProgramBuilder, ProgramSet, SlotId},
    types::DataType,
};
use oneiroi_nodes::{
    curve::nurbs::CubicNurbsSegmentCache,
    curve::{ControlPoint, NURBS, NURBS_BATCH, SAMPLE_CURVE, SAMPLE_CURVE_BATCH, TRANSLATE, swept, swept_batch},
    instance::{INSTANCE, REALIZE},
    primitive::CIRCLE,
    surface,
};
use serde::{Deserialize, Serialize};

/// How much a scenario works on: this many curves with this many control
/// points each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    pub curves: u32,
    pub points: u32,
}

impl Size {
    pub const fn new(curves: u32, points: u32) -> Self {
        Self { curves, points }
    }

    pub fn label(&self) -> String {
        format!("{} × {}", self.curves, self.points)
    }
}

/// Values of parameters, by name, as words.
pub type Params = Vec<(&'static str, Vec<u32>)>;

pub struct Scenario {
    pub name: &'static str,
    /// What the scenario runs, and which part an edit makes run again.
    pub about: &'static str,
    pub sizes: &'static [Size],
    /// A few of the sizes, for a quick look.
    pub quick: &'static [Size],
    pub work: Work,
}

/// What a scenario runs, and what an edit changes of it.
pub enum Work {
    /// One graph with one instance.
    One {
        /// The program for a size, and any it calls, which go into
        /// `programs`.
        build: fn(&NodeRegistry, &mut ProgramSet, Size) -> Program,
        /// The parameters before the first edit.
        setup: fn(Size) -> Params,
        /// What edit `n` sets, starting at 1. Every edit sets values which
        /// were not there before, so that nothing is found in the store.
        edit: fn(Size, u32) -> Params,
    },
    /// Many graphs with many instances each, all evaluated together, as a
    /// host does with what it shows.
    Scene {
        build: fn(&NodeRegistry, &mut ProgramSet, Size) -> Scene,
        /// What edit `n` sets on which instances, by their index.
        edit: fn(Size, u32) -> Vec<(usize, Params)>,
    },
}

/// The graphs of a scene and its instances.
pub struct Scene {
    pub graphs: Vec<Program>,
    /// Of which graph each instance is, by its index, and its parameters.
    pub instances: Vec<(usize, Params)>,
    /// Parameters which follow outputs of other instances, see
    /// [`Engine::link`](oneiroi_core::exec::Engine::link):
    /// `(instance, parameter, instance it follows, output)`.
    pub links: Vec<(usize, &'static str, usize, &'static str)>,
}

impl Work {
    /// The scene a scenario runs at `size`.
    pub fn scene(&self, registry: &NodeRegistry, programs: &mut ProgramSet, size: Size) -> Scene {
        match self {
            Work::One { build, setup, .. } => Scene {
                graphs: vec![build(registry, programs, size)],
                instances: vec![(0, setup(size))],
                links: Vec::new(),
            },
            Work::Scene { build, .. } => build(registry, programs, size),
        }
    }

    /// What edit `n` sets, by the index of the instance.
    pub fn edit(&self, size: Size, n: u32) -> Vec<(usize, Params)> {
        match self {
            Work::One { edit, .. } => vec![(0, edit(size, n))],
            Work::Scene { edit, .. } => edit(size, n),
        }
    }
}

const CURVES: &[Size] = &[
    Size::new(1, 32),
    Size::new(16, 32),
    Size::new(256, 32),
    Size::new(4096, 32),
    Size::new(1, 128),
    Size::new(16, 128),
    Size::new(256, 128),
    Size::new(1024, 128),
];
const CURVES_QUICK: &[Size] = &[Size::new(1, 32), Size::new(256, 32), Size::new(16, 128)];
const INSTANCES: &[Size] = &[Size::new(1, 32), Size::new(16, 32), Size::new(256, 32), Size::new(1024, 32)];
const INSTANCES_QUICK: &[Size] = &[Size::new(1, 32), Size::new(256, 32)];
const ELEMENTS: &[Size] = &[Size::new(16, 8), Size::new(256, 8), Size::new(4096, 8), Size::new(16384, 8)];
const ELEMENTS_QUICK: &[Size] = &[Size::new(256, 8), Size::new(4096, 8)];
const MESHES: &[Size] = &[Size::new(1, 32), Size::new(16, 32), Size::new(64, 32)];
/// For the fence, as many pickets.
const FENCES: &[Size] = &[Size::new(10, 1), Size::new(100, 1), Size::new(1000, 1), Size::new(10000, 1)];
const FENCES_QUICK: &[Size] = &[Size::new(10, 1), Size::new(1000, 1)];
/// How far apart the pickets of the fence are.
const PICKETS: f32 = 0.4;
const MESHES_QUICK: &[Size] = &[Size::new(1, 32), Size::new(16, 32)];

/// How many frames Sample Curve makes of each curve.
const SAMPLES: usize = 64;

pub const SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "nurbs",
        about: "NURBS Curve, the control points move",
        sizes: CURVES,
        quick: CURVES_QUICK,
        work: Work::One {
            build: |_, _, _| {
                let mut b = ProgramBuilder::new("nurbs");
                let segments = curves(&mut b);
                b.output("segments", segments);
                b.finish()
            },
            setup: points,
            edit: moved_points,
        },
    },
    Scenario {
        name: "translate",
        about: "Translate Curve of the curves, the offset changes",
        sizes: CURVES,
        quick: CURVES_QUICK,
        work: Work::One {
            build: |_, _, _| {
                let mut b = ProgramBuilder::new("translate");
                let segments = curves(&mut b);
                let offset = b.param_one_of::<Vec3>("offset", Vec3::ZERO);
                let moved = b.node(&TRANSLATE, &[segments, offset]).unwrap()[0];
                b.output("segments", moved);
                b.finish()
            },
            setup: points,
            edit: |_, n| vec![("offset", words(&[Vec3::new(n as f32 * 1e-3, 0.0, 0.0)]))],
        },
    },
    Scenario {
        name: "sweep circle",
        about: "Sweep of a circle along the curves, the radius changes",
        sizes: CURVES,
        quick: CURVES_QUICK,
        work: Work::One {
            build: |registry, _, _| {
                let mut b = ProgramBuilder::new("sweep circle");
                let spine = curves(&mut b);
                let radius = b.param_one_of::<f32>("radius", 0.1);
                let circle = b.node(&CIRCLE, &[radius]).unwrap()[0];
                let surface = swept_batch(&mut b, registry, spine, circle).unwrap();
                b.output("surface", surface);
                b.finish()
            },
            setup: points,
            edit: |_, n| vec![("radius", words(&[0.1 + n as f32 * 1e-6]))],
        },
    },
    Scenario {
        name: "sweep square",
        about: "Sweep of a square curve along the curves, the square changes",
        sizes: CURVES,
        quick: CURVES_QUICK,
        work: Work::One {
            build: |registry, _, _| {
                let mut b = ProgramBuilder::new("sweep square");
                let spine = curves(&mut b);
                let square = b.param_words("square", ControlPoint::INFO, &[]);
                let knots = b.constant(&square_knots());
                let profile = b.node(&NURBS, &[square, knots]).unwrap()[0];
                let surface = swept_batch(&mut b, registry, spine, profile).unwrap();
                b.output("surface", surface);
                b.finish()
            },
            setup: |size| {
                let mut params = points(size);
                params.push(("square", words(&square(1.0))));
                params
            },
            edit: |_, n| vec![("square", words(&square(1.0 + n as f32 * 1e-6)))],
        },
    },
    Scenario {
        name: "sample",
        about: "Sample Curve, 64 frames per curve at places which change",
        sizes: CURVES,
        quick: CURVES_QUICK,
        work: Work::One {
            build: |_, _, _| {
                let mut b = ProgramBuilder::new("sample");
                let segments = curves(&mut b);
                let count = b.constant::<u32>(&[SAMPLES as u32]);
                let spacing = b.constant::<f32>(&[0.0]);
                let t = b.param::<f32>("t", &[]);
                let frames = b.node(&SAMPLE_CURVE_BATCH, &[segments, count, spacing, t]).unwrap()[0];
                b.output("frames", frames);
                b.finish()
            },
            setup: |size| {
                let mut params = points(size);
                params.push(("t", words(&places(0))));
                params
            },
            edit: |_, n| vec![("t", words(&places(n)))],
        },
    },
    Scenario {
        name: "chain",
        about: "NURBS, Translate, a Sweep of a circle and Sample Curve, the control points move",
        sizes: CURVES,
        quick: CURVES_QUICK,
        work: Work::One {
            build: |registry, _, _| {
                let mut b = ProgramBuilder::new("chain");
                let segments = curves(&mut b);
                let offset = b.constant(&[Vec3::new(0.0, 1.0, 0.0)]);
                let moved = b.node(&TRANSLATE, &[segments, offset]).unwrap()[0];
                let radius = b.constant::<f32>(&[0.1]);
                let circle = b.node(&CIRCLE, &[radius]).unwrap()[0];
                let surface = swept_batch(&mut b, registry, moved, circle).unwrap();
                let count = b.constant::<u32>(&[SAMPLES as u32]);
                let spacing = b.constant::<f32>(&[0.0]);
                let t = b.constant::<f32>(&[]);
                let frames = b.node(&SAMPLE_CURVE_BATCH, &[moved, count, spacing, t]).unwrap()[0];
                b.output("surface", surface);
                b.output("frames", frames);
                b.finish()
            },
            setup: points,
            edit: moved_points,
        },
    },
    Scenario {
        name: "instances",
        about: "A ring of 10 patches put onto 64 frames along every curve by reference, the control points move",
        sizes: INSTANCES,
        quick: INSTANCES_QUICK,
        work: Work::One {
            build: |registry, _, _| {
                let mut b = ProgramBuilder::new("instances");
                let placed = rings_on_curves(&mut b, registry);
                b.output("instances", placed);
                b.finish()
            },
            setup: points,
            edit: moved_points,
        },
    },
    Scenario {
        name: "instances realized",
        about: "The rings of `instances`, copied to where each one is",
        sizes: INSTANCES,
        quick: INSTANCES_QUICK,
        work: Work::One {
            build: |registry, _, _| {
                let mut b = ProgramBuilder::new("instances realized");
                let placed = rings_on_curves(&mut b, registry);
                let copies = b.generic(registry, &REALIZE, &[placed]).unwrap()[0];
                b.output("copies", copies);
                b.finish()
            },
            setup: points,
            edit: moved_points,
        },
    },
    Scenario {
        name: "for each",
        about: "NURBS and Translate for every curve on its own, the control points of one curve move",
        sizes: ELEMENTS,
        quick: ELEMENTS_QUICK,
        work: Work::One {
            build: |_, programs, _| {
                let mut b = ProgramBuilder::new("for each");
                let moved = each_curve(&mut b, programs);
                b.output("segments", moved);
                b.finish()
            },
            setup: points,
            edit: moved_points,
        },
    },
    Scenario {
        name: "for each, all",
        about: "NURBS and Translate for every curve on its own, the offset of all of them changes",
        sizes: ELEMENTS,
        quick: ELEMENTS_QUICK,
        work: Work::One {
            build: |_, programs, _| {
                let mut b = ProgramBuilder::new("for each, all");
                let moved = each_curve(&mut b, programs);
                b.output("segments", moved);
                b.finish()
            },
            setup: points,
            edit: |_, n| vec![("offset", words(&[Vec3::new(n as f32 * 1e-3, 0.0, 0.0)]))],
        },
    },
    Scenario {
        name: "mesh",
        about: "Mesh of a swept circle, the tolerance changes",
        sizes: MESHES,
        quick: MESHES_QUICK,
        work: Work::One {
            build: |registry, _, _| {
                let mut b = ProgramBuilder::new("mesh");
                let spine = curves(&mut b);
                let radius = b.constant::<f32>(&[0.1]);
                let circle = b.node(&CIRCLE, &[radius]).unwrap()[0];
                let swept = swept_batch(&mut b, registry, spine, circle).unwrap();
                let tolerance = b.param_one_of::<f32>("tolerance", 0.01);
                for (name, slot) in surface::mesh(&mut b, registry, swept, tolerance).unwrap() {
                    b.output(name, slot);
                }
                b.finish()
            },
            setup: points,
            edit: |_, n| vec![("tolerance", words(&[0.01 * (1.0 + n as f32 * 1e-6)]))],
        },
    },
    Scenario {
        name: "fence",
        about: "The picket fence of the editor, with as many pickets, gets a little longer",
        sizes: FENCES,
        quick: FENCES_QUICK,
        work: Work::One {
            build: |registry, programs, _| {
                let context = oneiroi_graph::Context {
                    registry,
                    programs,
                    graphs: &std::collections::HashMap::new(),
                };
                oneiroi_graph::examples::fence().lower(context).expect("the fence is a whole graph")
            },
            setup: |size| vec![("length", words(&[fence_length(size)])), ("spacing", words(&[PICKETS]))],
            edit: |size, n| vec![("length", words(&[longer(fence_length(size), n)]))],
        },
    },
    Scenario {
        name: "mostly constant",
        about: "As many curves of constant points, each moved by a constant and an output of its own, and one curve moved by the only parameter, which changes",
        sizes: PARTS,
        quick: PARTS_QUICK,
        work: Work::One {
            build: |_, _, size| mostly_constant(size),
            setup: |_| vec![("offset", words(&[Vec3::ZERO]))],
            edit: |_, n| vec![("offset", words(&[Vec3::new(n as f32 * 1e-3, 0.0, 0.0)]))],
        },
    },
    Scenario {
        name: "scene, a few move",
        about: "Instances of different graphs, all evaluated together, of which 4 move with every edit. A size is the instances × the graphs. Every graph makes a curve out of the points of its instance and moves it, then sweeps, samples or moves it again in its own way",
        sizes: SCENES,
        quick: SCENES_QUICK,
        work: Work::Scene {
            build: scene,
            edit: |size, n| {
                let instances = size.curves as usize;
                (0..MOVED.min(instances))
                    .map(|k| {
                        let index = (n as usize * 7 + k * instances / MOVED) % instances;
                        (index, offset(index as u32, n))
                    })
                    .collect()
            },
        },
    },
    Scenario {
        name: "scene, all move",
        about: "The instances of `scene, a few move`, all of which move with every edit",
        sizes: SCENES,
        quick: SCENES_QUICK,
        work: Work::Scene {
            build: scene,
            edit: |size, n| (0..size.curves as usize).map(|index| (index, offset(index as u32, n))).collect(),
        },
    },
    Scenario {
        name: "scene, linked",
        about: "Rails of the curves of `scene, a few move`, each followed by a tube along it, of which 4 rails move with every edit. A size is the rails × the graphs of tubes",
        sizes: SCENES,
        quick: SCENES_QUICK,
        work: Work::Scene {
            build: linked,
            edit: |size, n| {
                let rails = size.curves as usize;
                (0..MOVED.min(rails))
                    .map(|k| {
                        let index = (n as usize * 7 + k * rails / MOVED) % rails;
                        (index, offset(index as u32, n))
                    })
                    .collect()
            },
        },
    },
];

/// `size.curves` rails, the first instances, each followed by a tube of
/// one of `size.points` graphs.
fn linked(registry: &NodeRegistry, programs: &mut ProgramSet, size: Size) -> Scene {
    let mut scene = scene(registry, programs, size);
    let mut rail = ProgramBuilder::new("rail");
    let points = rail.param::<ControlPoint>("points", &[]);
    let knots = rail.constant::<f32>(&[]);
    let curve = rail.node(&NURBS, &[points, knots]).unwrap()[0];
    let offset = rail.param_one_of::<Vec3>("offset", Vec3::ZERO);
    let moved = rail.node(&TRANSLATE, &[curve, offset]).unwrap()[0];
    rail.output("curve", moved);

    let tubes = (0..size.points).map(|index| {
        let mut b = ProgramBuilder::new(format!("tube {index}"));
        let spine = b.param::<CubicNurbsSegmentCache>("spine", &[]);
        let radius = b.constant::<f32>(&[0.05 + index as f32 * 1e-3]);
        let circle = b.node(&CIRCLE, &[radius]).unwrap()[0];
        let surface = swept(&mut b, registry, spine, circle).unwrap();
        b.output("surface", surface);
        b.finish()
    });
    scene.graphs = std::iter::once(rail.finish()).chain(tubes).collect();
    let count = scene.instances.len();
    for (graph, _) in &mut scene.instances {
        *graph = 0;
    }
    for index in 0..count {
        scene.instances.push((1 + index % size.points as usize, Vec::new()));
        scene.links.push((count + index, "spine", index, "curve"));
    }
    scene
}

/// For the scenes, as many instances × as many graphs.
const SCENES: &[Size] = &[
    Size::new(16, 4),
    Size::new(256, 16),
    Size::new(4096, 16),
    Size::new(4096, 64),
    Size::new(16384, 64),
];
const SCENES_QUICK: &[Size] = &[Size::new(256, 16), Size::new(4096, 16)];
/// How many instances an edit of `scene, a few move` moves.
const MOVED: usize = 4;
/// How many control points the curve of every instance of a scene has.
const SCENE_POINTS: u32 = 8;

/// `size.points` graphs, and `size.curves` instances of them, one after
/// the other.
fn scene(registry: &NodeRegistry, _: &mut ProgramSet, size: Size) -> Scene {
    let graphs = (0..size.points).map(|graph| scene_graph(registry, graph)).collect();
    let instances = (0..size.curves)
        .map(|index| {
            let points: Vec<ControlPoint> = (0..SCENE_POINTS)
                .map(|i| {
                    let t = i as f32;
                    let phase = index as f32 * 0.37;
                    ControlPoint::at(Vec3::new(t, (t * 0.9 + phase).sin(), (t * 1.3 + phase).cos()))
                })
                .collect();
            let params = vec![("points", words(&points)), ("offset", words(&[place(index)]))];
            ((index % size.points) as usize, params)
        })
        .collect();
    Scene {
        graphs,
        instances,
        links: Vec::new(),
    }
}

/// Where instance `index` of a scene is, on a grid.
fn place(index: u32) -> Vec3 {
    Vec3::new((index % 128) as f32 * 10.0, 0.0, (index / 128) as f32 * 10.0)
}

/// The offset of instance `index` of a scene for edit `n`, which no edit
/// had before.
fn offset(index: u32, n: u32) -> Params {
    vec![("offset", words(&[place(index) + Vec3::new(n as f32 * 1e-3, 0.0, 0.0)]))]
}

/// Graph `index` of a scene: a NURBS curve of the points of its instance,
/// moved by its offset, and then one of four things, each with constants
/// of its own. All graphs share the kinds of nodes, none of them shares a
/// value.
fn scene_graph(registry: &NodeRegistry, index: u32) -> Program {
    let mut b = ProgramBuilder::new(format!("scene graph {index}"));
    let points = b.param::<ControlPoint>("points", &[]);
    let knots = b.constant::<f32>(&[]);
    let curve = b.node(&NURBS, &[points, knots]).unwrap()[0];
    let offset = b.param_one_of::<Vec3>("offset", Vec3::ZERO);
    let moved = b.node(&TRANSLATE, &[curve, offset]).unwrap()[0];
    let own = index as f32 * 1e-3;
    match index % 4 {
        // A tube.
        0 => {
            let radius = b.constant::<f32>(&[0.05 + own]);
            let circle = b.node(&CIRCLE, &[radius]).unwrap()[0];
            let surface = swept(&mut b, registry, moved, circle).unwrap();
            b.output("surface", surface);
        }
        // Frames along it.
        1 => {
            let count = b.constant::<u32>(&[8 + index]);
            let spacing = b.constant::<f32>(&[0.0]);
            let t = b.constant::<f32>(&[]);
            let frames = b.node(&SAMPLE_CURVE, &[moved, count, spacing, t]).unwrap()[0];
            b.output("frames", frames);
        }
        // A tube a little above it.
        2 => {
            let up = b.constant(&[Vec3::new(0.0, 1.0 + own, 0.0)]);
            let lifted = b.node(&TRANSLATE, &[moved, up]).unwrap()[0];
            let radius = b.constant::<f32>(&[0.1]);
            let circle = b.node(&CIRCLE, &[radius]).unwrap()[0];
            let surface = swept(&mut b, registry, lifted, circle).unwrap();
            b.output("surface", surface);
        }
        // A square tube.
        _ => {
            let square = b.constant(&square(1.0 + own));
            let knots = b.constant(&square_knots());
            let profile = b.node(&NURBS, &[square, knots]).unwrap()[0];
            let surface = swept(&mut b, registry, moved, profile).unwrap();
            b.output("surface", surface);
        }
    }
    b.finish()
}

/// For [`mostly_constant`], as many constant curves. The largest is the
/// size of a scene, not of a node.
const PARTS: &[Size] = &[Size::new(16, 8), Size::new(256, 8), Size::new(4096, 8), Size::new(16384, 8)];
const PARTS_QUICK: &[Size] = &[Size::new(256, 8), Size::new(4096, 8)];

/// The program of the scenario `mostly constant`: everything but the last
/// curve is constant.
fn mostly_constant(size: Size) -> Program {
    let parts = size.curves;
    let mut b = ProgramBuilder::new("mostly constant");
    let knots = b.constant::<f32>(&[]);
    for part in 0..parts {
        let points: Vec<ControlPoint> = (0..8)
            .map(|i| ControlPoint::at(Vec3::new(i as f32, (i * i) as f32 * 0.1, part as f32)))
            .collect();
        let points = b.constant(&points);
        let curve = b.node(&NURBS, &[points, knots]).unwrap()[0];
        let by = b.constant(&[Vec3::new(0.0, part as f32, 0.0)]);
        let moved = b.node(&TRANSLATE, &[curve, by]).unwrap()[0];
        b.output(format!("part {part}"), moved);
    }
    let points: Vec<ControlPoint> = (0..8).map(|i| ControlPoint::at(Vec3::new(i as f32, 1.0, -1.0))).collect();
    let points = b.constant(&points);
    let curve = b.node(&NURBS, &[points, knots]).unwrap()[0];
    let offset = b.param_one_of::<Vec3>("offset", Vec3::ZERO);
    let moved = b.node(&TRANSLATE, &[curve, offset]).unwrap()[0];
    b.output("moved", moved);
    b.finish()
}

/// As long as the fence is with `size.curves` pickets, which start one
/// apart from either end.
fn fence_length(size: Size) -> f32 {
    (size.curves + 1) as f32 * PICKETS
}

/// `length` a little longer for the `n`th edit, by steps big enough that
/// every edit is a new value even where an `f32` is coarse, and small
/// enough that no picket is added over all of them.
fn longer(length: f32, n: u32) -> f32 {
    let step = f32::from_bits(length.to_bits() + 1) - length;
    length + n as f32 * step.max(1e-4)
}

fn words<T: Pod>(values: &[T]) -> Vec<u32> {
    bytemuck::cast_slice(values).to_vec()
}

/// The curves every scenario starts from, out of the parameter `points`.
fn curves(b: &mut ProgramBuilder) -> SlotId {
    let points = b.param_grouped("points", ControlPoint::INFO, &[]);
    let knots = b.constant::<f32>(&[]);
    b.node(&NURBS_BATCH, &[points, knots]).unwrap()[0]
}

/// A call of NURBS and Translate for every curve of the parameter
/// `points`, all by the parameter `offset`: many small runs, so what each
/// costs besides its kernels shows.
fn each_curve(b: &mut ProgramBuilder, programs: &mut ProgramSet) -> SlotId {
    let mut inner = ProgramBuilder::new("one curve");
    let segments = curves(&mut inner);
    let offset = inner.param_one_of::<Vec3>("offset", Vec3::ZERO);
    let moved = inner.node(&TRANSLATE, &[segments, offset]).unwrap()[0];
    inner.output("segments", moved);
    let inner = programs.add(inner.finish());

    let points = b.param_grouped("points", ControlPoint::INFO, &[]);
    let offset = b.param_one_of::<Vec3>("offset", Vec3::ZERO);
    b.for_each(programs, inner, &[Some(points), Some(offset)], 0, 1 << 16)
        .unwrap()[0]
}

/// A thin tube around a circle of 13 points, 10 patches, put onto 64
/// frames along every curve.
fn rings_on_curves(b: &mut ProgramBuilder, registry: &NodeRegistry) -> SlotId {
    let spine = curves(b);
    let count = b.constant::<u32>(&[SAMPLES as u32]);
    let spacing = b.constant::<f32>(&[0.0]);
    let t = b.constant::<f32>(&[]);
    let frames = b.node(&SAMPLE_CURVE_BATCH, &[spine, count, spacing, t]).unwrap()[0];

    let around: Vec<ControlPoint> = (0..=12)
        .map(|i| {
            let angle = i as f32 / 12.0 * std::f32::consts::TAU;
            ControlPoint::at(Vec3::new(angle.cos(), angle.sin(), 0.0) * 0.45)
        })
        .collect();
    let ring = b.constant(&around);
    let knots = b.constant::<f32>(&[]);
    let ring = b.node(&NURBS, &[ring, knots]).unwrap()[0];
    let radius = b.constant::<f32>(&[0.06]);
    let circle = b.node(&CIRCLE, &[radius]).unwrap()[0];
    let torus = swept(b, registry, ring, circle).unwrap();
    let scale = b.constant::<f32>(&[1.0]);
    oneiroi_nodes::with_defaults(b, registry, &INSTANCE, &[frames, torus, scale]).unwrap()[0]
}

/// The control points of every curve, with the first one of the first
/// curve moved by `shift`. Every curve bends in every direction, and they
/// lie next to each other.
fn control_points(size: Size, shift: f32) -> Vec<ControlPoint> {
    let mut points = Vec::with_capacity((size.curves * size.points) as usize);
    for curve in 0..size.curves {
        let phase = curve as f32 * 0.37;
        for i in 0..size.points {
            let t = i as f32;
            points.push(ControlPoint::at(Vec3::new(
                (t * 0.9 + phase).sin() * 2.0 + curve as f32 * 5.0,
                t * 0.7,
                (t * 1.3 + phase).cos(),
            )));
        }
    }
    points[0].position.x += shift;
    points
}

fn point_params(size: Size, shift: f32) -> Params {
    let offsets: Vec<u32> = (0..=size.curves).map(|curve| curve * size.points).collect();
    vec![
        ("points", words(&control_points(size, shift))),
        ("points#groups", offsets),
    ]
}

fn points(size: Size) -> Params {
    point_params(size, 0.0)
}

fn moved_points(size: Size, n: u32) -> Params {
    // The offsets stay, only the points change.
    vec![("points", words(&control_points(size, n as f32 * 1e-3)))]
}

/// Where Sample Curve puts its frames for edit `n`: spread over each
/// curve, a little closer to its start with every edit.
fn places(n: u32) -> Vec<f32> {
    let squeeze = 1.0 - n as f32 * 1e-6;
    (0..SAMPLES)
        .map(|i| i as f32 / (SAMPLES - 1) as f32 * squeeze)
        .collect()
}

/// A square with sharp corners as a NURBS curve, counter-clockwise.
fn square(scale: f32) -> Vec<ControlPoint> {
    let corners = [(0.1, -0.1), (0.1, 0.1), (-0.1, 0.1), (-0.1, -0.1), (0.1, -0.1)];
    let mut points = vec![ControlPoint::at(Vec3::new(0.1, -0.1, 0.0) * scale)];
    for pair in corners.windows(2) {
        let (a, b) = (Vec2::from(pair[0]), Vec2::from(pair[1]));
        for step in 1..=3 {
            let point = a.lerp(b, step as f32 / 3.0) * scale;
            points.push(ControlPoint::at(point.extend(0.0)));
        }
    }
    points
}

fn square_knots() -> Vec<f32> {
    let mut knots = vec![0.0; 4];
    for knot in [0.25, 0.5, 0.75] {
        knots.extend([knot; 3]);
    }
    knots.extend([1.0; 4]);
    knots
}
