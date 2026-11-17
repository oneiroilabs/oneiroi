//! Draws surfaces straight from their patches and compares the picture
//! with the one of the mesh the CPU makes of the same surface.
//!
//! These tests need a graphics adapter. A software one is enough. Without
//! any they pass without having checked anything.

use glam::{Vec2, Vec3};
use oneiroi_core::{
    exec::{Engine, Instance},
    node::NodeRegistry,
    program::{ProgramBuilder, SlotId},
};
use oneiroi_nodes::{
    curve::{NURBS, SWEEP, SweptPatch, TubePatch, nurbs::clamped_uniform_knots},
    primitive::CIRCLE,
    surface,
};
use oneiroi_wgpu::{
    AutoDetail, DeferredExecutor, DrawStage, DrawView, GpuExecutor, MeshData, MeshPreview,
    orbit::OrbitCamera,
};

const SIZE: u32 = 128;
const BACKGROUND: [u8; 4] = [0, 0, 255, 255];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
/// How close the mesh of a surface is to it.
const TOLERANCE: f32 = 0.005;

/// A device, with mesh shaders if the adapter has them.
fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()));
    let Ok(adapter) = adapter else {
        eprintln!("no graphics adapter, nothing was checked");
        return None;
    };
    let wanted = wgpu::Features::EXPERIMENTAL_MESH_SHADER | oneiroi_wgpu::FEATURES;
    let descriptor = wgpu::DeviceDescriptor {
        required_features: adapter.features() & wanted,
        required_limits: adapter.limits(),
        // SAFETY: mesh shaders are what is being tested here.
        experimental_features: unsafe { wgpu::ExperimentalFeatures::enabled() },
        ..Default::default()
    };
    pollster::block_on(adapter.request_device(&descriptor)).ok()
}

fn registry() -> NodeRegistry {
    let mut registry = NodeRegistry::new();
    oneiroi_nodes::register(&mut registry);
    registry
}

/// A square with sharp corners as a NURBS curve, counter-clockwise.
fn square(b: &mut ProgramBuilder) -> SlotId {
    let corners = [(0.3, -0.3), (0.3, 0.3), (-0.3, 0.3), (-0.3, -0.3), (0.3, -0.3)];
    let mut points = vec![[0.3, -0.3, 0., 1.]];
    for pair in corners.windows(2) {
        let (from, to) = (Vec2::from(pair[0]), Vec2::from(pair[1]));
        for step in 1..=3 {
            let point = from.lerp(to, step as f32 / 3.0);
            points.push([point.x, point.y, 0., 1.]);
        }
    }
    let mut knots = vec![0.0; 4];
    for knot in [0.25, 0.5, 0.75] {
        knots.extend([knot; 3]);
    }
    knots.extend([1.0; 4]);
    let (points, knots) = (b.constant(&points), b.constant(&knots));
    b.node(&NURBS, &[points, knots]).unwrap()[0]
}

const BENT: [[f32; 4]; 5] = [
    [-2., -1., 0., 1.],
    [-1., 1., 0., 1.],
    [0., -1., 1., 1.],
    [1., 1., 0., 1.],
    [2., -1., 0., 1.],
];

/// A profile swept along a bent curve, as a surface and as its mesh.
fn program(registry: &NodeRegistry, circle: bool) -> oneiroi_core::program::Program {
    program_along(registry, circle, &BENT)
}

fn program_along(
    registry: &NodeRegistry,
    circle: bool,
    spine: &[[f32; 4]],
) -> oneiroi_core::program::Program {
    let mut b = ProgramBuilder::new("surface");
    let points = b.constant::<[f32; 4]>(spine);
    let knots = b.constant(&clamped_uniform_knots(spine.len()));
    let spine = b.node(&NURBS, &[points, knots]).unwrap()[0];
    let profile = if circle {
        let radius = b.constant::<f32>(&[0.3]);
        b.node(&CIRCLE, &[radius]).unwrap()[0]
    } else {
        square(&mut b)
    };
    let surface = b.generic(registry, &SWEEP, &[spine, profile]).unwrap()[0];
    let tolerance = b.constant::<f32>(&[TOLERANCE]);
    let mesh = surface::mesh(&mut b, registry, surface, tolerance).unwrap();
    b.output("surface", surface);
    b.output("spine", spine);
    b.output("profile", profile);
    for (name, slot) in mesh {
        b.output(name, slot);
    }
    b.finish()
}

