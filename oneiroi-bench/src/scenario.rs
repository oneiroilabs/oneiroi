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
    curve::{NURBS, SAMPLE_CURVE, TRANSLATE, swept},
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
    /// The program, and any it calls, which go into `programs`.
    pub build: fn(&NodeRegistry, &mut ProgramSet) -> Program,
    /// The parameters before the first edit.
    pub setup: fn(Size) -> Params,
    /// What edit `n` sets, starting at 1. Every edit sets values which
    /// were not there before, so that nothing is found in the store.
    pub edit: fn(Size, u32) -> Params,
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
        build: |_, _| {
            let mut b = ProgramBuilder::new("nurbs");
            let segments = curves(&mut b);
            b.output("segments", segments);
            b.finish()
        },
        setup: points,
        edit: moved_points,
    },
    Scenario {
        name: "translate",
        about: "Translate Curve of the curves, the offset changes",
        sizes: CURVES,
        quick: CURVES_QUICK,
        build: |_, _| {
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
    Scenario {
        name: "sweep circle",
        about: "Sweep of a circle along the curves, the radius changes",
        sizes: CURVES,
        quick: CURVES_QUICK,
        build: |registry, _| {
            let mut b = ProgramBuilder::new("sweep circle");
            let spine = curves(&mut b);
            let radius = b.param::<f32>("radius", &[0.1]);
            let circle = b.node(&CIRCLE, &[radius]).unwrap()[0];
            let surface = swept(&mut b, registry, spine, circle).unwrap();
            b.output("surface", surface);
            b.finish()
        },
        setup: points,
        edit: |_, n| vec![("radius", words(&[0.1 + n as f32 * 1e-6]))],
    },
    Scenario {
        name: "sweep square",
        about: "Sweep of a square curve along the curves, the square changes",
        sizes: CURVES,
        quick: CURVES_QUICK,
        build: |registry, _| {
            let mut b = ProgramBuilder::new("sweep square");
            let spine = curves(&mut b);
            let square = b.param_grouped("square", <[f32; 4]>::INFO, &[]);
            let knots = b.constant(&square_knots());
            let profile = b.node(&NURBS, &[square, knots]).unwrap()[0];
            let surface = swept(&mut b, registry, spine, profile).unwrap();
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
    Scenario {
        name: "sample",
        about: "Sample Curve, 64 frames per curve at places which change",
        sizes: CURVES,
        quick: CURVES_QUICK,
        build: |_, _| {
            let mut b = ProgramBuilder::new("sample");
            let segments = curves(&mut b);
            let count = b.constant::<u32>(&[SAMPLES as u32]);
            let spacing = b.constant::<f32>(&[0.0]);
            let t = b.param::<f32>("t", &[]);
            let frames = b.node(&SAMPLE_CURVE, &[segments, count, spacing, t]).unwrap()[0];
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
    Scenario {
        name: "chain",
        about: "NURBS, Translate, a Sweep of a circle and Sample Curve, the control points move",
        sizes: CURVES,
        quick: CURVES_QUICK,
        build: |registry, _| {
            let mut b = ProgramBuilder::new("chain");
            let segments = curves(&mut b);
            let offset = b.constant(&[Vec3::new(0.0, 1.0, 0.0)]);
            let moved = b.node(&TRANSLATE, &[segments, offset]).unwrap()[0];
            let radius = b.constant::<f32>(&[0.1]);
            let circle = b.node(&CIRCLE, &[radius]).unwrap()[0];
            let surface = swept(&mut b, registry, moved, circle).unwrap();
            let count = b.constant::<u32>(&[SAMPLES as u32]);
            let spacing = b.constant::<f32>(&[0.0]);
            let t = b.constant::<f32>(&[]);
            let frames = b.node(&SAMPLE_CURVE, &[moved, count, spacing, t]).unwrap()[0];
            b.output("surface", surface);
            b.output("frames", frames);
            b.finish()
        },
        setup: points,
        edit: moved_points,
    },
    Scenario {
        name: "instances",
        about: "A ring of 10 patches put onto 64 frames along every curve by reference, the control points move",
        sizes: INSTANCES,
        quick: INSTANCES_QUICK,
        build: |registry, _| {
            let mut b = ProgramBuilder::new("instances");
            let placed = rings_on_curves(&mut b, registry);
            b.output("instances", placed);
            b.finish()
        },
        setup: points,
        edit: moved_points,
    },
    Scenario {
        name: "instances realized",
        about: "The rings of `instances`, copied to where each one is",
        sizes: INSTANCES,
        quick: INSTANCES_QUICK,
        build: |registry, _| {
            let mut b = ProgramBuilder::new("instances realized");
            let placed = rings_on_curves(&mut b, registry);
            let copies = b.generic(registry, &REALIZE, &[placed]).unwrap()[0];
            b.output("copies", copies);
            b.finish()
        },
        setup: points,
        edit: moved_points,
    },
    Scenario {
        name: "for each",
        about: "NURBS and Translate for every curve on its own, the control points of one curve move",
        sizes: ELEMENTS,
        quick: ELEMENTS_QUICK,
        build: |_, programs| {
            let mut b = ProgramBuilder::new("for each");
            let moved = each_curve(&mut b, programs);
            b.output("segments", moved);
            b.finish()
        },
        setup: points,
        edit: moved_points,
    },
    Scenario {
        name: "for each, all",
        about: "NURBS and Translate for every curve on its own, the offset of all of them changes",
        sizes: ELEMENTS,
        quick: ELEMENTS_QUICK,
        build: |_, programs| {
            let mut b = ProgramBuilder::new("for each, all");
            let moved = each_curve(&mut b, programs);
            b.output("segments", moved);
            b.finish()
        },
        setup: points,
        edit: |_, n| vec![("offset", words(&[Vec3::new(n as f32 * 1e-3, 0.0, 0.0)]))],
    },
    Scenario {
        name: "mesh",
        about: "Mesh of a swept circle, the tolerance changes",
        sizes: MESHES,
        quick: MESHES_QUICK,
        build: |registry, _| {
            let mut b = ProgramBuilder::new("mesh");
            let spine = curves(&mut b);
            let radius = b.constant::<f32>(&[0.1]);
            let circle = b.node(&CIRCLE, &[radius]).unwrap()[0];
            let swept = swept(&mut b, registry, spine, circle).unwrap();
            let tolerance = b.param_one_of::<f32>("tolerance", 0.01);
            for (name, slot) in surface::mesh(&mut b, registry, swept, tolerance).unwrap() {
                b.output(name, slot);
            }
            b.finish()
        },
        setup: points,
        edit: |_, n| vec![("tolerance", words(&[0.01 * (1.0 + n as f32 * 1e-6)]))],
    },
    Scenario {
        name: "fence",
        about: "The picket fence of the editor, with as many pickets, gets a little longer",
        sizes: FENCES,
        quick: FENCES_QUICK,
        build: |registry, programs| {
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
];

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
    let points = b.param_grouped("points", <[f32; 4]>::INFO, &[]);
    let knots = b.constant::<f32>(&[]);
    b.node(&NURBS, &[points, knots]).unwrap()[0]
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

    let points = b.param_grouped("points", <[f32; 4]>::INFO, &[]);
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
    let frames = b.node(&SAMPLE_CURVE, &[spine, count, spacing, t]).unwrap()[0];

    let around: Vec<[f32; 4]> = (0..=12)
        .map(|i| {
            let angle = i as f32 / 12.0 * std::f32::consts::TAU;
            [0.45 * angle.cos(), 0.45 * angle.sin(), 0.0, 1.0]
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
fn control_points(size: Size, shift: f32) -> Vec<[f32; 4]> {
    let mut points = Vec::with_capacity((size.curves * size.points) as usize);
    for curve in 0..size.curves {
        let phase = curve as f32 * 0.37;
        for i in 0..size.points {
            let t = i as f32;
            points.push([
                (t * 0.9 + phase).sin() * 2.0 + curve as f32 * 5.0,
                t * 0.7,
                (t * 1.3 + phase).cos(),
                1.0,
            ]);
        }
    }
    points[0][0] += shift;
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
fn square(scale: f32) -> Vec<[f32; 4]> {
    let corners = [(0.1, -0.1), (0.1, 0.1), (-0.1, 0.1), (-0.1, -0.1), (0.1, -0.1)];
    let mut points = vec![[0.1 * scale, -0.1 * scale, 0.0, 1.0]];
    for pair in corners.windows(2) {
        let (a, b) = (Vec2::from(pair[0]), Vec2::from(pair[1]));
        for step in 1..=3 {
            let point = a.lerp(b, step as f32 / 3.0) * scale;
            points.push([point.x, point.y, 0.0, 1.0]);
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
