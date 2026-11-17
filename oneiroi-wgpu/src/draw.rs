//! Draws values of the store as they are, without a mesh in between.

use std::collections::HashMap;

use glam::{Mat4, Vec3};
use oneiroi_core::{
    node::{Drawable, Topology},
    shader::{self, DrawArgs, LodArgs, LodMethod, MAX_LOD_LEVEL, ShaderError},
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
    /// Lets a surface choose its own steps by how large it appears. `rings`
    /// and `sides` are then only for what can not do that, like a line.
    pub auto: Option<AutoDetail>,
}

/// As much detail as there is to see: every patch of a surface gets steps
/// of about `pixels` on screen, and more of them as it comes closer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutoDetail {
    /// How long a step should be on screen, in pixels.
    pub pixels: f32,
    /// A patch never gets more than `2^max_level` steps along a side.
    pub max_level: u32,
    /// The size in pixels of the area which is drawn into.
    pub viewport: (f32, f32),
    /// Colours every patch by how fine it is drawn.
    pub debug: bool,
}

/// Which of the shaders of a type something is drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Fixed,
    Lod(LodMethod),
}

struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

enum Draw {
    Vertices { vertices: u32, instances: u32 },
    /// One task shader per patch.
    Tasks { patches: u32 },
}

struct Prepared {
    key: (&'static str, Kind),
    bind_group: wgpu::BindGroup,
    draw: Draw,
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
    /// By the name of the data type and the kind of shader. `None` when
    /// the shader is broken.
    pipelines: HashMap<(&'static str, Kind), Option<Pipeline>>,
    errors: Vec<ShaderError>,
    uniform: wgpu::Buffer,
    lod_uniform: wgpu::Buffer,
    /// Whether the device can run task and mesh shaders.
    mesh_shaders: bool,
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
        let lod_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Oneiroi Lod Args"),
            size: size_of::<LodArgs>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            format: surface_format.add_srgb_suffix(),
            pipelines: HashMap::new(),
            errors: Vec::new(),
            uniform,
            lod_uniform,
            mesh_shaders: device
                .features()
                .contains(wgpu::Features::EXPERIMENTAL_MESH_SHADER),
            depth: depth_texture(device, (1, 1)),
            prepared: None,
        }
    }

    /// The types whose shader did not compile. They are not drawn.
    pub fn errors(&self) -> &[ShaderError] {
        &self.errors
    }

    /// Whether surfaces which choose their own detail are drawn with task
    /// and mesh shaders. That is the case when the device has them.
    pub fn uses_mesh_shaders(&self) -> bool {
        self.mesh_shaders
    }

    /// Draws with vertex shaders only, also where there are mesh shaders.
    pub fn without_mesh_shaders(mut self) -> Self {
        self.mesh_shaders = false;
        self
    }