/// Lets `draw` draw into a blue texture and returns its pixels.
fn picture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    draw: impl FnOnce(&mut wgpu::CommandEncoder, &wgpu::TextureView),
) -> Vec<[u8; 4]> {
    let extent = wgpu::Extent3d {
        width: SIZE,
        height: SIZE,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    draw(&mut encoder, &view);

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (SIZE * SIZE * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE * 4),
                rows_per_image: None,
            },
        },
        extent,
    );
    queue.submit([encoder.finish()]);

    buffer.map_async(wgpu::MapMode::Read, .., |result| result.unwrap());
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let pixels: Vec<[u8; 4]> = bytemuck::cast_slice(&buffer.get_mapped_range(..).unwrap()).to_vec();

    if let Ok(directory) = std::env::var("ONEIROI_TEST_IMAGES") {
        static DRAWS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let draw = DRAWS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut image = format!("P6 {SIZE} {SIZE} 255\n").into_bytes();
        image.extend(pixels.iter().flat_map(|pixel| &pixel[..3]));
        let name = std::thread::current().name().unwrap_or("draw").to_owned();
        std::fs::write(format!("{directory}/{name}-{draw}.ppm"), image).unwrap();
    }
    pixels
}

fn covered(pixels: &[[u8; 4]]) -> usize {
    pixels.iter().filter(|pixel| **pixel != BACKGROUND).count()
}

/// How many pixels are clearly not the same in both pictures.
fn different(a: &[[u8; 4]], b: &[[u8; 4]]) -> usize {
    a.iter()
        .zip(b)
        .filter(|(a, b)| a.iter().zip(*b).any(|(a, b)| a.abs_diff(*b) > 12))
        .count()
}

struct Scene {
    device: wgpu::Device,
    queue: wgpu::Queue,
    engine: Engine<GpuExecutor>,
    registry: NodeRegistry,
    instance: Instance,
    camera: OrbitCamera,
}

fn scene(circle: bool) -> Option<Scene> {
    scene_along(circle, &BENT)
}

fn scene_along(circle: bool, spine: &[[f32; 4]]) -> Option<Scene> {
    let (device, queue) = device()?;
    let registry = registry();
    let mut engine = Engine::new(GpuExecutor::new(device.clone(), queue.clone()));
    let id = engine.add_program(program_along(&registry, circle, spine));
    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);

    let mut camera = OrbitCamera::default();
    camera.yaw_pitch_delta(0.5, -0.4);
    camera.frame(Vec3::ZERO, 7.0);
    Some(Scene {
        device,
        queue,
        engine,
        registry,
        instance,
        camera,
    })
}

impl Scene {
    fn view(&self, rings: u32, sides: u32) -> DrawView {
        DrawView {
            view_projection: self.camera.view_projection(1.0),
            light: self.camera.position() - self.camera.target(),
            color: Vec3::new(0.8, 0.7, 0.6),
            rings,
            sides,
            auto: None,
        }
    }

    /// As fine as it appears, with steps of `pixels` on screen.
    fn auto(&self, pixels: f32, debug: bool) -> DrawView {
        DrawView {
            auto: Some(AutoDetail {
                pixels,
                max_level: 6,
                viewport: (SIZE as f32, SIZE as f32),
                debug,
                count: true,
            }),
            ..self.view(1, 1)
        }
    }

