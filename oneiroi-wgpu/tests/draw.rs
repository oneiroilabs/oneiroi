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
    curve::{NURBS, SWEEP, nurbs::clamped_uniform_knots},
    primitive::CIRCLE,
    surface::TESSELLATE,
};
use oneiroi_wgpu::{
    DeferredExecutor, DrawStage, DrawView, GpuExecutor, MeshData, MeshPreview, orbit::OrbitCamera,
};

const SIZE: u32 = 128;
const BACKGROUND: [u8; 4] = [0, 0, 255, 255];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DETAIL: (u32, u32) = (12, 10);

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()));
    let Ok(adapter) = adapter else {
        eprintln!("no graphics adapter, nothing was checked");
        return None;
    };
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()
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

/// A profile swept along a bent curve, as a surface and as its mesh.
fn program(registry: &NodeRegistry, circle: bool) -> oneiroi_core::program::Program {
    let mut b = ProgramBuilder::new("surface");
    let points = b.constant::<[f32; 4]>(&[
        [-2., -1., 0., 1.],
        [-1., 1., 0., 1.],
        [0., -1., 1., 1.],
        [1., 1., 0., 1.],
        [2., -1., 0., 1.],
    ]);
    let knots = b.constant(&clamped_uniform_knots(5));
    let spine = b.node(&NURBS, &[points, knots]).unwrap()[0];
    let profile = if circle {
        let radius = b.constant::<f32>(&[0.3]);
        b.node(&CIRCLE, &[radius]).unwrap()[0]
    } else {
        square(&mut b)
    };
    let surface = b.generic(registry, &SWEEP, &[spine, profile]).unwrap()[0];
    let rings = b.constant::<u32>(&[DETAIL.0]);
    let sides = b.constant::<u32>(&[DETAIL.1]);
    let mesh = b
        .generic(registry, &TESSELLATE, &[surface, rings, sides])
        .unwrap();
    b.output("surface", surface);
    for (port, slot) in TESSELLATE.outputs.iter().zip(mesh) {
        b.output(port.name, slot);
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
    let (device, queue) = device()?;
    let registry = registry();
    let mut engine = Engine::new(GpuExecutor::new(device.clone(), queue.clone()));
    let id = engine.add_program(program(&registry, circle));
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
        }
    }

    /// The surface drawn straight from its patches.
    fn drawn(&mut self, stage: &mut DrawStage, view: DrawView) -> Vec<[u8; 4]> {
        let key = self.engine.output_key(&self.instance, "surface").unwrap();
        let program = self.engine.programs().get(self.instance.program());
        let ty = program.slot(program.outputs[0].slot).ty;
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

    /// The mesh the CPU made of the same surface.
    fn meshed(&mut self) -> Vec<[u8; 4]> {
        let positions = self.engine.fetch::<Vec3>(&self.instance, "positions").unwrap().to_vec();
        let normals = self.engine.fetch::<Vec3>(&self.instance, "normals").unwrap().to_vec();
        let indices = self.engine.fetch::<u32>(&self.instance, "indices").unwrap().to_vec();

        let mut preview = MeshPreview::new(&self.device, FORMAT);
        preview.set_mesh(
            &self.device,
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
        let drawn = scene.drawn(&mut stage, scene.view(DETAIL.0, DETAIL.1));
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
    assert_eq!(scene.engine.executor().stats(), nodes);

    stage.clear();
    let nothing = picture(&scene.device, &scene.queue, |encoder, target| {
        stage.render(encoder, target, None)
    });
    assert_eq!(covered(&nothing), 0);
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
