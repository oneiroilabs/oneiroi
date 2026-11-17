//! Draws values of the store as they are, without a mesh in between.

use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use glam::{Mat4, Vec3};
use oneiroi_core::{
    node::{Drawable, Topology},
    shader::{self, DrawArgs, LodArgs, LodMethod, LodShader, MAX_LOD_LEVEL, ShaderError},
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
    /// Counts the triangles this makes, see [`DrawStage::triangles`].
    pub count: bool,
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
    /// What counts the triangles of a surface which picks its own detail.
    count: Option<(wgpu::ComputePipeline, wgpu::BindGroupLayout)>,
}

enum Draw {
    Vertices { vertices: u32, instances: u32 },
    /// One task shader for every few patches.
    Tasks { groups: u32 },
}

struct Prepared {
    key: (&'static str, Kind),
    bind_group: wgpu::BindGroup,
    draw: Draw,
}

/// Brings the number of triangles over from the device, without waiting
/// for it: what is read is what an earlier frame made.
struct Counter {
    buffer: wgpu::Buffer,
    readback: wgpu::Buffer,
    /// Set once `readback` can be read.
    ready: Arc<AtomicBool>,
    /// Whether `readback` is on its way, so it can not be used now.
    pending: bool,
    last: Option<u32>,
}

impl Counter {
    fn new(device: &wgpu::Device) -> Self {
        let buffer = |label, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: 4,
                usage,
                mapped_at_creation: false,
            })
        };
        Self {
            buffer: buffer(
                "Oneiroi Triangles",
                wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            ),
            readback: buffer(
                "Oneiroi Triangles Readback",
                wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            ),
            ready: Arc::new(AtomicBool::new(false)),
            pending: false,
            last: None,
        }
    }

    /// Takes over what arrived since the last time.
    fn poll(&mut self, device: &wgpu::Device) -> Option<u32> {
        if self.pending {
            let _ = device.poll(wgpu::PollType::Poll);
            if self.ready.swap(false, Ordering::AcqRel) {
                if let Ok(bytes) = self.readback.get_mapped_range(..) {
                    self.last = Some(bytemuck::pod_read_unaligned(&bytes[..4]));
                }
                self.readback.unmap();
                self.pending = false;
            }
        }
        self.last
    }
}

/// What is drawn into one view.
struct View {
    uniform: wgpu::Buffer,
    lod_uniform: wgpu::Buffer,
    depth: wgpu::TextureView,
    prepared: Option<Prepared>,
    counter: Counter,
    /// The bind group of the last frame, and what it was made for. Pages
    /// and uniforms stay, so it can almost always be used again.
    bound: Option<(BindKey, wgpu::BindGroup)>,
}

/// What a bind group of a view is made of: the pipeline it is for, the
/// page it reads, and which of the two uniforms.
#[derive(Clone, PartialEq)]
struct BindKey {
    pipeline: (&'static str, Kind),
    page: wgpu::Buffer,
    lod: bool,
}

impl View {
    fn new(device: &wgpu::Device) -> Self {
        let uniform = |label, size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        Self {
            uniform: uniform("Oneiroi Draw Args", size_of::<DrawArgs>() as u64),
            lod_uniform: uniform("Oneiroi Lod Args", size_of::<LodArgs>() as u64),
            depth: depth_texture(device, (1, 1)),
            prepared: None,
            counter: Counter::new(device),
            bound: None,
        }
    }
}

/// Draws the elements of a [`Drawable`] type straight out of the page they
/// are stored in. Every vertex is computed in the shader from the element
/// and the index it is drawn with, so nothing but the elements themselves
/// is ever stored or uploaded.
///
/// It can draw into several views, like a camera and the light of a
/// shadow. Every view reads the same elements, so what is drawn is made
/// once however many views look at it.
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
    /// Whether the device can run task and mesh shaders.
    mesh_shaders: bool,
    views: Vec<View>,
    bind_groups: usize,
}