    /// The surface drawn straight from its patches.
    fn drawn(&mut self, stage: &mut DrawStage, view: DrawView) -> Vec<[u8; 4]> {
        self.output(stage, "surface", view)
    }

    /// Any output which can be drawn as it is.
    fn output(&mut self, stage: &mut DrawStage, name: &str, view: DrawView) -> Vec<[u8; 4]> {
        let key = self.engine.output_key(&self.instance, name).unwrap();
        let program = self.engine.programs().get(self.instance.program());
        let output = &program.outputs[program.output_index(name).unwrap()];
        let ty = program.slot(output.slot).ty;
        let drawable = *self.registry.drawable(&ty).unwrap();

        let (store, executor) = self.engine.parts_mut();
        executor.sync(store);
        let handle = store.get(key).unwrap();
        let page = executor.page(handle.page).unwrap();
        stage.prepare(&self.device, &self.queue, (SIZE, SIZE), &drawable, page, handle, &view);
        picture(&self.device, &self.queue, |encoder, target| {
            stage.render(encoder, target, None)
        })
    }

    /// How fine the CPU made the mesh of the surface.
    fn detail(&mut self, circle: bool) -> (u32, u32) {
        let (rings, sides) = match circle {
            true => {
                let patches = self.engine.fetch::<TubePatch>(&self.instance, "surface").unwrap();
                surface::detail(patches, TOLERANCE)
            }
            false => {
                let patches = self.engine.fetch::<SweptPatch>(&self.instance, "surface").unwrap();
                surface::detail(patches, TOLERANCE)
            }
        };
        (rings as u32, sides as u32)
    }

    /// The mesh the CPU made of the same surface.
    fn meshed(&mut self) -> Vec<[u8; 4]> {
        let positions = self.engine.fetch::<Vec3>(&self.instance, "positions").unwrap().to_vec();
        let normals = self.engine.fetch::<Vec3>(&self.instance, "normals").unwrap().to_vec();
        let indices = self.engine.fetch::<u32>(&self.instance, "indices").unwrap().to_vec();

        let mut preview = MeshPreview::new(&self.device, FORMAT);
        preview.set_mesh(
            &self.device,
            &self.queue,
            Some(MeshData {
                positions: &positions,
                normals: Some(&normals),
                indices: &indices,
            }),
        );
        let view = self.view(0, 0);
        preview.prepare(&self.device, &self.queue, (SIZE, SIZE), view.view_projection, view.light);
        picture(&self.device, &self.queue, |encoder, target| {
            preview.render(encoder, target, None)
        })
    }
}

#[test]
fn a_surface_drawn_from_its_patches_looks_like_its_mesh() {
    for circle in [true, false] {
        let Some(mut scene) = scene(circle) else {
            return;
        };
        let mut stage = DrawStage::new(&scene.device, FORMAT);
        // As fine as the mesh is, so both are the same triangles.
        let (rings, sides) = scene.detail(circle);
        let drawn = scene.drawn(&mut stage, scene.view(rings, sides));
        let meshed = scene.meshed();
        assert!(stage.errors().is_empty(), "{:?}", stage.errors());

        // Clearly there, and the same picture up to a few pixels on edges.
        let lit = covered(&meshed);
        assert!(lit > 500, "{lit}");
        let off = different(&drawn, &meshed);
        assert!(off * 50 < lit, "{off} of {lit} pixels differ");
    }
}

