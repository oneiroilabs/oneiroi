/* use glam::Mat4;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct NurbsGpuSegmentCache {
    pub matrix: Mat4,
    pub t_start: f32,
    pub t_end: f32,
    pub starting_point_idx: u32,
    pub _pad0: u32,
}

pub fn create_nurbs_render_resources(
    device: &wgpu::Device,
) -> (wgpu::BindGroupLayout, wgpu::PipelineLayout) {
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("NURBS Dual-Array Bind Group Layout"),
        entries: &[
            // @binding(0): global_points array
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::MESH,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // @binding(1): segments metadata array
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
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("NURBS Pipeline Layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        ..Default::default()
    });

    (bind_group_layout, pipeline_layout)
}

pub struct GpuNurbsCurve {
    /// Holds the global pool of homogeneous control points (w*x, w*y, w*z, w)
    pub points_buffer: wgpu::Buffer,
    /// Holds the 48-byte minimal segment metadata blocks (matrix + t_bounds + index)
    pub segments_buffer: wgpu::Buffer,
    /// Pre-configured bind group mapping these buffers directly to the shader
    pub bind_group: wgpu::BindGroup,
    /// Total count of control points currently stored
    pub point_count: usize,
}

use wgpu::util::DeviceExt;

impl GpuNurbsCurve {
    pub fn init_from_cpu(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        nurbs: &super::CubicNurbs,
    ) -> Self {
        let n = nurbs.points.len();
        let num_segments = n - 3;
        let knots = &nurbs.knots_followed_by_weights;
        let weights = &nurbs.knots_followed_by_weights[n + 4..];

        // 1. Package flat global homogeneous positions block
        let mut global_points = vec![glam::Vec4::ZERO; n];
        for i in 0..n {
            let w = weights[i];
            global_points[i] = glam::Vec4::new(
                nurbs.points[i].x * w,
                nurbs.points[i].y * w,
                nurbs.points[i].z * w,
                w,
            );
        }

        // 2. Package flat minimal segment metadata blocks
        let mut minimal_segments = Vec::with_capacity(num_segments);
        for idx in 0..num_segments {
            let r = idx + 3;
            minimal_segments.push(NurbsGpuSegmentCache {
                matrix: nurbs.mardsen_cache[idx],
                t_start: knots[r],
                t_end: knots[r + 1],
                starting_point_idx: (r - 3) as u32,
                _pad0: 0,
            });
        }

        // 3. Allocate actual hardware GPU buffers with exact sizes
        let points_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("NURBS Global Points Buffer"),
            contents: bytemuck::cast_slice(&global_points),
            // COPY_DST lets us write updates directly to individual indexes later
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let segments_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("NURBS Compressed Segments Buffer"),
            contents: bytemuck::cast_slice(&minimal_segments),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // 4. Map buffers directly onto the BindGroup descriptors
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("NURBS Render Instance Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: points_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: segments_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            points_buffer,
            segments_buffer,
            bind_group,
            point_count: n,
        }
    }

    pub fn update_point_at_index(
        &self,
        queue: &wgpu::Queue,
        index: usize,
        new_pos: glam::Vec3,
        weight: f32,
    ) {
        assert!(
            index < self.point_count,
            "Control point index out of bounds"
        );
        assert!(
            weight > 0.0,
            "NURBS weight parameters must be strictly positive"
        );

        // Convert the modified data to its homogeneous form [w*x, w*y, w*z, w]
        let updated_homogeneous_pt = glam::Vec4::new(
            new_pos.x * weight,
            new_pos.y * weight,
            new_pos.z * weight,
            weight,
        );

        // Calculate the precise byte offset where this specific index sits in memory
        // Each Vec4 takes up exactly 16 bytes
        let byte_offset = (index * 16) as wgpu::BufferAddress;

        // Stream the exact 16 bytes straight to the GPU
        queue.write_buffer(
            &self.points_buffer,
            byte_offset,
            bytemuck::bytes_of(&updated_homogeneous_pt),
        );
    }
}
 */
