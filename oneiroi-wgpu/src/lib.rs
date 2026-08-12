use std::sync::Arc;

use glam::{Mat4, Vec4};
use oneiroi_core::curve::nurbs::{CubicNurbs, CubicNurbsSegmentCache};
use wgpu::{BindGroup, Buffer, ComputePipeline, RenderPipeline, TextureFormat, util::DeviceExt};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle},
    window::{Window, WindowId},
};

use crate::orbit::OrbitCamera;

pub mod orbit;

pub const DEBUG: bool = false;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TubeUniforms {
    view_projection: glam::Mat4,
    tube_radius: f32,
    radial_segments: u32,
    _pad0: u32,
    _pad1: u32,
}

impl TubeUniforms {
    pub fn new(view_projection: Mat4, tube_radius: f32, radial_segments: u32) -> Self {
        Self {
            view_projection,
            tube_radius,
            radial_segments,
            _pad0: 0,
            _pad1: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawIndirectArgs {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RmfVisualizerUniforms {
    view_projection: glam::Mat4,
    vector_scale: f32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

impl RmfVisualizerUniforms {
    pub fn new(view_projection: Mat4, vector_scale: f32) -> Self {
        Self {
            view_projection,
            vector_scale,

            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        }
    }
}

pub struct State {
    instance: wgpu::Instance,
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,

    pub camera: OrbitCamera,
    curve: CubicNurbs,

    pipeline_state: PipelineState,
}

pub struct PipelineState {
    compute_pipeline: ComputePipeline,
    compute_bind_group_0: BindGroup,
    compute_bind_group_1: BindGroup,
    segments_buffer: Buffer,
    evaluated_frames_buffer: Buffer,

    render_bind_group_0: BindGroup,
    render_pipeline: RenderPipeline,

    indirect_buffer: wgpu::Buffer,

    tube_uniforms: wgpu::Buffer,

    debug_vis: RenderPipeline,
    debug_bind_group_0: BindGroup,
    visualizer_uniform_buffer: wgpu::Buffer,

    depth_texture_view: wgpu::TextureView,
    //mesh_pipeline: RenderPipeline,
    //mesh_bind_group_0: BindGroup,
}

impl PipelineState {
    pub fn new(device: &wgpu::Device, surface_format: TextureFormat) -> Self {
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: 1,  //size.width,
                height: 1, // size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let segments_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Curve Segments Buffer"),
            size: 16,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let evaluated_frames_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Evaluated Frames Storage Buffer"),
            size: 64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        /* let uniform_data = TubeUniforms {
            view_projection: view_projection_matrix,
            tube_radius,
            radial_segments,
            _pad0: 0,
            _pad1: 0,
        }; */

        let tube_uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Tube Render Uniform Buffer"),
            size: std::mem::size_of::<TubeUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        /* let indirect_args = DrawIndirectArgs {
            vertex_count: if DEBUG { 6 } else { radial_segments * 6 },
            instance_count: total_evaluated_points - 1,
            first_vertex: 0,
            first_instance: 0,
        }; */

        let indirect_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Tube Draw Indirect Buffer"),
            size: std::mem::size_of::<DrawIndirectArgs>() as u64,
            usage: wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });

        let compute_bind_group_layout_0 =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute Input Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        min_binding_size: None,
                        has_dynamic_offset: false,
                    },
                    count: None,
                }],
            });

        let compute_bind_group_layout_1 =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute Output Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        min_binding_size: None,
                        has_dynamic_offset: false,
                    },
                    count: None,
                }],
            });

        let compute_bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Input Bind Group"),
            layout: &compute_bind_group_layout_0,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: segments_buffer.as_entire_binding(),
            }],
        });

        let compute_bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Output Bind Group"),
            layout: &compute_bind_group_layout_1,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: evaluated_frames_buffer.as_entire_binding(),
            }],
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Layout"),
                bind_group_layouts: &[
                    Some(&compute_bind_group_layout_0),
                    Some(&compute_bind_group_layout_1),
                ],
                immediate_size: 0,
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("NURBS Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &device.create_shader_module(wgpu::include_wgsl!("shader.wgsl")),
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Tube Render Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            min_binding_size: None,
                            has_dynamic_offset: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            min_binding_size: None,
                            has_dynamic_offset: false,
                        },
                        count: None,
                    },
                ],
            });

        let render_bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Tube Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: evaluated_frames_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: tube_uniforms.as_entire_binding(),
                },
            ],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Layout"),
                bind_group_layouts: &[Some(&render_bind_group_layout)],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3D Tube Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &device.create_shader_module(wgpu::include_wgsl!("tube.wgsl")),
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &device.create_shader_module(wgpu::include_wgsl!("tube.wgsl")),
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format.add_srgb_suffix(),
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
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
            cache: None,
            multiview_mask: None,
        });

        /* let visualizer_uniform_data = RmfVisualizerUniforms {
            view_projection: view_projection_matrix,
            vector_scale: 0.25,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        }; */

        let visualizer_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RMF Visualizer Uniform Buffer"),
            size: std::mem::size_of::<RmfVisualizerUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let visualizer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("RMF Visualizer Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let visualizer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RMF Visualizer Bind Group"),
            layout: &visualizer_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: visualizer_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: evaluated_frames_buffer.as_entire_binding(),
                },
            ],
        });

        let visualizer_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("RMF Visualizer Pipeline Layout"),
                bind_group_layouts: &[Some(&visualizer_bind_group_layout)],
                immediate_size: 0,
            });

        let visualizer_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("RMF Visualizer Pipeline"),
            layout: Some(&visualizer_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &device.create_shader_module(wgpu::include_wgsl!("rmf_vis.wgsl")),
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &device.create_shader_module(wgpu::include_wgsl!("rmf_vis.wgsl")),
                entry_point: Some("fr_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format.add_srgb_suffix(),
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
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

        /* let mesh_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Tube Pure Mesh Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::MESH | wgpu::ShaderStages::TASK,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            min_binding_size: None,
                            has_dynamic_offset: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::MESH | wgpu::ShaderStages::TASK,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            min_binding_size: None,
                            has_dynamic_offset: false,
                        },
                        count: None,
                    },
                ],
            });

        let mesh_bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mesh Render Bind Group"),
            layout: &mesh_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: segments_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let mesh_shader_module =
            device.create_shader_module(wgpu::include_wgsl!("tube_task_mesh.wgsl"));

        let mesh_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mesh Render Layout"),
            bind_group_layouts: &[Some(&mesh_bind_group_layout)],
            immediate_size: 0,
        });

        let mesh_pipeline = device.create_mesh_pipeline(&wgpu::MeshPipelineDescriptor {
            label: Some("Mesh Shading Tube Pipeline"),
            layout: Some(&mesh_pipeline_layout),
            task: Some(TaskState {
                module: &mesh_shader_module,
                entry_point: Some("ts_main"),
                compilation_options: Default::default(),
            }),
            mesh: MeshState {
                module: &mesh_shader_module,
                entry_point: Some("ms_main"),
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &mesh_shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format.add_srgb_suffix(),
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            cache: None,
            multiview: None,
        }); */

        Self {
            compute_pipeline,
            render_pipeline,
            compute_bind_group_0,
            compute_bind_group_1,
            render_bind_group_0,
            indirect_buffer,
            tube_uniforms,
            debug_vis: visualizer_pipeline,
            debug_bind_group_0: visualizer_bind_group,
            visualizer_uniform_buffer,
            depth_texture_view,
            segments_buffer,
            evaluated_frames_buffer,
            // mesh_pipeline,
            //mesh_bind_group_0,
        }
    }

    fn update_depth_texture(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        if self.depth_texture_view.texture().height() != size.1
            || self.depth_texture_view.texture().width() != size.0
        {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("cubes depth texture"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            self.depth_texture_view = tex.create_view(&wgpu::TextureViewDescriptor::default());

            //self.depth_pipeline.update(device, &tex);
        }
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_size: (u32, u32),
        tube_uniforms: &TubeUniforms,
        vis_uniforms: &RmfVisualizerUniforms,
        segments: &[CubicNurbsSegmentCache],
    ) {
        self.update_depth_texture(device, target_size);

        queue.write_buffer(&self.tube_uniforms, 0, bytemuck::bytes_of(tube_uniforms));

        queue.write_buffer(&self.segments_buffer, 0, bytemuck::cast_slice(segments));
    }

    pub fn render(
        &self,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        //clip_bounds: Rectangle<u32>,
        num_segments: u32,
    ) {
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("NURBS Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group_0, &[]);
            compute_pass.set_bind_group(1, &self.compute_bind_group_1, &[]);

            compute_pass.dispatch_workgroups(num_segments, 1, 1);
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Tube Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.01,
                            g: 0.01,
                            b: 0.02,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
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

            if DEBUG {
                render_pass.set_pipeline(&self.debug_vis);
                render_pass.set_bind_group(0, &self.debug_bind_group_0, &[]);
                render_pass.draw_indirect(&self.indirect_buffer, 0);
            } else {
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.render_bind_group_0, &[]);
                render_pass.draw_indirect(&self.indirect_buffer, 0);
            }

            // render_pass.set_pipeline(&self.mesh_pipeline);
            // render_pass.set_bind_group(0, &self.mesh_bind_group_0, &[]);
            // render_pass.draw_mesh_tasks(self.curve.segments.len() as u32, 1, 1);
        }
    }
}

