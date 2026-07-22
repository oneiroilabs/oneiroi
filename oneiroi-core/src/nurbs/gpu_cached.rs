use std::num::NonZeroU32;
use wgpu::util::DeviceExt;

pub struct NurbsPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl NurbsPipeline {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        // 1. Shader Modul laden
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("NURBS Mesh Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("nurbs.wgsl").into()),
        });

        // 2. Bind Group Layout definieren (Spiegelt die @group(0) Bindings des Shaders)
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("NURBS Bind Group Layout"),
            entries: &[
                // Binding 0: RenderArgs Uniform Buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::MESH,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 1: Segments Cache Storage Buffer (Read-Only)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::MESH,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 2: Control Points Storage Buffer (Read-Only)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::MESH,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // 3. Pipeline Layout erstellen
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("NURBS Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            ..Default::default()
        });

        // 4. Mesh Pipeline konfigurieren (Nutzt create_mesh_pipeline statt create_render_pipeline)
        let pipeline = device.create_mesh_pipeline(&wgpu::MeshPipelineDescriptor {
            label: Some("NURBS Ribbon Mesh Pipeline"),
            layout: Some(&pipeline_layout),
            task: None, // Wir überspringen den Task-Shader (Objekt-Culling) und gehen direkt in den Mesh-Shader
            mesh: wgpu::MeshState {
                module: &shader,
                entry_point: Some("main"), // Name des @mesh Shaders
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"), // Name des @fragment Shaders
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // Mesh Shader gibt Dreiecke aus
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Beide Seiten rendern (Bänder können sich drehen)
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None, // Bei Bedarf hier ein Standard-DepthStencilState einfügen
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }
}
