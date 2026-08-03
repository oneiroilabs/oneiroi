use std::sync::Arc;

use glam::{Vec3, Vec4};
use oneiroi_core::nurbs::CubicNurbs;
use wgpu::{BindGroup, ComputePipeline, RenderPipeline, util::DeviceExt, wgc::resource::Buffer};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle},
    window::{Window, WindowId},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuSample {
    position: Vec3,
    tangent: Vec3,
    normal: Vec3,
    binormal: Vec3,
}

struct State {
    instance: wgpu::Instance,
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,

    compute_pipeline: ComputePipeline,
    compute_bind_group_0: BindGroup,
    compute_bind_group_1: BindGroup,

    render_bind_group_0: BindGroup,
    render_pipeline: RenderPipeline,

    indirect_buffer: wgpu::Buffer,

    curve: CubicNurbs,

    uniforms: wgpu::Buffer,
}

impl State {
    async fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> State {
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
                required_limits: wgpu::Limits::downlevel_defaults(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let control_points = vec![
            Vec4::new(0.0, 0.0, 0.0, 1.),
            Vec4::new(1.0, 2.0, 0.0, 1.),
            Vec4::new(2.0, -1.0, 0.0, 1.),
            Vec4::new(3.0, 3.0, 0.0, 1.),
            Vec4::new(4.0, 0.0, 0.0, 1.),
            Vec4::new(5.0, 2.0, 0.0, 1.),
            Vec4::new(6.0, 1.0, 0.0, 1.),
            Vec4::new(7.0, 4.0, 0.0, 1.),
        ];
        let num_points = control_points.len();
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

        let curve = oneiroi_core::nurbs::CubicNurbs::new(control_points, knot_vec);

        let num_segments = curve.segments.len() as u32;
        let total_evaluated_points = num_segments * 32; // 32 Lanes pro Segment

        // Konfiguration der Röhren-Ecken
        let radial_segments = 16u32;

        // Buffer 1: Eingabe-Kurvensegmente (Für Compute Shader)
        let segments_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Curve Segments Buffer"),
            contents: bytemuck::cast_slice(&curve.segments),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Buffer 2: Dedizierter GPU-Ausgabe- & Render-Zwischenspeicher (Passend zu `EvaluatedFrame`)
        // Wichtig: STORAGE zum Schreiben im Compute Pass, VERTEX (bzw. STORAGE-Read) für den Vertex Pass
        let evaluated_frames_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Evaluated Frames Storage Buffer"),
            size: (total_evaluated_points as usize * std::mem::size_of::<GpuSample>()) as u64, // GpuSample matcht das Layout von EvaluatedFrame
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Buffer 3: Uniform-Buffer für Render-Konfiguration (Matrix, Radius, Ecken)
        let tube_radius = 0.5f32;
        let aspect_ratio = size.width as f32 / size.height as f32;

        // Projektionsmatrix: 45 Grad Sichtfeld, Z-Near: 0.1, Z-Far: 100.0
        let projection = glam::Mat4::perspective_lh(45.0f32.to_radians(), aspect_ratio, 0.1, 100.0);

        // Viewmatrix: Kamera bei (3.5, 1.0, -8.0) platziert, blickt auf das Zentrum der Kurve (3.5, 1.0, 0.0)
        let view = glam::Mat4::look_at_lh(
            glam::Vec3::new(3.5, 1.0, -8.0), // Kameraposition
            glam::Vec3::new(3.5, 1.0, 0.0),  // Fokuspunkt (Mitte deiner Kurve)
            glam::Vec3::Y,                   // Up-Vektor
        );

        let view_projection_matrix = projection * view;

        #[repr(C)]
        #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
        struct TubeUniforms {
            view_projection: glam::Mat4,
            tube_radius: f32,
            radial_segments: u32,
            _pad0: u32,
            _pad1: u32,
        }

        let uniform_data = TubeUniforms {
            view_projection: view_projection_matrix,
            tube_radius,
            radial_segments,
            _pad0: 0,
            _pad1: 0,
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Tube Render Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniform_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Buffer 4: Draw Indirect Argumenten-Buffer
        // Berechnet die exakte Anzahl an Vertices und Instanzen für die Röhren-Quads
        #[repr(C)]
        #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
        struct DrawIndirectArgs {
            vertex_count: u32,
            instance_count: u32,
            first_vertex: u32,
            first_instance: u32,
        }

        let total_instances = total_evaluated_points - 1; // Röhren-Zwischenstücke
        let indirect_args = DrawIndirectArgs {
            vertex_count: radial_segments * 6, // 6 Vertices bilden ein Quad pro Tortenstück
            instance_count: total_instances,
            first_vertex: 0,
            first_instance: 0,
        };

        let indirect_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Tube Draw Indirect Buffer"),
            contents: bytemuck::cast_slice(&[indirect_args]),
            usage: wgpu::BufferUsages::INDIRECT,
        });

        // --- BIND GROUPS FÜR COMPUTE PIPELINE ---
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
            module: &device.create_shader_module(wgpu::include_wgsl!("shader.wgsl")), // Dein Compute Shader File
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // --- BIND GROUPS FÜR RENDER PIPELINE ---
        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Tube Render Layout"),
                entries: &[
                    // Binding 0: Frames als Read-Only Storage Buffer im Vertex Shader
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
                    // Binding 1: Uniform Config
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
                    resource: uniform_buffer.as_entire_binding(),
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
                module: &device.create_shader_module(wgpu::include_wgsl!("tube.wgsl")), // Dein Vertex Shader File
                entry_point: Some("vs_main"),
                buffers: &[], // Leer! Vertex-Fetching geschieht über vertex_index
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
                topology: wgpu::PrimitiveTopology::TriangleList, // Dreiecke für Röhren-Geometrie
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back), // Backface Culling aktivieren
                ..Default::default()
            },
            depth_stencil: None, // Bei Bedarf Depth-Buffer aktivieren
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });

        let state = State {
            instance,
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
            compute_pipeline,
            render_pipeline,
            compute_bind_group_0,
            compute_bind_group_1,
            render_bind_group_0,
            indirect_buffer,
            curve,
            uniforms: uniform_buffer,
        };

        // Configure surface for the first time
        state.configure_surface();

        state
    }

    fn get_window(&self) -> &Window {
        &self.window
    }

    fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            // Request compatibility with the sRGB-format texture view we‘re going to create later.
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size = new_size;
        self.configure_surface();

        // Kamera-Matrix mit neuem Seitenverhältnis neu berechnen
        let aspect_ratio = new_size.width as f32 / new_size.height as f32;
        let projection = glam::Mat4::perspective_lh(45.0f32.to_radians(), aspect_ratio, 0.1, 100.0);
        let view = glam::Mat4::look_at_lh(
            glam::Vec3::new(3.5, 1.0, -8.0),
            glam::Vec3::new(3.5, 1.0, 0.0),
            glam::Vec3::Y,
        );

        // Entspricht der Struktur deines Uniform-Blocks
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct TubeUniforms {
            view_projection: glam::Mat4,
            tube_radius: f32,
            radial_segments: u32,
            _pad0: u32,
            _pad1: u32,
        }

        let updated_uniforms = TubeUniforms {
            view_projection: projection * view,
            tube_radius: 0.2f32,
            radial_segments: 16,
            _pad0: 0,
            _pad1: 0,
        };

        // Neue Daten direkt an die GPU senden
        self.queue
            .write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&[updated_uniforms]));
    }

    fn render(&mut self) {
        // Create texture view.
        // NOTE: We must handle Timeout because the surface may be unavailable
        // (e.g., when the window is occluded on macOS).
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
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        // Renders a GREEN screen
        let mut encoder = self.device.create_command_encoder(&Default::default());
        // Create the renderpass which will clear the screen.
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("NURBS Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group_0, &[]);
            compute_pass.set_bind_group(1, &self.compute_bind_group_1, &[]);

            // Genau so viele Gruppen abschicken, wie Segmente vorhanden sind
            compute_pass.dispatch_workgroups(self.curve.segments.len() as u32, 1, 1);
        } // Compute-Pass endet hier. WGPU setzt automatisch eine Speicher-Barriere!

        // SCHRITT 2: Render Pass ausführen (In ein echtes Framebuffer-Ziel zeichnen)
        {
            // 'view' ist die aktuelle TextureView deines Window-Surface-Outputs
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Tube Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view, // Hier die aktuelle View der Swapchain übergeben
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
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_bind_group_0, &[]); // Der Indirect Call zeichnet die Röhre vollautomatisch anhand der GPU-Daten
            render_pass.draw_indirect(&self.indirect_buffer, 0);
        }

        // Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.queue.present(surface_texture);
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window object
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        let state = pollster::block_on(State::new(
            event_loop.owned_display_handle(),
            window.clone(),
        ));
        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                state.render();
                // Emits a new redraw requested event.
                state.get_window().request_redraw();
            }
            WindowEvent::Resized(size) => {
                // Reconfigures the size of the surface. We do not re-render
                // here as this event is always followed up by redraw request.
                state.resize(size);
            }
            _ => (),
        }
    }
}

fn main() {
    // wgpu uses `log` for all of our logging, so we initialize a logger with the `env_logger` crate.
    //
    // To change the log level, set the `RUST_LOG` environment variable. See the `env_logger`
    // documentation for more information.
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    // When the current loop iteration finishes, immediately begin a new
    // iteration regardless of whether or not new events are available to
    // process. Preferred for applications that want to render as fast as
    // possible, like games.
    event_loop.set_control_flow(ControlFlow::Poll);

    // When the current loop iteration finishes, suspend the thread until
    // another event arrives. Helps keeping CPU utilization low if nothing
    // is happening, which is preferred if the application might be idling in
    // the background.
    // event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
