//! Runs nodes on the GPU.

use std::collections::HashMap;

use oneiroi_core::{
    exec::{CpuExecutor, Executor, Work},
    node::{Caps, GpuKernel, Inputs, NodeDef},
    shader::{self, ShaderError, WORKGROUP_SIZE},
    store::{ContentKey, Handle, Store},
};
use wgpu::util::DeviceExt;

/// How many nodes ran where since the executor was made.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GpuStats {
    pub gpu: usize,
    pub cpu: usize,
    /// How many values were brought back from the GPU.
    pub downloads: usize,
}

struct Pipeline {
    kernel: &'static GpuKernel,
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
}

/// An [`Executor`] which runs every node that has a kernel for this device
/// as a compute shader, and all the others on the CPU.
///
/// The pages of the store are mirrored into buffers on the device, and
/// only what was written since the last time is uploaded. A kernel reads
/// its inputs straight out of those buffers and its output is copied into
/// them, at the place the store reserved for it.
///
/// A result stays on the device. It is only brought back when a node on
/// the CPU reads it or somebody asks for it, see
/// [`Engine::fetch`](oneiroi_core::exec::Engine::fetch).
pub struct GpuExecutor {
    device: wgpu::Device,
    queue: wgpu::Queue,
    caps: Caps,
    cpu: CpuExecutor,
    /// The buffer of every page of the store, by its index.
    pages: Vec<wgpu::Buffer>,
    /// Bound for inputs which have no elements at all.
    empty: wgpu::Buffer,
    /// By the id of the node. `None` for nodes which run on the CPU.
    pipelines: HashMap<&'static str, Option<Pipeline>>,
    stats: GpuStats,
    errors: Vec<ShaderError>,
    handles: Vec<Handle>,
}

