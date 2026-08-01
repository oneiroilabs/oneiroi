use glam::{Vec3, Vec4};
/// To serve as an introduction to the wgpu api, we will implement a simple
/// compute shader which takes a list of numbers on the CPU and doubles them on the GPU.
///
/// While this isn't a very practical example, you will see all the major components
/// of using wgpu headlessly, including getting a device, running a shader, and transferring
/// data between the CPU and GPU.
///
/// If you time the recording and execution of this example you will certainly see that
/// running on the gpu is slower than doing the same calculation on the cpu. This is because
/// floating point multiplication is a very simple operation so the transfer/submission overhead
/// is quite a lot higher than the actual computation. This is normal and shows that the GPU
/// needs a lot higher work/transfer ratio to come out ahead.
use std::{num::NonZeroU64, str::FromStr};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuSample {
    position: Vec4,
    tangent: Vec4,
    normal: Vec4,
    binormal: Vec4,
}

fn main() {
    // Parse all arguments as floats. We need to skip argument 0, which is the name of the program.
    /* let arguments: Vec<f32> = std::env::args()
        .skip(1)
        .map(|s| {
            f32::from_str(&s).unwrap_or_else(|_| panic!("Cannot parse argument {s:?} as a float."))
        })
        .collect();

    if arguments.is_empty() {
        println!("No arguments provided. Please provide a list of numbers to double.");
        return;
    }

    println!("Parsed {} arguments", arguments.len()); */

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

    // wgpu uses `log` for all of our logging, so we initialize a logger with the `env_logger` crate.
    //
    // To change the log level, set the `RUST_LOG` environment variable. See the `env_logger`
    // documentation for more information.
    env_logger::init();

    // We first initialize an wgpu `Instance`, which contains any "global" state wgpu needs.
    //
    // This is what loads the vulkan/dx12/metal/opengl libraries.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

    // We then create an `Adapter` which represents a physical gpu in the system. It allows
    // us to query information about it and create a `Device` from it.
    //
    // This function is asynchronous in WebGPU, so request_adapter returns a future. On native/webgl
    // the future resolves immediately, so we can block on it without harm.
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("Failed to create adapter");

    // Print out some basic information about the adapter.
    println!("Running on Adapter: {:#?}", adapter.get_info());

    // Check to see if the adapter supports compute shaders. While WebGPU guarantees support for
    // compute shaders, wgpu supports a wider range of devices through the use of "downlevel" devices.
    let downlevel_capabilities = adapter.get_downlevel_capabilities();
    if !downlevel_capabilities
        .flags
        .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
    {
        panic!("Adapter does not support compute shaders");
    }

    // We then create a `Device` and a `Queue` from the `Adapter`.
    //
    // The `Device` is used to create and manage GPU resources.
    // The `Queue` is a queue used to submit work for the GPU to process.
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::SUBGROUP,
        required_limits: wgpu::Limits::downlevel_defaults(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
    }))
    .expect("Failed to create device");

    // Create a shader module from our shader code. This will parse and validate the shader.
    //
    // `include_wgsl` is a macro provided by wgpu like `include_str` which constructs a ShaderModuleDescriptor.
    // If you want to load shaders differently, you can construct the ShaderModuleDescriptor manually.
    //let module = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

    // Create a buffer with the data we want to process on the GPU.
    //
    // `create_buffer_init` is a utility provided by `wgpu::util::DeviceExt` which simplifies creating
    // a buffer with some initial data.
    //
    // We use the `bytemuck` crate to cast the slice of f32 to a &[u8] to be uploaded to the GPU.
    let sample_count = curve.segments.len() as u32; // Number of equidistant points requested

    println!("{sample_count}");

    // Buffer 1: Input Curve Segments
    let segments_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Curve Segments Buffer"),
        contents: bytemuck::cast_slice(&curve.segments),
        usage: wgpu::BufferUsages::STORAGE,
    });

    // Buffer 2: Dedicated GPU Output storage
    let output_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Samples Storage Buffer"),
        size: (sample_count as usize * std::mem::size_of::<GpuSample>() * 32) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Buffer 3: CPU Read-back download staging buffer
    let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Download Buffer"),
        size: output_data_buffer.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Define Bind Group Configurations matching the shader layout
    let bind_group_layout_0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
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

    let bind_group_layout_1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
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

    let bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout_0,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: segments_buffer.as_entire_binding(),
        }],
    });

    let bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout_1,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: output_data_buffer.as_entire_binding(),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&bind_group_layout_0), Some(&bind_group_layout_1)],
        immediate_size: 0,
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &device.create_shader_module(wgpu::include_wgsl!("shader.wgsl")),
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });

    // Record and Dispatch workload execution steps
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group_0, &[]);
        compute_pass.set_bind_group(1, &bind_group_1, &[]);

        let workgroup_count = 5; //sample_count.div_ceil(64);
        println!("{workgroup_count}");
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }

    encoder.copy_buffer_to_buffer(
        &output_data_buffer,
        0,
        &download_buffer,
        0,
        output_data_buffer.size(),
    );
    queue.submit([encoder.finish()]);

    // We now map the download buffer so we can read it. Mapping tells wgpu that we want to read/write
    // to the buffer directly by the CPU and it should not permit any more GPU operations on the buffer.
    //
    // Mapping requires that the GPU be finished using the buffer before it resolves, so mapping has a callback
    // to tell you when the mapping is complete.
    let buffer_slice = download_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |_| {
        // In this case we know exactly when the mapping will be finished,
        // so we don't need to do anything in the callback.
    });

    // Wait for the GPU to finish working on the submitted work. This doesn't work on WebGPU, so we would need
    // to rely on the callback to know when the buffer is mapped.
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    // We can now read the data from the buffer.
    let data = buffer_slice.get_mapped_range().unwrap();
    // Convert the data back to f32 via an aligned copy.
    let result: Vec<GpuSample> = bytemuck::allocation::pod_collect_to_vec(&data);

    //let num_samples = samples.len();
    //let mut cpu_eval = Vec::with_capacity(num_samples);
    /* for t in sample_count.into_iter() {
        let vec = curve.evaluate(t);
        cpu_eval.push(Vec4::new(vec.x, vec.y, vec.z, 0.0));
    } */

    // Print out the result.
    println!("Result: {result:?}");

    //assert_eq!(cpu_eval, result);
}
