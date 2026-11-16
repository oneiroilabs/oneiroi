//! Draws a triangle mesh an engine computed, to look at it.

use glam::{Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;

/// A triangle mesh as the outputs of a graph describe it.
#[derive(Debug, Clone, Copy)]
pub struct MeshData<'a> {
    pub positions: &'a [Vec3],
    /// Computed from the triangles when missing or of the wrong length.
    pub normals: Option<&'a [Vec3]>,
    /// Three per triangle.
    pub indices: &'a [u32],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_projection: Mat4,
    light: Vec4,
    color: Vec4,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

struct Buffers {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
}

/// One normal per position, each the average of the triangles around it.
fn smooth_normals(positions: &[Vec3], indices: &[u32]) -> Vec<Vec3> {
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for triangle in indices.chunks_exact(3) {
        let [a, b, c] = [0, 1, 2].map(|i| triangle[i] as usize);
        if a.max(b).max(c) < positions.len() {
            let face = (positions[b] - positions[a]).cross(positions[c] - positions[a]);
            normals[a] += face;
            normals[b] += face;
            normals[c] += face;
        }
    }
    normals.iter().map(|n| n.normalize_or_zero()).collect()
}

/// A pipeline that draws one mesh with a simple light.
///
/// It draws on top of whatever the target holds and brings its own depth
/// buffer, which is what a widget inside a user interface needs. Depth is
/// reversed like in the rest of this crate: use the view projection of an
/// [`OrbitCamera`](crate::orbit::OrbitCamera).
pub struct MeshPreview {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    depth: wgpu::TextureView,
    buffers: Option<Buffers>,
    bounds: Option<(Vec3, f32)>,
    pub color: Vec3,
}

impl MeshPreview {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("mesh.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Mesh Preview Uniform Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mesh Preview Uniforms"),
            size: size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mesh Preview Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mesh Preview Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Mesh Preview Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format.add_srgb_suffix(),
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Greater),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            depth: Self::depth(device, (1, 1)),
            buffers: None,
            bounds: None,
            color: Vec3::new(0.8, 0.7, 0.6),
        }
    }

    fn depth(device: &wgpu::Device, size: (u32, u32)) -> wgpu::TextureView {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Mesh Preview Depth"),
                size: wgpu::Extent3d {
                    width: size.0.max(1),
                    height: size.1.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Replaces the mesh that is drawn. Call it when the mesh changed, not
    /// every frame: it uploads everything again.
    pub fn set_mesh(&mut self, device: &wgpu::Device, mesh: Option<MeshData<'_>>) {
        self.buffers = None;
        self.bounds = None;
        let Some(mesh) = mesh else {
            return;
        };
        let in_range = |index: &u32| (*index as usize) < mesh.positions.len();
        if mesh.indices.len() < 3 || !mesh.indices.iter().all(in_range) {
            return;
        }

        let computed;
        let normals = match mesh.normals {
            Some(normals) if normals.len() == mesh.positions.len() => normals,
            _ => {
                computed = smooth_normals(mesh.positions, mesh.indices);
                &computed
            }
        };
        let vertices: Vec<_> = mesh
            .positions
            .iter()
            .zip(normals)
            .map(|(position, normal)| Vertex {
                position: position.to_array(),
                normal: normal.to_array(),
            })
            .collect();

        let (min, max) = mesh
            .positions
            .iter()
            .fold((Vec3::MAX, Vec3::MIN), |(min, max), p| (min.min(*p), max.max(*p)));
        self.bounds = Some(((min + max) * 0.5, (max - min).length() * 0.5));

        self.buffers = Some(Buffers {
            vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh Preview Vertices"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh Preview Indices"),
                contents: bytemuck::cast_slice(&mesh.indices[..mesh.indices.len() / 3 * 3]),
                usage: wgpu::BufferUsages::INDEX,
            }),
            index_count: (mesh.indices.len() / 3 * 3) as u32,
        });
    }

    /// Center and radius of a sphere around the mesh, to aim a camera.
    pub fn bounds(&self) -> Option<(Vec3, f32)> {
        self.bounds
    }

    /// Call once per frame before [`MeshPreview::render`]. `target_size` is
    /// the size of the whole target in pixels, `light` points from the
    /// mesh to the light.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_size: (u32, u32),
        view_projection: Mat4,
        light: Vec3,
    ) {
        let texture = self.depth.texture();
        if (texture.width(), texture.height()) != (target_size.0.max(1), target_size.1.max(1)) {
            self.depth = Self::depth(device, target_size);
        }

        let uniforms = Uniforms {
            view_projection,
            light: light.extend(0.0),
            color: self.color.extend(1.0),
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Draws into `bounds` (x, y, width, height in pixels) of `target`, or
    /// all of it.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        bounds: Option<(f32, f32, f32, f32)>,
    ) {
        let Some(buffers) = &self.buffers else {
            return;
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Mesh Preview Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        if let Some((x, y, width, height)) = bounds {
            let texture = self.depth.texture();
            let (max_x, max_y) = (texture.width() as f32, texture.height() as f32);
            let (left, top) = (x.clamp(0.0, max_x), y.clamp(0.0, max_y));
            let (right, bottom) = ((x + width).clamp(0.0, max_x), (y + height).clamp(0.0, max_y));
            if right - left < 1.0 || bottom - top < 1.0 {
                return;
            }
            pass.set_viewport(x, y, width, height, 0.0, 1.0);
            pass.set_scissor_rect(
                left as u32,
                top as u32,
                (right - left) as u32,
                (bottom - top) as u32,
            );
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, buffers.vertices.slice(..));
        pass.set_index_buffer(buffers.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..buffers.index_count, 0, 0..1);
    }
}