#[test]
fn detail_is_decided_when_drawing() {
    let Some(mut scene) = scene(true) else {
        return;
    };
    let mut stage = DrawStage::new(&scene.device, FORMAT);
    let nodes = scene.engine.executor().stats();

    // The same patches, drawn coarse and fine.
    let coarse = scene.drawn(&mut stage, scene.view(2, 3));
    let fine = scene.drawn(&mut stage, scene.view(40, 40));
    assert!(covered(&coarse) > 200 && covered(&fine) > covered(&coarse));
    assert!(different(&coarse, &fine) > 100);
    // Nothing ran for it and nothing was brought back from the device.
    let stats = scene.engine.executor().stats();
    assert_eq!((stats.gpu, stats.cpu, stats.downloads), (nodes.gpu, nodes.cpu, nodes.downloads));

    // Drawing the same thing again needs no new bind group.
    let made = stage.bind_groups();
    for _ in 0..3 {
        scene.drawn(&mut stage, scene.view(40, 40));
    }
    assert_eq!(stage.bind_groups(), made);

    stage.clear();
    let nothing = picture(&scene.device, &scene.queue, |encoder, target| {
        stage.render(encoder, target, None)
    });
    assert_eq!(covered(&nothing), 0);
}

#[test]
fn curves_and_profiles_are_drawn_as_lines() {
    let Some(mut scene) = scene(true) else {
        return;
    };
    let mut stage = DrawStage::new(&scene.device, FORMAT);
    let tube = scene.drawn(&mut stage, scene.view(12, 10));

    // The curve runs through the middle of the tube around it.
    let curve = scene.output(&mut stage, "spine", scene.view(32, 1));
    let on_curve = covered(&curve);
    assert!(on_curve > 40 && on_curve * 3 < covered(&tube), "{on_curve}");
    let inside = curve
        .iter()
        .zip(&tube)
        .filter(|(line, tube)| **line != BACKGROUND && **tube != BACKGROUND)
        .count();
    assert!(inside * 10 > on_curve * 9, "{inside} of {on_curve}");

    // The circle is a small closed line around the origin.
    scene.camera.frame(Vec3::ZERO, 1.5);
    let circle = scene.output(&mut stage, "profile", scene.view(48, 1));
    let on_circle = covered(&circle);
    assert!(on_circle > 60, "{on_circle}");
    let middle = (SIZE * SIZE / 2 + SIZE / 2) as usize;
    assert_eq!(circle[middle], BACKGROUND);

    assert!(stage.errors().is_empty(), "{:?}", stage.errors());
}

#[test]
fn drawing_stays_inside_its_bounds() {
    let Some(mut scene) = scene(true) else {
        return;
    };
    let mut stage = DrawStage::new(&scene.device, FORMAT);
    scene.drawn(&mut stage, scene.view(12, 10));

    let half = SIZE as f32 / 2.0;
    let pixels = picture(&scene.device, &scene.queue, |encoder, target| {
        stage.render(encoder, target, Some((half, 0.0, half, SIZE as f32)))
    });
    assert!(covered(&pixels) > 100);
    for row in pixels.chunks_exact(SIZE as usize) {
        assert_eq!(covered(&row[..SIZE as usize / 2]), 0);
    }
}

#[test]
fn a_device_can_turn_up_later() {
    let Some((device, queue)) = device() else {
        return;
    };
    let registry = registry();
    let mut engine = Engine::new(DeferredExecutor::default());
    let id = engine.add_program(program(&registry, true));
    let mut instance = engine.instantiate(id);

    // Planned without a device, as an application does it.
    assert_eq!(engine.plan([&mut instance]), 4);
    assert!(engine.executor().gpu().is_none());

    engine.executor_mut().attach(device.clone(), queue.clone());
    assert_eq!(engine.flush(), 4);

    let key = engine.output_key(&instance, "surface").unwrap();
    let (store, executor) = engine.parts_mut();
    let gpu = executor.gpu_mut().unwrap();
    gpu.sync(store);
    let handle = store.get(key).unwrap();
    assert!(gpu.page(handle.page).is_some() && handle.len == 2 * 28);
}

/// Both ways of drawing a surface as fine as it appears: with vertex
/// shaders only, and with mesh shaders if the device has them.
fn stages(scene: &Scene) -> Vec<(&'static str, DrawStage)> {
    let mut stages = vec![(
        "vertex pulling",
        DrawStage::new(&scene.device, FORMAT).without_mesh_shaders(),
    )];
    let stage = DrawStage::new(&scene.device, FORMAT);
    if stage.uses_mesh_shaders() {
        stages.push(("mesh shaders", stage));
    } else {
        eprintln!("no mesh shaders, only vertex pulling was checked");
    }
    stages
}

