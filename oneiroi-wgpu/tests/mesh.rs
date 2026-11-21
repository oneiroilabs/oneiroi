//! Draws the mesh of a graph into a texture and looks at the pixels.
//!
//! These tests need a graphics adapter. A software one is enough. Without
//! any they pass without having checked anything.

use glam::Vec3;
use oneiroi_core::{exec::Engine, node::NodeRegistry, program::ProgramBuilder};
use oneiroi_nodes::{
    curve::{ControlPoint, NURBS, nurbs::clamped_uniform_knots, tube},
    surface::MESH,
};
use oneiroi_wgpu::{MeshData, MeshPreview, orbit::OrbitCamera};

const SIZE: u32 = 128;
const BACKGROUND: [u8; 4] = [0, 0, 255, 255];

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

/// The positions, normals and indices of a tube along a bent curve.
fn tube_mesh() -> (Vec<Vec3>, Vec<Vec3>, Vec<u32>) {
    let mut registry = NodeRegistry::new();
    oneiroi_nodes::register(&mut registry);
    let mut engine = Engine::default();
    let tube = engine.add_program(tube(&registry));

    let mut b = ProgramBuilder::new("test");
    let points = b.constant(
        &[
            [-2., -1., 0., 1.],
            [-1., 1., 0., 1.],
            [0., -1., 1., 1.],
            [1., 1., 0., 1.],
            [2., -1., 0., 1.],
        ]
        .map(ControlPoint::from),
    );
    let knots = b.constant(&clamped_uniform_knots(5));
    let spine = b.node(&NURBS, &[points, knots]).unwrap()[0];
    let outputs = b.call(engine.programs(), tube, &[Some(spine), None, None]).unwrap();
    // The tube gives its surface first, then the mesh of it.
    for (port, slot) in MESH.outputs.iter().zip(&outputs[1..]) {
        b.output(port.name, *slot);
    }
    let id = engine.add_program(b.finish());
    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);

    (
        engine.read(&instance, "positions").unwrap().to_vec(),
        engine.read(&instance, "normals").unwrap().to_vec(),
        engine.read(&instance, "indices").unwrap().to_vec(),
    )
}

/// Draws into a blue texture and returns its pixels.
fn draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    preview: &mut MeshPreview,
    bounds: Option<(f32, f32, f32, f32)>,
) -> Vec<[u8; 4]> {
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
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
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let mut camera = OrbitCamera::default();
    if let Some((center, radius)) = preview.bounds() {
        camera.frame(center, radius * 2.5);
    }
    let light = camera.position() - camera.target();
    preview.prepare(device, queue, (SIZE, SIZE), camera.view_projection(1.0), light);

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
    preview.render(&mut encoder, &view, bounds);

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
        let mut image = format!("P6 {SIZE} {SIZE} 255\n").into_bytes();
        image.extend(pixels.iter().flat_map(|pixel| &pixel[..3]));
        static DRAWS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let draw = DRAWS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let name = std::thread::current().name().unwrap_or("mesh").to_owned();
        std::fs::write(format!("{directory}/{name}-{draw}.ppm"), image).unwrap();
    }
    pixels
}

fn covered(pixels: &[[u8; 4]]) -> usize {
    pixels.iter().filter(|pixel| **pixel != BACKGROUND).count()
}

#[test]
fn a_mesh_shows_up() {
    let Some((device, queue)) = device() else {
        return;
    };
    let mut preview = MeshPreview::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);

    // Nothing is drawn while there is no mesh.
    assert_eq!(covered(&draw(&device, &queue, &mut preview, None)), 0);

    let (positions, normals, indices) = tube_mesh();
    preview.set_mesh(
        &device,
        &queue,
        Some(MeshData {
            positions: &positions,
            normals: Some(&normals),
            indices: &indices,
        }),
    );
    let pixels = draw(&device, &queue, &mut preview, None);
    let lit = covered(&pixels);
    // A thin tube across the picture: clearly there, clearly not all of it.
    assert!(lit > 300 && lit < (SIZE * SIZE / 2) as usize, "{lit}");
    // It is lit from the camera, so it is not just a dark silhouette.
    assert!(pixels.iter().any(|pixel| pixel[0] > 150));

    // Without normals they are worked out from the triangles.
    preview.set_mesh(
        &device,
        &queue,
        Some(MeshData {
            positions: &positions,
            normals: None,
            indices: &indices,
        }),
    );
    let flat = draw(&device, &queue, &mut preview, None);
    assert!(covered(&flat).abs_diff(lit) < 20);
    assert!(flat.iter().any(|pixel| pixel[0] > 150));

    preview.set_mesh(&device, &queue, None);
    assert_eq!(covered(&draw(&device, &queue, &mut preview, None)), 0);

    // Showing a mesh again, or a smaller one, needs no new buffers.
    let allocations = preview.allocations();
    for count in [indices.len(), indices.len() / 2, indices.len()] {
        preview.set_mesh(
            &device,
            &queue,
            Some(MeshData {
                positions: &positions,
                normals: Some(&normals),
                indices: &indices[..count],
            }),
        );
    }
    assert_eq!(preview.allocations(), allocations);
    assert!(covered(&draw(&device, &queue, &mut preview, None)) > 300);
}

#[test]
fn drawing_stays_inside_its_bounds() {
    let Some((device, queue)) = device() else {
        return;
    };
    let mut preview = MeshPreview::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let (positions, normals, indices) = tube_mesh();
    preview.set_mesh(
        &device,
        &queue,
        Some(MeshData {
            positions: &positions,
            normals: Some(&normals),
            indices: &indices,
        }),
    );

    // The right half of the picture only.
    let half = SIZE as f32 / 2.0;
    let pixels = draw(&device, &queue, &mut preview, Some((half, 0.0, half, SIZE as f32)));
    assert!(covered(&pixels) > 100);
    for row in pixels.chunks_exact(SIZE as usize) {
        assert_eq!(covered(&row[..SIZE as usize / 2]), 0);
    }

    // Bounds which are partly or completely outside are fine too.
    draw(&device, &queue, &mut preview, Some((100.0, -20.0, 200.0, 200.0)));
    let outside = draw(&device, &queue, &mut preview, Some((500.0, 500.0, 50.0, 50.0)));
    assert_eq!(covered(&outside), 0);
}

#[test]
fn broken_meshes_are_not_drawn() {
    let Some((device, queue)) = device() else {
        return;
    };
    let mut preview = MeshPreview::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let positions = [Vec3::ZERO, Vec3::X, Vec3::Y];
    preview.set_mesh(
        &device,
        &queue,
        Some(MeshData {
            positions: &positions,
            normals: None,
            indices: &[0, 1, 7],
        }),
    );
    assert_eq!(preview.bounds(), None);
    assert_eq!(covered(&draw(&device, &queue, &mut preview, None)), 0);
}