impl DrawStage {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        Self {
            format: surface_format.add_srgb_suffix(),
            pipelines: HashMap::new(),
            errors: Vec::new(),
            // `ONEIROI_MESH_SHADERS=off` draws with vertex shaders anyway.
            mesh_shaders: device
                .features()
                .contains(wgpu::Features::EXPERIMENTAL_MESH_SHADER)
                && !std::env::var("ONEIROI_MESH_SHADERS")
                    .is_ok_and(|value| value.eq_ignore_ascii_case("off")),
            views: Vec::new(),
            bind_groups: 0,
        }
    }

    /// How many bind groups were made. While the same value is drawn from
    /// the same view, this does not grow.
    pub fn bind_groups(&self) -> usize {
        self.bind_groups
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

        // naga zeroes workgroup memory with an index of the invocation of
        // its own. A mesh shader then has two of them in SPIR-V, which is
        // invalid and makes some drivers lose the device. The shaders write
        // all of it before they read it anyway.
        let unzeroed = wgpu::PipelineCompilationOptions {
            zero_initialize_workgroup_memory: false,
            ..Default::default()
        };
        let pipeline = if mesh {
            device.create_mesh_pipeline(&wgpu::MeshPipelineDescriptor {
                label: Some(drawable.ty.name),
                layout: Some(&pipeline_layout),
                task: Some(wgpu::TaskState {
                    module: &module,
                    entry_point: Some("ts_main"),
                    compilation_options: unzeroed.clone(),
                }),
                mesh: wgpu::MeshState {
                    module: &module,
                    entry_point: Some("ms_main"),
                    compilation_options: unzeroed,
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
        let count = match kind {
            Kind::Lod(_) => Some(Self::build_count(device, drawable, &module)),
            Kind::Fixed => None,
        };
        Pipeline {
            pipeline,
            layout,
            count,
        }
    }

    fn build_count(
        device: &wgpu::Device,
        drawable: &Drawable,
        module: &wgpu::ShaderModule,
    ) -> (wgpu::ComputePipeline, wgpu::BindGroupLayout) {
        let buffer = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
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
                buffer(2, wgpu::BufferBindingType::Storage { read_only: false }),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(drawable.ty.name),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(drawable.ty.name),
            layout: Some(&pipeline_layout),
            module,
            entry_point: Some("cs_count"),
            compilation_options: Default::default(),
            cache: None,
        });
        (pipeline, layout)
    }

    /// Draws nothing into any view until something is prepared again.
    pub fn clear(&mut self) {
        for view in &mut self.views {
            view.prepared = None;
        }
    }

    /// [`DrawStage::prepare_view`] for the first view.
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
        self.prepare_view(0, device, queue, target_size, drawable, page, handle, view);
    }

    /// Call once per frame before [`DrawStage::render_view`], with the
    /// value to draw: `handle` says where its elements are inside `page`,
    /// the buffer which mirrors that page of the store. `target_size` is
    /// the size of the whole target in pixels. Views are numbered from 0.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_view(
        &mut self,
        index: usize,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_size: (u32, u32),
        drawable: &Drawable,
        page: &wgpu::Buffer,
        handle: Handle,
        view: &DrawView,
    ) {
        while self.views.len() <= index {
            self.views.push(View::new(device));
        }
        let slot = &mut self.views[index];
        slot.prepared = None;
        let texture = slot.depth.texture();
        if (texture.width(), texture.height()) != (target_size.0.max(1), target_size.1.max(1)) {
            slot.depth = depth_texture(device, target_size);
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
        let slot = &mut self.views[index];

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
                    LodMethod::MeshShader => LodShader::MESH_MAX_LEVEL,
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
                    patches: instances,
                    _padding: 0,
                };
                queue.write_buffer(&slot.lod_uniform, 0, bytemuck::bytes_of(&args));
                let draw = match method {
                    LodMethod::MeshShader => Draw::Tasks {
                        groups: instances.div_ceil(LodShader::TASK_GROUP),
                    },
                    LodMethod::VertexPulling => Draw::Vertices {
                        vertices: LodShader::vertices(max_level),
                        instances,
                    },
                };

                let counter = &mut slot.counter;
                if auto.count
                    && !counter.pending
                    && let Some((count, layout)) = &pipeline.count
                {
                    Self::count(device, queue, counter, count, layout, page, &slot.lod_uniform, instances);
                }
                (&slot.lod_uniform, draw)
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
                queue.write_buffer(&slot.uniform, 0, bytemuck::bytes_of(&args));
                let vertices = match drawable.topology {
                    Topology::Surface => rings * sides * 6,
                    Topology::Line => rings * 2,
                };
                (&slot.uniform, Draw::Vertices { vertices, instances })
            }
        };

        let bind_key = BindKey {
            pipeline: key,
            page: page.clone(),
            lod: matches!(draw, Draw::Tasks { .. }) || std::ptr::eq(uniform, &slot.lod_uniform),
        };
        let bind_group = match &slot.bound {
            Some((bound, bind_group)) if *bound == bind_key => bind_group.clone(),
            _ => {
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
                self.bind_groups += 1;
                slot.bound = Some((bind_key, bind_group.clone()));
                bind_group
            }
        };
        slot.prepared = Some(Prepared {
            key,
            bind_group,
            draw,
        });
    }

    /// Counts the triangles of what is prepared, on the device.
    #[allow(clippy::too_many_arguments)]
    fn count(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        counter: &mut Counter,
        pipeline: &wgpu::ComputePipeline,
        layout: &wgpu::BindGroupLayout,
        page: &wgpu::Buffer,
        uniform: &wgpu::Buffer,
        patches: u32,
    ) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Oneiroi Count"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: page.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: counter.buffer.as_entire_binding(),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.clear_buffer(&counter.buffer, 0, None);
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Oneiroi Count"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(patches.div_ceil(LodShader::COUNT_GROUP), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&counter.buffer, 0, &counter.readback, 0, 4);
        queue.submit([encoder.finish()]);

        let ready = counter.ready.clone();
        counter.readback.map_async(wgpu::MapMode::Read, .., move |result| {
            ready.store(result.is_ok(), Ordering::Release);
        });
        counter.pending = true;
    }

    /// How many triangles a view got the last time they were counted.
    /// They are counted when a view is prepared with [`AutoDetail::count`],
    /// and arrive some frames later without anybody waiting for them.
    pub fn triangles(&mut self, device: &wgpu::Device, index: usize) -> Option<u32> {
        self.views.get_mut(index)?.counter.poll(device)
    }

    /// [`DrawStage::render_view`] for the first view.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        bounds: Option<(f32, f32, f32, f32)>,
    ) {
        self.render_view(0, encoder, target, bounds);
    }

    /// Draws what was prepared for a view into `bounds` (x, y, width,
    /// height in pixels) of `target`, or all of it.
    pub fn render_view(
        &self,
        index: usize,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        bounds: Option<(f32, f32, f32, f32)>,
    ) {
        let Some(view) = self.views.get(index) else {
            return;
        };
        let Some(prepared) = &view.prepared else {
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
                view: &view.depth,
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
        if !restrict(&mut pass, &view.depth, bounds) {
            return;
        }

        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &prepared.bind_group, &[]);
        match prepared.draw {
            Draw::Vertices { vertices, instances } => pass.draw(0..vertices, 0..instances),
            Draw::Tasks { groups } => pass.draw_mesh_tasks(groups, 1, 1),
        }
    }
}