/// How many pixels are missing in the middle of what should be there: a
/// gap between two patches shows up as background inside the surface.
fn holes(drawn: &[[u8; 4]], reference: &[[u8; 4]]) -> usize {
    let size = SIZE as usize;
    let solid = |x: usize, y: usize| reference[y * size + x] != BACKGROUND;
    let mut holes = 0;
    for y in 2..size - 2 {
        for x in 2..size - 2 {
            // Well inside: everything around it is part of the surface.
            let inside = (y - 2..=y + 2).all(|y| (x - 2..=x + 2).all(|x| solid(x, y)));
            if inside && drawn[y * size + x] == BACKGROUND {
                holes += 1;
            }
        }
    }
    holes
}

/// How much red and how much blue there is in what was drawn.
fn red_and_blue(pixels: &[[u8; 4]]) -> (u32, u32) {
    pixels
        .iter()
        .filter(|pixel| **pixel != BACKGROUND)
        .fold((0, 0), |(red, blue), pixel| {
            (red + pixel[0] as u32, blue + pixel[2] as u32)
        })
}

#[test]
fn detail_follows_the_size_on_screen() {
    for circle in [true, false] {
        let Some(mut scene) = scene(circle) else {
            return;
        };
        // What it should look like: the surface drawn very fine.
        let mut fixed = DrawStage::new(&scene.device, FORMAT);
        let near = scene.drawn(&mut fixed, scene.view(64, 64));
        let lit = covered(&near);

        for (name, mut stage) in stages(&scene) {
            // Small steps look like the fine picture up to its outline,
            // without a gap anywhere between the patches.
            let fine = scene.drawn(&mut stage, scene.auto(3.0, false));
            let off = different(&fine, &near);
            assert!(off * 10 < lit, "{name}: {off} of {lit} pixels differ");
            assert_eq!(holes(&fine, &near), 0, "{name}");

            // Steps as long as the whole picture leave as few as there
            // can be: one per edge, and four around a closed profile.
            let coarse = scene.drawn(&mut stage, scene.auto(4096.0, false));
            let least = scene.drawn(&mut fixed, scene.view(1, if circle { 4 } else { 1 }));
            assert!(different(&coarse, &least) * 40 < lit, "{name}");
            assert!(different(&coarse, &near) > lit / 10, "{name}");

            assert!(stage.errors().is_empty(), "{name}: {:?}", stage.errors());
        }
    }
}

/// How many pixels in the middle of the picture are not drawn.
fn open_in_the_middle(pixels: &[[u8; 4]]) -> usize {
    let size = SIZE as usize;
    let (from, to) = (size / 2 - 16, size / 2 + 16);
    (from..to)
        .flat_map(|y| (from..to).map(move |x| y * size + x))
        .filter(|index| pixels[*index] == BACKGROUND)
        .count()
}

#[test]
fn there_are_no_cracks_between_patches() {
    for circle in [true, false] {
        let Some(mut scene) = scene(circle) else {
            return;
        };
        // Right where the two segments of the curve meet, so close that
        // the surface covers the middle of the picture. The near patch is
        // much finer there than the far one next to it.
        let curve = oneiroi_nodes::curve::nurbs::CubicNurbs::new(
            [[-2., -1., 0., 1.], [-1., 1., 0., 1.], [0., -1., 1., 1.], [1., 1., 0., 1.], [2., -1., 0., 1.]]
                .into_iter()
                .map(glam::Vec4::from_array)
                .collect(),
            clamped_uniform_knots(5),
        );
        let seam = curve.segments()[0].frame(1.0);
        scene.camera.frame(seam.position, 1.1);

        let mut fixed = DrawStage::new(&scene.device, FORMAT);
        let reference = scene.drawn(&mut fixed, scene.view(64, 64));
        assert_eq!(open_in_the_middle(&reference), 0, "the surface covers the middle");

        for (name, mut stage) in stages(&scene) {
            for pixels in [3.0, 6.0, 10.0, 17.0, 30.0, 60.0] {
                let drawn = scene.drawn(&mut stage, scene.auto(pixels, true));
                assert_eq!(open_in_the_middle(&drawn), 0, "{name} at {pixels}");
            }
        }
    }
}

