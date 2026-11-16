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
    /// How many nodes ran on the GPU.
    pub gpu: usize,
    /// How many nodes ran on the CPU.
    pub cpu: usize,
    /// How many compute passes the nodes on the GPU took together.
    pub dispatches: usize,
    /// How many values were brought back from the GPU.
    pub downloads: usize,
}

struct Pipeline {
    kernel: &'static GpuKernel,
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    call_words: usize,
}

/// One node to run on the GPU, with what is known before it does.
struct Call<'a> {
    item: &'a Work,
    inputs: Vec<Handle>,
    elements: usize,
}

/// An [`Executor`] which runs every node that has a kernel for this device
/// as a compute shader, and all the others on the CPU.
///
/// The pages of the store are mirrored into buffers on the device, and
/// only what was written since the last time is uploaded. A kernel reads
/// its inputs straight out of those buffers and its output is copied into
/// them, at the place the store reserved for it.
///
/// Nodes which do not depend on each other run together: all calls of one
/// node are a single compute pass, as long as each of their inputs lies
/// in the same page for all of them.
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
                    Ok(shader) => Some(self.build(node, kernel, &shader)),
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

    fn build(
        &self,
        node: &NodeDef,
        kernel: &'static GpuKernel,
        shader: &shader::ComputeShader,
    ) -> Pipeline {
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
        entries.push(buffer(inputs + 1, wgpu::BufferBindingType::Storage { read_only: true }));

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
                source: wgpu::ShaderSource::Wgsl(shader.wgsl.as_str().into()),
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
            call_words: shader.call_words,
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

    /// Runs every call of one node which is in `items`. The ones whose
    /// output is too large for the device are left to the CPU.
    fn run_node(&mut self, store: &mut Store, node: &'static NodeDef, items: &[&Work]) {
        let Some(pipeline) = self.pipeline(node) else {
            for item in items {
                self.run_on_cpu(store, item);
            }
            return;
        };
        let kernel = pipeline.kernel;
        let words = node.outputs[0].ty.words as u64;
        let limits = self.device.limits();
        let max_bytes = limits.max_storage_buffer_binding_size.min(limits.max_buffer_size);
        let max_groups = limits.max_compute_workgroups_per_dimension as u64;

        let mut batch: Vec<Call<'_>> = Vec::new();
        let mut batch_bytes = 0;
        for item in items {
            // Asking for the length may look at some inputs, which then
            // have to be here.
            let reads: Vec<_> = kernel.reads.iter().map(|index| item.inputs[*index]).collect();
            self.download(store, &reads);

            let inputs: Vec<_> = item
                .inputs
                .iter()
                .map(|key| {
                    store
                        .get(*key)
                        .expect("planned inputs are stored before they are read")
                })
                .collect();
            let elements = (kernel.len)(&Inputs::new(store, node, &inputs));
            let bytes = elements as u64 * words * 4;
            if elements == 0 {
                store.insert(item.outputs[0], &[]);
                self.stats.gpu += 1;
                continue;
            }
            if bytes > max_bytes || elements.div_ceil(WORKGROUP_SIZE as usize) as u64 > max_groups {
                self.run_on_cpu(store, item);
                continue;
            }

            // Every input is bound once for the whole batch, so all calls
            // have to find theirs in the same page.
            let fits = batch.first().is_none_or(|first| {
                first
                    .inputs
                    .iter()
                    .zip(&inputs)
                    .all(|(a, b)| a.len == 0 || b.len == 0 || a.page == b.page)
            });
            if !fits || batch_bytes + bytes > max_bytes || batch.len() as u64 == max_groups {
                self.dispatch(store, node, &batch);
                batch.clear();
                batch_bytes = 0;
            }
            batch_bytes += bytes;
            batch.push(Call {
                item,
                inputs,
                elements,
            });
        }
        self.dispatch(store, node, &batch);
    }

    /// One compute pass for all of `batch`.
    fn dispatch(&mut self, store: &mut Store, node: &'static NodeDef, batch: &[Call<'_>]) {
        if batch.is_empty() {
            return;
        }
        let words = node.outputs[0].ty.words as usize;
        let inputs = node.inputs.len();

        // Make room for the results where they belong, then bring the
        // device up to date with what the CPU wrote.
        let targets: Vec<_> = batch
            .iter()
            .map(|call| store.reserve(call.item.outputs[0], (call.elements * words) as u32))
            .collect();
        self.upload(store);

        let pipeline = self.pipelines[node.id].as_ref().unwrap();
        let mut calls = Vec::with_capacity(batch.len() * pipeline.call_words);
        let mut offset = 0;
        for call in batch {
            for input in &call.inputs {
                calls.extend([input.offset, input.len]);
            }
            calls.extend([offset, call.elements as u32]);
            offset += (call.elements * words) as u32;
        }
        let calls = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Oneiroi Calls"),
                contents: bytemuck::cast_slice(&calls),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let result = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Oneiroi Output"),
            size: offset as u64 * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let mut entries: Vec<_> = (0..inputs)
            .map(|index| {
                // The page of the first call which has anything there.
                let page = batch
                    .iter()
                    .map(|call| call.inputs[index])
                    .find(|handle| handle.len > 0)
                    .map(|handle| handle.page);
                wgpu::BindGroupEntry {
                    binding: index as u32,
                    resource: match page {
                        Some(page) => self.pages[page as usize].as_entire_binding(),
                        None => self.empty.as_entire_binding(),
                    },
                }
            })
            .collect();
        entries.push(wgpu::BindGroupEntry {
            binding: inputs as u32,
            resource: result.as_entire_binding(),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: inputs as u32 + 1,
            resource: calls.as_entire_binding(),
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(node.id),
            layout: &pipeline.layout,
            entries: &entries,
        });

        let most = batch.iter().map(|call| call.elements).max().unwrap_or(0);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(node.id),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(
                most.div_ceil(WORKGROUP_SIZE as usize) as u32,
                batch.len() as u32,
                1,
            );
        }
        let mut offset = 0;
        for (call, target) in batch.iter().zip(&targets) {
            let bytes = (call.elements * words) as u64 * 4;
            let page = &self.pages[target.page as usize];
            encoder.copy_buffer_to_buffer(&result, offset, page, target.offset as u64 * 4, bytes);
            offset += bytes;
        }
        self.queue.submit([encoder.finish()]);

        self.stats.gpu += batch.len();
        self.stats.dispatches += 1;
    }

    fn run_on_cpu(&mut self, store: &mut Store, item: &Work) {
        // A node on the CPU reads all of its inputs.
        self.download(store, &item.inputs);
        self.cpu.execute(store, std::slice::from_ref(item));
        self.stats.cpu += 1;
    }
}

impl Executor for GpuExecutor {
    fn execute(&mut self, store: &mut Store, work: &[Work]) {
        // Work which reads nothing of this batch is level 0, work which
        // reads from level 0 is level 1 and so on. Whatever shares a level
        // can run in any order, and therefore together.
        let mut produced: HashMap<ContentKey, usize> = HashMap::new();
        let mut levels: Vec<Vec<&Work>> = Vec::new();
        for item in work {
            let level = item
                .inputs
                .iter()
                .filter_map(|key| produced.get(key))
                .max()
                .map_or(0, |level| level + 1);
            for output in &item.outputs {
                produced.insert(*output, level);
            }
            if levels.len() <= level {
                levels.resize_with(level + 1, Vec::new);
            }
            levels[level].push(item);
        }

        for level in levels {
            let mut nodes: Vec<(&'static NodeDef, Vec<&Work>)> = Vec::new();
            for item in level {
                match nodes.iter_mut().find(|(node, _)| std::ptr::eq(*node, item.node)) {
                    Some((_, items)) => items.push(item),
                    None => nodes.push((item.node, vec![item])),
                }
            }
            for (node, items) in nodes {
                self.run_node(store, node, &items);
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