impl GpuExecutor {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let features = device.features();
        let caps = Caps {
            subgroups: features.contains(wgpu::Features::SUBGROUP),
            mesh_shaders: features.contains(wgpu::Features::EXPERIMENTAL_MESH_SHADER),
        };
        let empty = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Oneiroi Empty Input"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        Self {
            device,
            queue,
            caps,
            cpu: CpuExecutor::default(),
            pages: Vec::new(),
            empty,
            pipelines: HashMap::new(),
            stats: GpuStats::default(),
            errors: Vec::new(),
            handles: Vec::new(),
        }
    }

    /// What the device can do, which decides the kernels that are used.
    pub fn caps(&self) -> Caps {
        self.caps
    }

    pub fn stats(&self) -> GpuStats {
        self.stats
    }

    /// The kernels which did not compile. Their nodes run on the CPU.
    pub fn errors(&self) -> &[ShaderError] {
        &self.errors
    }

    fn pipeline(&mut self, node: &'static NodeDef) -> Option<&Pipeline> {
        if !self.pipelines.contains_key(node.id) {
            let pipeline = node.gpu_kernel(self.caps).and_then(|kernel| {
                match shader::compute(node, kernel) {
                    Ok(shader) => Some(self.build(node, kernel, &shader.wgsl)),
                    Err(error) => {
                        self.errors.push(error);
                        None
                    }
                }
            });
            self.pipelines.insert(node.id, pipeline);
        }
        self.pipelines[node.id].as_ref()
    }

    fn build(&self, node: &NodeDef, kernel: &'static GpuKernel, wgsl: &str) -> Pipeline {
        let buffer = |binding: usize, ty| wgpu::BindGroupLayoutEntry {
            binding: binding as u32,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let inputs = node.inputs.len();
        let mut entries: Vec<_> = (0..inputs)
            .map(|index| buffer(index, wgpu::BufferBindingType::Storage { read_only: true }))
            .collect();
        entries.push(buffer(inputs, wgpu::BufferBindingType::Storage { read_only: false }));
        entries.push(buffer(inputs + 1, wgpu::BufferBindingType::Uniform));

        let layout = self
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(node.id),
                entries: &entries,
            });
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(node.id),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(node.id),
                source: wgpu::ShaderSource::Wgsl(wgsl.into()),
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(node.id),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
        Pipeline {
            kernel,
            pipeline,
            layout,
        }
    }

    /// Brings the buffers up to date with the pages of the store.
    fn upload(&mut self, store: &mut Store) {
        let dirty = store.take_dirty();
        for (index, page) in store.pages().enumerate() {
            if index == self.pages.len() {
                self.pages.push(self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Oneiroi Page"),
                    size: page.len() as u64 * 4,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }));
            }
        }
        let pages: Vec<_> = store.pages().collect();
        for (page, range) in dirty {
            let words = &pages[page as usize][range.start as usize..range.end as usize];
            self.queue.write_buffer(
                &self.pages[page as usize],
                range.start as u64 * 4,
                bytemuck::cast_slice(words),
            );
        }
    }

    /// Runs one node as a compute shader. Returns whether it did: an
    /// output which is too large for the device is left to the CPU.
    fn run(&mut self, store: &mut Store, item: &Work) -> bool {
        self.handles.clear();
        self.handles.extend(item.inputs.iter().map(|key| {
            store
                .get(*key)
                .expect("planned inputs are stored before they are read")
        }));

        let Some(pipeline) = self.pipeline(item.node) else {
            return false;
        };
        let kernel = pipeline.kernel;
        let output = item.node.outputs[0].ty;
        // Asking for the length may look at some inputs, which then have
        // to be here.
        let reads: Vec<_> = kernel.reads.iter().map(|index| item.inputs[*index]).collect();
        self.download(store, &reads);
        let elements = (kernel.len)(&Inputs::new(store, item.node, &self.handles));
        if elements == 0 {
            store.insert(item.outputs[0], &[]);
            return true;
        }

        let limits = self.device.limits();
        let groups = elements.div_ceil(WORKGROUP_SIZE as usize) as u64;
        let size = elements as u64 * output.words as u64 * 4;
        if groups > limits.max_compute_workgroups_per_dimension as u64
            || size > limits.max_storage_buffer_binding_size
            || size > limits.max_buffer_size
        {
            return false;
        }

        let target = store.reserve(item.outputs[0], (elements * output.words as usize) as u32);
        self.upload(store);

        let mut args: Vec<[u32; 4]> = self
            .handles
            .iter()
            .map(|handle| [handle.offset, handle.len, 0, 0])
            .collect();
        args.push([elements as u32, 0, 0, 0]);
        let args = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Oneiroi Args"),
                contents: bytemuck::cast_slice(&args),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let result = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Oneiroi Output"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let inputs = self.handles.len();
        let mut entries: Vec<_> = self
            .handles
            .iter()
            .enumerate()
            .map(|(binding, handle)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: match handle.len {
                    0 => self.empty.as_entire_binding(),
                    _ => self.pages[handle.page as usize].as_entire_binding(),
                },
            })
            .collect();
        entries.push(wgpu::BindGroupEntry {
            binding: inputs as u32,
            resource: result.as_entire_binding(),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: inputs as u32 + 1,
            resource: args.as_entire_binding(),
        });

        let pipeline = self.pipelines[item.node.id].as_ref().unwrap();
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(item.node.id),
            layout: &pipeline.layout,
            entries: &entries,
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(item.node.id),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(groups as u32, 1, 1);
        }
        let page = &self.pages[target.page as usize];
        encoder.copy_buffer_to_buffer(&result, 0, page, target.offset as u64 * 4, size);
        self.queue.submit([encoder.finish()]);
        true
    }
}

impl Executor for GpuExecutor {
    fn execute(&mut self, store: &mut Store, work: &[Work]) {
        for item in work {
            if self.run(store, item) {
                self.stats.gpu += 1;
            } else {
                // A node on the CPU reads all of its inputs.
                self.download(store, &item.inputs);
                self.cpu.execute(store, std::slice::from_ref(item));
                self.stats.cpu += 1;
            }
        }
    }

    fn download(&mut self, store: &mut Store, keys: &[ContentKey]) {
        let remote: Vec<_> = keys
            .iter()
            .filter(|key| !store.is_resident(**key))
            .filter_map(|key| Some((*key, store.get(*key)?)))
            .collect();
        let size: u64 = remote.iter().map(|(_, handle)| handle.len as u64 * 4).sum();
        if size == 0 {
            return;
        }

        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Oneiroi Readback"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mut offset = 0;
        for (_, handle) in &remote {
            let bytes = handle.len as u64 * 4;
            let page = &self.pages[handle.page as usize];
            encoder.copy_buffer_to_buffer(page, handle.offset as u64 * 4, &readback, offset, bytes);
            offset += bytes;
        }
        self.queue.submit([encoder.finish()]);

        readback.map_async(wgpu::MapMode::Read, .., |result| {
            result.expect("the readback buffer is only used here")
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device is alive while its executor is");
        let bytes = readback
            .get_mapped_range(..)
            .expect("the buffer was mapped right above");
        let words: &[u32] = bytemuck::cast_slice(&bytes);

        let mut offset = 0;
        for (key, handle) in &remote {
            store.fill(*key, &words[offset..offset + handle.len as usize]);
            offset += handle.len as usize;
            self.stats.downloads += 1;
        }
    }
}