#[test]
fn what_is_far_away_is_drawn_coarser() {
    let Some(mut scene) = scene(true) else {
        return;
    };
    for (name, mut stage) in stages(&scene) {
        // Coloured by how fine each patch is drawn: red is fine.
        scene.camera.frame(Vec3::ZERO, 5.0);
        let near = red_and_blue(&scene.drawn(&mut stage, scene.auto(8.0, true)));
        scene.camera.frame(Vec3::ZERO, 40.0);
        let far = red_and_blue(&scene.drawn(&mut stage, scene.auto(8.0, true)));

        assert!(near.0 > near.1, "{name}: near is fine, {near:?}");
        assert!(far.1 > far.0, "{name}: far is coarse, {far:?}");
    }
}

#[test]
fn both_ways_draw_the_same_picture() {
    let Some(mut scene) = scene(false) else {
        return;
    };
    let mut stages = stages(&scene);
    if stages.len() < 2 {
        return;
    }
    // From close by and at an angle, where the patches differ in detail.
    scene.camera.yaw_pitch_delta(0.9, 0.2);
    scene.camera.frame(Vec3::new(1.0, 0.0, 0.0), 4.0);
    for pixels in [4.0, 12.0, 40.0] {
        let pulled = scene.drawn(&mut stages[0].1, scene.auto(pixels, false));
        let meshed = scene.drawn(&mut stages[1].1, scene.auto(pixels, false));
        let lit = covered(&pulled);
        assert!(lit > 500);
        let off = different(&pulled, &meshed);
        assert!(off * 50 < lit, "{off} of {lit} pixels differ at {pixels}");
    }
}

#[test]
fn what_is_out_of_view_is_not_drawn() {
    let Some(mut scene) = scene(true) else {
        return;
    };
    for (name, mut stage) in stages(&scene) {
        // The camera looks at something far to the side of the tube.
        scene.camera.frame(Vec3::new(0.0, 60.0, 0.0), 7.0);
        let away = scene.drawn(&mut stage, scene.auto(8.0, false));
        assert_eq!(covered(&away), 0, "{name}");

        // Half of it in view is half of the fine picture.
        scene.camera.frame(Vec3::new(2.0, 0.0, 0.0), 3.0);
        let mut fixed = DrawStage::new(&scene.device, FORMAT);
        let reference = scene.drawn(&mut fixed, scene.view(64, 64));
        let partly = scene.drawn(&mut stage, scene.auto(3.0, false));
        let lit = covered(&reference);
        assert!(lit > 500);
        assert!(different(&partly, &reference) * 40 < lit, "{name}");
    }
}

/// Prepares view `index` of `stage` with what the scene shows and counts
/// its triangles once they arrived.
fn counted(scene: &mut Scene, stage: &mut DrawStage, pixels: f32) -> u32 {
    let view = scene.auto(pixels, false);
    scene.drawn(stage, view);
    scene.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    stage.triangles(&scene.device, 0).expect("they were counted")
}