    fn build(&self, device: &wgpu::Device, drawable: &Drawable, kind: Kind, wgsl: &str) -> Pipeline {
        let mesh = kind == Kind::Lod(LodMethod::MeshShader);
        let visibility = match mesh {
            true => {
                wgpu::ShaderStages::TASK | wgpu::ShaderStages::MESH | wgpu::ShaderStages::FRAGMENT
            }
            false => wgpu::ShaderStages::VERTEX_FRAGMENT,
        };
        let buffer = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
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
        let targets = [Some(wgpu::ColorTargetState {
            format: self.format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let fragment = Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            targets: &targets,
            compilation_options: Default::default(),
        });
        let primitive = wgpu::PrimitiveState {
            topology: match drawable.topology {
                Topology::Surface => wgpu::PrimitiveTopology::TriangleList,
                Topology::Line => wgpu::PrimitiveTopology::LineList,
            },
            cull_mode: None,
            ..Default::default()
        };
        let depth_stencil = Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Greater),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });

        let pipeline = if mesh {
            device.create_mesh_pipeline(&wgpu::MeshPipelineDescriptor {
                label: Some(drawable.ty.name),
                layout: Some(&pipeline_layout),
                task: Some(wgpu::TaskState {
                    module: &module,
                    entry_point: Some("ts_main"),
                    compilation_options: Default::default(),
                }),
                mesh: wgpu::MeshState {
                    module: &module,
                    entry_point: Some("ms_main"),
                    compilation_options: Default::default(),
                },
                fragment,
                primitive,
                depth_stencil,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        } else {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(drawable.ty.name),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment,
                primitive,
                depth_stencil,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
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

        // A surface picks its own detail if it is asked to, with mesh
        // shaders where the device has them.
        let auto = view.auto.filter(|_| drawable.topology == Topology::Surface);
        let kind = match auto {
            Some(_) if self.mesh_shaders => Kind::Lod(LodMethod::MeshShader),
            Some(_) => Kind::Lod(LodMethod::VertexPulling),
            None => Kind::Fixed,
        };
        let key = (drawable.ty.name, kind);
        if !self.pipelines.contains_key(&key) {
            let wgsl = match kind {
                Kind::Fixed => shader::draw(drawable).map(|shader| shader.wgsl),
                Kind::Lod(method) => shader::draw_lod(drawable, method).map(|shader| shader.wgsl),
            };
            let pipeline = match wgsl {
                Ok(wgsl) => Some(self.build(device, drawable, kind, &wgsl)),
                Err(error) => {
                    self.errors.push(error);
                    None
                }
            };
            self.pipelines.insert(key, pipeline);
        }
        let Some(pipeline) = &self.pipelines[&key] else {
            return;
        };

        let instances = handle.len / drawable.ty.words;
        if instances == 0 {
            return;
        }
        let view_projection = view.view_projection.to_cols_array_2d();
        let light = view.light.extend(0.0).to_array();
        let color = view.color.extend(1.0).to_array();

        let (uniform, draw) = match (kind, auto) {
            (Kind::Lod(method), Some(auto)) => {
                // Drawing the finest grid for every patch gets expensive
                // quickly, a mesh shader only makes what is needed.
                let max_level = auto.max_level.min(match method {
                    LodMethod::MeshShader => MAX_LOD_LEVEL,
                    LodMethod::VertexPulling => MAX_LOD_LEVEL - 1,
                });
                let args = LodArgs {
                    view_projection,
                    light,
                    color,
                    viewport: [auto.viewport.0.max(1.0), auto.viewport.1.max(1.0)],
                    pixels: auto.pixels,
                    offset: handle.offset,
                    max_level,
                    debug: auto.debug as u32,
                    _padding: [0; 2],
                };
                queue.write_buffer(&self.lod_uniform, 0, bytemuck::bytes_of(&args));
                let draw = match method {
                    LodMethod::MeshShader => Draw::Tasks { patches: instances },
                    LodMethod::VertexPulling => Draw::Vertices {
                        vertices: shader::LodShader::vertices(max_level),
                        instances,
                    },
                };
                (&self.lod_uniform, draw)
            }
            _ => {
                let (rings, sides) = (view.rings.max(1), view.sides.max(1));
                let args = DrawArgs {
                    view_projection,
                    light,
                    color,
                    offset: handle.offset,
                    rings,
                    sides,
                    _padding: 0,
                };
                queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&args));
                let vertices = match drawable.topology {
                    Topology::Surface => rings * sides * 6,
                    Topology::Line => rings * 2,
                };
                (&self.uniform, Draw::Vertices { vertices, instances })
            }
        };

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(key.0),
            layout: &pipeline.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: page.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform.as_entire_binding(),
                },
            ],
        });
        self.prepared = Some(Prepared {
            key,
            bind_group,
            draw,
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
        let Some(Some(pipeline)) = self.pipelines.get(&prepared.key) else {
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
        match prepared.draw {
            Draw::Vertices { vertices, instances } => pass.draw(0..vertices, 0..instances),
            Draw::Tasks { patches } => pass.draw_mesh_tasks(patches, 1, 1),
        }
    }
}
