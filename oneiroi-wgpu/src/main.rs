use std::{borrow::Cow, sync::Arc};

use oneiroi_wgpu::{dispatch_graph, setup_work_graph};
use renderdoc::{InputButton, RenderDoc, V110, V141};
use wgpu::{
    Backends, PassthroughShaderEntryPoint, SurfaceConfiguration, wgc::api::Dx12,
    wgt::CreateShaderModuleDescriptorPassthrough,
};
use windows::{
    Win32::Graphics::Direct3D12::{
        self, ID3D12CommandList, ID3D12GraphicsCommandList10, ID3D12Resource, ID3D12StateObject,
    },
    core::Interface,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle},
    keyboard::{KeyCode, NativeKeyCode, PhysicalKey},
    window::{Window, WindowId},
};

struct State {
    instance: wgpu::Instance,
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    // pipeline: Option<wgpu::RenderPipeline>,
    state_object: ID3D12StateObject,
    backing_mem: ID3D12Resource,
}

impl State {
    async fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> State {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(display),
        ));

        let required_features = /* wgpu::Features::EXPERIMENTAL_MESH_SHADER
            | */ wgpu::Features::EXPERIMENTAL_WORK_GRAPHS
            | wgpu::Features::PASSTHROUGH_SHADERS;

        let adapters = instance.enumerate_adapters(Backends::all()).await;

        let mut chosen_adapter = None;
        for adapter in adapters {
            /* if let Some(surface) = surface {
                if !adapter.is_surface_supported(surface) {
                    continue;
                }
            } */
            let adapter_features = adapter.features();
            println!("{:?}", adapter.get_info());
            println!("{adapter_features}");
            if !adapter_features.contains(required_features) {
                continue;
            } else {
                chosen_adapter = Some(adapter);
                break;
            }
        }

        let adapter = chosen_adapter.expect("No suitable GPU adapters found on the system!");
        /* let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap(); */

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features,
                required_limits: wgpu::Limits::defaults()
                    .using_recommended_minimum_mesh_shader_values(),
                experimental_features: unsafe { wgpu::ExperimentalFeatures::enabled() },
                ..Default::default()
            })
            .await
            .unwrap();

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        // Configure surface for the first time

        /* let shader = state
            .device
            .create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let pipeline_layout =
            state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[],
                    immediate_size: 0,
                });
        let pipeline = state
            .device
            .create_mesh_pipeline(&wgpu::MeshPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                task: Some(wgpu::TaskState {
                    module: &shader,
                    entry_point: Some("ts_main"),
                    compilation_options: Default::default(),
                }),
                mesh: wgpu::MeshState {
                    module: &shader,
                    entry_point: Some("ms_main"),
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(
                        state.surface.get_configuration().unwrap().view_formats[0].into(),
                    )],
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            }); */

        /* state.pipeline = Some(pipeline); */

        let wg_shader = unsafe {
            device.create_shader_module_passthrough(CreateShaderModuleDescriptorPassthrough {
                label: Some("work-graph-shader"),
                hlsl: Some(Cow::Borrowed("shader.hlsl")),
                ..Default::default()
            })
        };

        let what = unsafe { device.as_hal::<wgpu::hal::api::Dx12>() };

        let temp_device = what
            .unwrap()
            .raw_device()
            .cast::<Direct3D12::ID3D12Device14>()
            .unwrap();

        /* let what =
        unsafe { device.CreateStateObject::<ID3D12StateObject>(&state_object) }.unwrap(); */

        let (state_object, backing_mem) = unsafe { setup_work_graph(&temp_device) }.unwrap();

        let mut state = State {
            instance,
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
            state_object,
            backing_mem,
            //pipeline: None,
        };
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
        self.size = new_size;

        // reconfigure the surface
        self.configure_surface();
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
        /*  let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        rpass.push_debug_group("Prepare data for draw.");
        rpass.set_pipeline(self.pipeline.as_ref().unwrap());
        rpass.pop_debug_group();
        rpass.insert_debug_marker("Draw!");
        rpass.draw_mesh_tasks(1, 1, 1);

        // If you wanted to call any drawing commands, they would go here.

        // End the renderpass.
        drop(rpass);

        // Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.queue.present(surface_texture); */

        unsafe {
            encoder.as_hal_mut::<Dx12, _, _>(|h| unsafe {
                let cmd_list_10 = h.unwrap().raw_list();

                dispatch_graph(cmd_list_10, &self.state_object, &self.backing_mem)
            });
        }
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
    //let mut renderdoc: RenderDoc<V110> = RenderDoc::new().unwrap();
    //renderdoc.start_frame_capture(std::ptr::null(), std::ptr::null());
    event_loop.run_app(&mut app).unwrap();
    //renderdoc.end_frame_capture(std::ptr::null(), std::ptr::null());
}