#[test]
fn triangles_are_counted() {
    for circle in [true, false] {
        let Some(mut scene) = scene(circle) else {
            return;
        };
        for (name, mut stage) in stages(&scene) {
            // As coarse as it gets: one step along each of the 2 segments,
            // and around either a circle of 4 steps or 4 straight edges.
            assert_eq!(counted(&mut scene, &mut stage, 4096.0), 16, "{name}");

            let fine = counted(&mut scene, &mut stage, 4.0);
            let coarse = counted(&mut scene, &mut stage, 16.0);
            assert!(fine > 4 * coarse / 2 && coarse > 16, "{name}: {fine} {coarse}");

            // What is out of view has none.
            let camera = scene.camera;
            scene.camera.frame(Vec3::new(0.0, 60.0, 0.0), 7.0);
            assert_eq!(counted(&mut scene, &mut stage, 4.0), 0, "{name}");
            scene.camera = camera;
        }
    }
}

#[test]
fn several_views_draw_the_same_patches() {
    let Some(mut scene) = scene(true) else {
        return;
    };
    for (name, mut stage) in stages(&scene) {
        // Every view alone, as a reference.
        let front = scene.camera;
        let front_view = scene.auto(6.0, false);
        let alone_front = scene.drawn(&mut stage, front_view);
        scene.camera.yaw_pitch_delta(1.4, 0.6);
        let side_view = scene.auto(6.0, false);
        let alone_side = scene.drawn(&mut stage, side_view);
        assert!(different(&alone_front, &alone_side) > 200, "{name}");

        // Both prepared at once from the same patches, and drawn after.
        let key = scene.engine.output_key(&scene.instance, "surface").unwrap();
        let program = scene.engine.programs().get(scene.instance.program());
        let drawable = *scene.registry.drawable(&program.slot(program.outputs[0].slot).ty).unwrap();
        let ran = scene.engine.executor().stats();
        let (store, executor) = scene.engine.parts_mut();
        executor.sync(store);
        let handle = store.get(key).unwrap();
        let page = executor.page(handle.page).unwrap();
        for (index, view) in [front_view, side_view].iter().enumerate() {
            stage.prepare_view(index, &scene.device, &scene.queue, (SIZE, SIZE), &drawable, page, handle, view);
        }
        let both: Vec<_> = (0..2)
            .map(|index| {
                picture(&scene.device, &scene.queue, |encoder, target| {
                    stage.render_view(index, encoder, target, None)
                })
            })
            .collect();
        assert_eq!(different(&both[0], &alone_front), 0, "{name}");
        assert_eq!(different(&both[1], &alone_side), 0, "{name}");
        // Nothing was made again for the second view.
        let stats = scene.engine.executor().stats();
        assert_eq!((stats.gpu, stats.cpu, stats.downloads), (ran.gpu, ran.cpu, ran.downloads));
        scene.camera = front;
    }
}

#[test]
fn many_patches_are_drawn_alike_both_ways() {
    // A long winding curve: 37 segments, more than two task groups' worth.
    let spine: Vec<_> = (0..40)
        .map(|i| {
            let t = i as f32 * 0.35;
            [t.cos() * 2.0, t * 0.3 - 2.0, t.sin() * 2.0, 1.0]
        })
        .collect();
    for circle in [true, false] {
        let Some(mut scene) = scene_along(circle, &spine) else {
            return;
        };
        let mut stages = stages(&scene);
        if stages.len() < 2 {
            return;
        }
        // Close by, where some patches are cut into blocks, and far away,
        // where many small ones share a mesh shader.
        for (distance, pixels) in [(4.0, 4.0), (9.0, 8.0), (40.0, 6.0)] {
            scene.camera.frame(Vec3::ZERO, distance);
            let pulled = scene.drawn(&mut stages[0].1, scene.auto(pixels, false));
            let meshed = scene.drawn(&mut stages[1].1, scene.auto(pixels, false));
            let lit = covered(&pulled);
            assert!(lit > 100, "{lit}");
            let off = different(&pulled, &meshed);
            assert!(off * 50 < lit, "{off} of {lit} pixels differ at {distance}");

            let a = counted(&mut scene, &mut stages[0].1, pixels);
            let b = counted(&mut scene, &mut stages[1].1, pixels);
            assert_eq!(a, b);
        }
        assert!(stages[1].1.errors().is_empty());
    }
}