impl State {
    pub async fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> State {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(display),
        ));
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::SUBGROUP,
                //| wgpu::Features::EXPERIMENTAL_MESH_SHADER,
                required_limits: adapter.limits(),
                experimental_features: unsafe { wgpu::ExperimentalFeatures::enabled() },
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Directory(std::path::PathBuf::from(
                    std::env!("CARGO_MANIFEST_DIR").to_string() + "/trace",
                )),
            })
            .await
            .unwrap();

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        /* let control_points = vec![
            Vec4::new(0.0, 4.0, 0.0, 1.),
            Vec4::new(1.0, 2.0, 0.0, 1.),
            Vec4::new(2.0, -1.0, 0.0, 1.),
            Vec4::new(3.0, 3.0, 0.0, 1.),
            Vec4::new(4.0, 0.0, 0.0, 1.),
            Vec4::new(5.0, 2.0, 0.0, 1.),
            Vec4::new(6.0, 1.0, 0.0, 1.),
            Vec4::new(7.0, 4.0, 0.0, 1.),
        ]; */

        let num_points = 15;
        let mut control_points = Vec::with_capacity(num_points);

        let step_distance = 0.25;

        // Scale factor for how quickly the spiral expands outwards (the 'a' coefficient)
        let expansion_rate = 0.15;

        for i in 0..num_points {
            let step = i as f64;

            let target_arc_length = step * step_distance;

            let theta = if target_arc_length > 0.0 {
                (2.0 * target_arc_length / expansion_rate).sqrt()
            } else {
                0.0
            };

            let radius = expansion_rate * theta;

            let x = radius * theta.cos();
            let y = radius * theta.sin();

            control_points.push(Vec4::new(x as f32, y as f32, 0.0, 1.0));
        }

        /* for i in 0..num_points {
            control_points.push(Vec4::new((i as i32 - 100) as f32, 0.0, 0.0, 1.0));
        } */

        let num_knots = num_points + 4;

        let mut knot_vec = vec![0.0; num_knots];
        for i in num_points..num_knots {
            knot_vec[i] = 1.0;
        }
        let num_interior_segments = num_points - 3;
        for i in 4..num_points {
            let interior_t = (i - 3) as f32 / num_interior_segments as f32;
            knot_vec[i] = interior_t;
        }

        let curve = oneiroi_core::curve::nurbs::CubicNurbs::new(control_points, knot_vec);

        let num_segments = curve.segments().len() as u32;
        let total_evaluated_points = num_segments * 32;

        let radial_segments = 16u32;

        let tube_radius = 0.5f32;
        let aspect_ratio = size.width as f32 / size.height as f32;

        let camera = OrbitCamera::new(glam::Vec3::new(0., 0., 0.0), 10.0);

        let view = camera.build_view_matrix();
        let projection =
            glam::Mat4::perspective_infinite_reverse_lh(45.0f32.to_radians(), aspect_ratio, 0.1);
        let view_projection_matrix = projection * view;

        let state = State {
            pipeline_state: PipelineState::new(&device, surface_format),
            instance,
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
            curve,
            camera,
        };

        state.configure_surface();

        state
    }

    pub fn update_camera_buffers(&self) {
        let aspect_ratio = self.size.width as f32 / self.size.height as f32;
        let projection =
            glam::Mat4::perspective_infinite_reverse_lh(45.0f32.to_radians(), aspect_ratio, 0.1);
        let view = self.camera.build_view_matrix();
        let view_projection = projection * view;

        // 1. Tube Uniforms aktualisieren
        let tube_uniforms = TubeUniforms {
            view_projection,
            tube_radius: 0.2f32,
            radial_segments: 16,
            _pad0: 0,
            _pad1: 0,
        };
        self.queue.write_buffer(
            &self.pipeline_state.tube_uniforms,
            0,
            bytemuck::cast_slice(&[tube_uniforms]),
        );

        // 2. RMF Visualizer Uniforms aktualisieren
        let vis_uniforms = RmfVisualizerUniforms {
            view_projection,
            vector_scale: 0.25,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        };
        self.queue.write_buffer(
            &self.pipeline_state.visualizer_uniform_buffer,
            0,
            bytemuck::bytes_of(&vis_uniforms),
        );
    }

    pub fn get_window(&self) -> &Window {
        &self.window
    }

    pub fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size = new_size;
        self.configure_surface();
        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: new_size.width,
                height: new_size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        self.pipeline_state.depth_texture_view =
            depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.update_camera_buffers();
    }

    pub fn render(&mut self) {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => return,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.configure_surface();
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.configure_surface();
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                unreachable!("No error scope registered, so validation errors will panic")
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self.instance.create_surface(self.window.clone()).unwrap();
                self.configure_surface();
                return;
            }
        };
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        let mut encoder = self.device.create_command_encoder(&Default::default());

        self.pipeline_state.render(
            &texture_view,
            &mut encoder,
            //clip_bounds,
            self.curve.segments().len() as u32,
        );

        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.queue.present(surface_texture);
    }
}
