//! Draws values of the store as they are, without a mesh in between.

use std::collections::HashMap;

use glam::{Mat4, Vec3};
use oneiroi_core::{
    node::{Drawable, Topology},
    shader::{self, DrawArgs, ShaderError},
    store::Handle,
};

use crate::mesh::{depth_texture, restrict};

/// From where and how finely something is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawView {
    pub view_projection: Mat4,
    /// Points from what is drawn towards the light.
    pub light: Vec3,
    pub color: Vec3,
    /// Steps along every element.
    pub rings: u32,
    /// Steps across every patch of a surface.
    pub sides: u32,
}

struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

struct Prepared {
    ty: &'static str,
    bind_group: wgpu::BindGroup,
    vertices: u32,
    instances: u32,
}

/// Draws the elements of a [`Drawable`] type straight out of the page they
/// are stored in. Every vertex is computed in the shader from the element
/// and the index it is drawn with, so nothing but the elements themselves
/// is ever stored or uploaded.
///
/// Like [`MeshPreview`](crate::MeshPreview) it draws on top of whatever
/// the target holds, brings its own depth buffer and expects reversed
/// depth.
pub struct DrawStage {
    format: wgpu::TextureFormat,
    /// By the name of the data type. `None` when its shader is broken.
    pipelines: HashMap<&'static str, Option<Pipeline>>,
    errors: Vec<ShaderError>,
    uniform: wgpu::Buffer,
    depth: wgpu::TextureView,
    prepared: Option<Prepared>,
}

impl DrawStage {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Oneiroi Draw Args"),
            size: size_of::<DrawArgs>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            format: surface_format.add_srgb_suffix(),
            pipelines: HashMap::new(),
            errors: Vec::new(),
            uniform,
            depth: depth_texture(device, (1, 1)),
            prepared: None,
        }
    }

    /// The types whose shader did not compile. They are not drawn.
    pub fn errors(&self) -> &[ShaderError] {
        &self.errors
    }

    fn build(&self, device: &wgpu::Device, drawable: &Drawable, wgsl: &str) -> Pipeline {
        let buffer = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(drawable.ty.name),
            entries: &[
                buffer(0, wgpu::BufferBindingType::Storage { read_only: true }),
                buffer(1, wgpu::BufferBindingType::Uniform),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(drawable.ty.name),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(drawable.ty.name),
            source: wgpu::ShaderSource::Wgsl(wgsl.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(drawable.ty.name),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: match drawable.topology {
                    Topology::Surface => wgpu::PrimitiveTopology::TriangleList,
                    Topology::Line => wgpu::PrimitiveTopology::LineList,
                },
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
        Pipeline { pipeline, layout }
    }

    /// Draws nothing until something is prepared again.
    pub fn clear(&mut self) {
        self.prepared = None;
    }

    /// Call once per frame before [`DrawStage::render`], with the value to
    /// draw: `handle` says where its elements are inside `page`, the
    /// buffer which mirrors that page of the store. `target_size` is the
    /// size of the whole target in pixels.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_size: (u32, u32),
        drawable: &Drawable,
        page: &wgpu::Buffer,
        handle: Handle,
        view: &DrawView,
    ) {
        self.prepared = None;
        let texture = self.depth.texture();
        if (texture.width(), texture.height()) != (target_size.0.max(1), target_size.1.max(1)) {
            self.depth = depth_texture(device, target_size);
        }

        let name = drawable.ty.name;
        if !self.pipelines.contains_key(name) {
            let pipeline = match shader::draw(drawable) {
                Ok(shader) => Some(self.build(device, drawable, &shader.wgsl)),
                Err(error) => {
                    self.errors.push(error);
                    None
                }
            };
            self.pipelines.insert(name, pipeline);
        }
        let Some(pipeline) = &self.pipelines[name] else {
            return;
        };

        let (rings, sides) = (view.rings.max(1), view.sides.max(1));
        let instances = handle.len / drawable.ty.words;
        if instances == 0 {
            return;
        }
        let args = DrawArgs {
            view_projection: view.view_projection.to_cols_array_2d(),
            light: view.light.extend(0.0).to_array(),
            color: view.color.extend(1.0).to_array(),
            offset: handle.offset,
            rings,
            sides,
            _padding: 0,
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&args));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(name),
            layout: &pipeline.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: page.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.uniform.as_entire_binding(),
                },
            ],
        });
        self.prepared = Some(Prepared {
            ty: name,
            bind_group,
            vertices: match drawable.topology {
                Topology::Surface => rings * sides * 6,
                Topology::Line => rings * 2,
            },
            instances,
        });
    }

    /// Draws what was prepared into `bounds` (x, y, width, height in
    /// pixels) of `target`, or all of it.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        bounds: Option<(f32, f32, f32, f32)>,
    ) {
        let Some(prepared) = &self.prepared else {
            return;
        };
        let Some(Some(pipeline)) = self.pipelines.get(prepared.ty) else {
            return;
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Oneiroi Draw Pass"),
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
        if !restrict(&mut pass, &self.depth, bounds) {
            return;
        }

        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &prepared.bind_group, &[]);
        pass.draw(0..prepared.vertices, 0..prepared.instances);
    }
}
