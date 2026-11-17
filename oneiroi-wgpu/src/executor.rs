//! Runs nodes on the GPU.

use std::{collections::HashMap, num::NonZeroU32};

use oneiroi_core::{
    exec::{CpuExecutor, Executor, Work},
    node::{Caps, GpuKernel, Inputs, NodeDef},
    shader::{self, CALL_WINDOW, GATHER, GATHER_CALL_WORDS, ShaderError, WORKGROUP_SIZE},
    store::{ContentKey, Handle, Store},
};

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
    /// How many buffers the executor made. Once every page of the store
    /// has one, this does not grow anymore.
    pub buffers: usize,
    /// How many times work was handed to the device.
    pub submits: usize,
}

/// The features a device needs for an executor to run anything on it.
/// Without them everything runs on the CPU.
pub const FEATURES: wgpu::Features = wgpu::Features::BUFFER_BINDING_ARRAY
    .union(wgpu::Features::STORAGE_RESOURCE_BINDING_ARRAY)
    .union(wgpu::Features::PARTIALLY_BOUND_BINDING_ARRAY)
    .union(wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING);

/// How many pages of the store a device has to be able to bind at once.
/// A store with more pages runs what touches the others on the CPU.
pub const MAX_PAGES: u32 = 1024;

/// How many words of calls the ring holds before work is handed over.
const RING_WORDS: u32 = 1 << 18;

struct Pipeline {
    kernel: &'static GpuKernel,
    /// One per output, offsets included.
    pipelines: Vec<wgpu::ComputePipeline>,
    call_words: usize,
}

/// One node to run on the GPU, with what is known before it does.
struct Call<'a> {
    item: &'a Work,
    /// Its inputs and the offsets of the grouped ones.
    inputs: Vec<Handle>,
    /// How many elements each of its outputs gets.
    elements: Vec<usize>,
}

/// What is needed to run anything at all, if the device can.
struct Bindless {
    layout: wgpu::BindGroupLayout,
    /// The calls, through the ring.
    calls: wgpu::BindGroup,
    pipeline_layout: wgpu::PipelineLayout,
    gather: wgpu::ComputePipeline,
    /// How many pages one bind group holds.
    max_pages: u32,
    /// Every page, made again when a page is added.
    bind_group: Option<wgpu::BindGroup>,
}

/// An [`Executor`] which runs every node that has a kernel for this device
/// as a compute shader, and all the others on the CPU.
///
/// The pages of the store are mirrored into buffers on the device, and
/// only what was written since the last time is uploaded. All of them are
/// bound at once, so a kernel reads its inputs from wherever they are and
/// writes its output right where the store reserved it. Nodes which do not
/// depend on each other run together: all calls of one node are a single
/// compute pass.
///
/// Nothing is made per call: calls go through a ring buffer, results are
/// read back through one buffer which only grows, and all work of one run
/// is handed to the device at once.
///
/// A result stays on the device. It is only brought back when a node on
/// the CPU reads it or somebody asks for it, see
/// [`Engine::fetch`](oneiroi_core::exec::Engine::fetch).
pub struct GpuExecutor {
    device: wgpu::Device,
    queue: wgpu::Queue,
    caps: Caps,
    cpu: CpuExecutor,
    /// `None` when the device lacks [`FEATURES`].
    bindless: Option<Bindless>,
    /// The buffer of every page of the store, by its index.
    pages: Vec<wgpu::Buffer>,
    /// The calls of the dispatches which were not handed over yet.
    ring: wgpu::Buffer,
    ring_cursor: u32,
    /// In words, what a dynamic offset into the ring has to be a multiple of.
    ring_alignment: u32,
    readback: Option<wgpu::Buffer>,
    /// Work which was recorded and not handed over yet.
    encoder: Option<wgpu::CommandEncoder>,
    /// By the id of the node. `None` for nodes which run on the CPU.
    pipelines: HashMap<&'static str, Option<Pipeline>>,
    stats: GpuStats,
    errors: Vec<ShaderError>,
}

impl GpuExecutor {
    /// Whether a device has what an executor needs to run anything on it.
    pub fn supports(device: &wgpu::Device) -> bool {
        device.features().contains(FEATURES)
            && device.limits().max_binding_array_elements_per_shader_stage > 0
    }

    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let features = device.features();
        let caps = Caps {
            subgroups: features.contains(wgpu::Features::SUBGROUP),
            mesh_shaders: features.contains(wgpu::Features::EXPERIMENTAL_MESH_SHADER),
        };
        let ring = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Oneiroi Calls"),
            size: RING_WORDS as u64 * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bindless = Self::supports(&device).then(|| Self::bindless(&device, &ring));
        Self {
            ring_alignment: (device.limits().min_storage_buffer_offset_alignment / 4).max(1),
            device,
            queue,
            caps,
            cpu: CpuExecutor::default(),
            bindless,
            pages: Vec::new(),
            ring,
            ring_cursor: 0,
            readback: None,
            encoder: None,
            pipelines: HashMap::new(),
            stats: GpuStats {
                buffers: 1,
                ..GpuStats::default()
            },
            errors: Vec::new(),
        }
    }

    fn bindless(device: &wgpu::Device, ring: &wgpu::Buffer) -> Bindless {
        let max_pages = device
            .limits()
            .max_binding_array_elements_per_shader_stage
            .min(MAX_PAGES);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Oneiroi Pages"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: NonZeroU32::new(max_pages),
                },
            ],
        });
        let calls_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Oneiroi Calls"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let calls = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Oneiroi Calls"),
            layout: &calls_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: ring,
                    offset: 0,
                    size: wgpu::BufferSize::new(CALL_WINDOW as u64 * 4),
                }),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Oneiroi Pages"),
            bind_group_layouts: &[Some(&layout), Some(&calls_layout)],
            immediate_size: 0,
        });
        let gather =
            Self::compute_pipeline(device, &pipeline_layout, "Oneiroi Gather", GATHER, "main");
        Bindless {
            layout,
            calls,
            pipeline_layout,
            gather,
            max_pages,
            bind_group: None,
        }
    }

    fn compute_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        label: &str,
        wgsl: &str,
        entry: &str,
    ) -> wgpu::ComputePipeline {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(wgsl.into()),
        });
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(label),
            layout: Some(layout),
            module: &module,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            cache: None,
        })
    }

    /// What the device can do, which decides the kernels that are used.
    pub fn caps(&self) -> Caps {
        self.caps
    }

    /// Whether anything runs on the GPU at all, see [`FEATURES`].
    pub fn is_supported(&self) -> bool {
        self.bindless.is_some()
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
            let bindless = self.bindless.as_ref();
            let pipeline = bindless
                .and_then(|bindless| Some((bindless, node.gpu_kernel(self.caps)?)))
                .and_then(|(bindless, kernel)| match shader::compute(node, kernel) {
                    Ok(shader) => Some(Pipeline {
                        kernel,
                        pipelines: (0..kernel.outputs.len())
                            .map(|index| {
                                Self::compute_pipeline(
                                    &self.device,
                                    &bindless.pipeline_layout,
                                    node.id,
                                    &shader.wgsl,
                                    &format!("main_{index}"),
                                )
                            })
                            .collect(),
                        call_words: shader.call_words,
                    }),
                    Err(error) => {
                        self.errors.push(error);
                        None
                    }
                });
            self.pipelines.insert(node.id, pipeline);
        }
        self.pipelines[node.id].as_ref()
    }

    /// The buffer which mirrors a page of the store. Whoever draws a value
    /// binds the page its handle names. Call [`GpuExecutor::sync`] first.
    pub fn page(&self, index: u32) -> Option<&wgpu::Buffer> {
        self.pages.get(index as usize)
    }

    /// Brings the buffers up to date with what the CPU wrote into the
    /// store, and hands over all work which was recorded. Kernels do this
    /// themselves, anything else which reads the buffers has to ask for it.
    pub fn sync(&mut self, store: &mut Store) {
        self.upload(store);
        self.submit();
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
                self.stats.buffers += 1;
                if let Some(bindless) = &mut self.bindless {
                    bindless.bind_group = None;
                }
            }
        }
        let pages: Vec<_> = store.pages().collect();
        for (page, range) in dirty {
            let words = &pages[page as usize][range.start as usize..range.end as usize];
            // The write happens before anything which is handed over after
            // it, including what is recorded already. That is fine: it goes
            // to memory nothing recorded touches, since nothing in the
            // store is ever written twice.
            self.queue.write_buffer(
                &self.pages[page as usize],
                range.start as u64 * 4,
                bytemuck::cast_slice(words),
            );
        }
    }

    /// Whether a value is somewhere every kernel can reach it.
    fn reachable(&self, handle: &Handle) -> bool {
        let Some(bindless) = &self.bindless else {
            return false;
        };
        let max_bytes = self.device.limits().max_storage_buffer_binding_size;
        handle.page < bindless.max_pages
            && self
                .pages
                .get(handle.page as usize)
                .is_none_or(|page| page.size() <= max_bytes)
    }

    fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
        self.encoder.get_or_insert_with(|| {
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Oneiroi"),
                })
        })
    }

    /// Hands over all work which was recorded.
    fn submit(&mut self) {
        if let Some(encoder) = self.encoder.take() {
            self.queue.submit([encoder.finish()]);
            self.stats.submits += 1;
        }
        self.ring_cursor = 0;
    }

    /// Puts calls into the ring and returns the dynamic offset in bytes
    /// they are found at.
    fn push_calls(&mut self, calls: &[u32]) -> u32 {
        debug_assert!(calls.len() <= CALL_WINDOW as usize);
        let align = self.ring_alignment;
        let mut offset = self.ring_cursor.div_ceil(align) * align;
        if offset + CALL_WINDOW > RING_WORDS {
            // Full: what was recorded goes first, then the ring is free.
            self.submit();
            offset = 0;
        }
        self.queue
            .write_buffer(&self.ring, offset as u64 * 4, bytemuck::cast_slice(calls));
        self.ring_cursor = offset + calls.len() as u32;
        offset * 4
    }

    /// The bind group of every page.
    fn bind_group(&mut self) -> wgpu::BindGroup {
        let bindless = self.bindless.as_mut().expect("only used where it exists");
        if let Some(bind_group) = &bindless.bind_group {
            return bind_group.clone();
        }
        let pages: Vec<_> = self
            .pages
            .iter()
            .take(bindless.max_pages as usize)
            .map(|page| page.as_entire_buffer_binding())
            .collect();
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Oneiroi Pages"),
            layout: &bindless.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::BufferArray(&pages),
            }],
        });
        bindless.bind_group = Some(bind_group.clone());
        bind_group
    }

    /// Records one compute pass which runs `pipeline` for every call.
    fn record(&mut self, label: &str, pipeline: &wgpu::ComputePipeline, calls: &[u32], groups: (u32, u32)) {
        let offset = self.push_calls(calls);
        let bind_group = self.bind_group();
        let calls = self.bindless.as_ref().unwrap().calls.clone();
        let encoder = self.encoder();
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some(label),
            timestamp_writes: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_bind_group(1, &calls, &[offset]);
        pass.dispatch_workgroups(groups.0, groups.1, 1);
        drop(pass);
        self.stats.dispatches += 1;
    }

    /// Runs every call of one node which is in `items`. The ones the device
    /// can not take are left to the CPU.
    fn run_node(&mut self, store: &mut Store, node: &'static NodeDef, items: &[&Work]) {
        let Some(pipeline) = self.pipeline(node) else {
            for item in items {
                self.run_on_cpu(store, item);
            }
            return;
        };
        let (kernel, call_words) = (pipeline.kernel, pipeline.call_words);
        let words: Vec<u64> = node
            .outputs
            .iter()
            .map(|port| port.ty.words as u64)
            .chain(node.hidden_outputs().map(|_| 1))
            .collect();
        let limits = self.device.limits();
        let max_bytes = limits.max_storage_buffer_binding_size.min(limits.max_buffer_size);
        let max_groups = limits.max_compute_workgroups_per_dimension as usize;
        let max_rows = (CALL_WINDOW as usize / call_words).min(max_groups);

        let mut batch: Vec<Call<'_>> = Vec::new();
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
            let view = Inputs::new(store, node, &inputs);
            if kernel.accepts.is_some_and(|accepts| !accepts(&view)) {
                self.run_on_cpu(store, item);
                continue;
            }
            let elements: Vec<usize> = kernel.outputs.iter().map(|output| (output.len)(&view)).collect();
            let fits = elements.iter().zip(&words).all(|(elements, words)| {
                *elements as u64 * words * 4 <= max_bytes
                    && elements.div_ceil(WORKGROUP_SIZE as usize) <= max_groups
            }) && inputs.iter().all(|handle| self.reachable(handle));
            if !fits {
                self.run_on_cpu(store, item);
                continue;
            }
            if batch.len() == max_rows {
                self.dispatch(store, node, &batch);
                batch.clear();
            }
            batch.push(Call {
                item,
                inputs,
                elements,
            });
        }
        self.dispatch(store, node, &batch);
    }

    /// A compute pass per output for all of `batch`.
    fn dispatch(&mut self, store: &mut Store, node: &'static NodeDef, batch: &[Call<'_>]) {
        if batch.is_empty() {
            return;
        }
        let words: Vec<usize> = node
            .outputs
            .iter()
            .map(|port| port.ty.words as usize)
            .chain(node.hidden_outputs().map(|_| 1))
            .collect();

        // Make room for the results where they belong, or write the ones
        // the CPU knows already, then bring the device up to date with what
        // the CPU wrote and the new pages.
        let kernel = self.pipelines[node.id].as_ref().unwrap().kernel;
        let targets: Vec<Vec<Option<Handle>>> = batch
            .iter()
            .map(|call| {
                (0..words.len())
                    .map(|output| match kernel.outputs[output].words {
                        Some(make) => {
                            let values = make(&Inputs::new(store, node, &call.inputs));
                            store.insert(call.item.outputs[output], &values);
                            None
                        }
                        None => {
                            let len = (call.elements[output] * words[output]) as u32;
                            Some(store.reserve(call.item.outputs[output], len))
                        }
                    })
                    .collect()
            })
            .collect();
        self.upload(store);
        let reachable = targets.iter().flatten().flatten().all(|target| self.reachable(target));
        if !reachable {
            for call in batch {
                self.run_on_cpu(store, call.item);
            }
            return;
        }

        for output in 0..words.len() {
            if kernel.outputs[output].words.is_some() {
                continue;
            }
            let mut calls = Vec::new();
            for (call, targets) in batch.iter().zip(&targets) {
                for input in &call.inputs {
                    calls.extend([input.page, input.offset, input.len]);
                }
                let target = targets[output].unwrap();
                calls.extend([target.page, target.offset, call.elements[output] as u32]);
            }
            let most = batch.iter().map(|call| call.elements[output]).max().unwrap_or(0);
            if most == 0 {
                continue;
            }
            let groups = (
                most.div_ceil(WORKGROUP_SIZE as usize) as u32,
                batch.len() as u32,
            );
            let pipeline = self.pipelines[node.id].as_ref().unwrap().pipelines[output].clone();
            self.record(node.id, &pipeline, &calls, groups);
        }
        self.stats.gpu += batch.len();
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
        self.submit();
    }

    /// Copies the parts together on the device. Nothing is brought over.
    fn gather(&mut self, store: &mut Store, parts: &[ContentKey], output: ContentKey) {
        let handles: Vec<_> = parts.iter().filter_map(|part| store.get(*part)).collect();
        let len: u32 = handles.iter().map(|handle| handle.len).sum();
        if len == 0 {
            store.insert(output, &[]);
            return;
        }
        let target = store.reserve(output, len);
        // Parts the CPU made are only in the store so far.
        self.upload(store);

        let reachable = self.bindless.is_some()
            && self.reachable(&target)
            && handles.iter().all(|handle| self.reachable(handle));
        if !reachable {
            self.download(store, parts);
            let mut words = Vec::with_capacity(len as usize);
            for handle in &handles {
                words.extend_from_slice(store.words(*handle));
            }
            store.insert(output, &words);
            return;
        }

        // A part longer than one dispatch can reach is copied in pieces.
        let max_groups = self.device.limits().max_compute_workgroups_per_dimension;
        let longest = max_groups * 64;
        let mut calls = Vec::new();
        let mut offset = target.offset;
        for handle in handles.iter().filter(|handle| handle.len > 0) {
            let mut start = 0;
            while start < handle.len {
                let len = (handle.len - start).min(longest);
                calls.push([handle.page, handle.offset + start, len, target.page, offset]);
                offset += len;
                start += len;
            }
        }
        let pipeline = self.bindless.as_ref().unwrap().gather.clone();
        let max_rows = (CALL_WINDOW as usize / GATHER_CALL_WORDS).min(max_groups as usize);
        for chunk in calls.chunks(max_rows) {
            let most = chunk.iter().map(|call| call[2]).max().unwrap_or(0);
            let flat: Vec<u32> = chunk.iter().flatten().copied().collect();
            self.record("Oneiroi Gather", &pipeline, &flat, (most.div_ceil(64), chunk.len() as u32));
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

        // What is to be read may not even be computed yet.
        self.submit();
        if self.readback.as_ref().is_none_or(|readback| readback.size() < size) {
            self.readback = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Oneiroi Readback"),
                size: size.next_power_of_two(),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }));
            self.stats.buffers += 1;
        }
        let readback = self.readback.as_ref().unwrap();

        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mut offset = 0;
        for (_, handle) in &remote {
            let bytes = handle.len as u64 * 4;
            let page = &self.pages[handle.page as usize];
            encoder.copy_buffer_to_buffer(page, handle.offset as u64 * 4, readback, offset, bytes);
            offset += bytes;
        }
        self.queue.submit([encoder.finish()]);
        self.stats.submits += 1;

        readback.map_async(wgpu::MapMode::Read, ..size, |result| {
            result.expect("the readback buffer is only used here")
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device is alive while its executor is");
        {
            let bytes = readback
                .get_mapped_range(..size)
                .expect("the buffer was mapped right above");
            let words: &[u32] = bytemuck::cast_slice(&bytes);

            let mut offset = 0;
            for (key, handle) in &remote {
                store.fill(*key, &words[offset..offset + handle.len as usize]);
                offset += handle.len as usize;
                self.stats.downloads += 1;
            }
        }
        readback.unmap();
    }
}

/// An [`Executor`] for when the graphics device only turns up later, as it
/// does inside a user interface which owns it.
///
/// Until a device is [attached](DeferredExecutor::attach) everything runs
/// on the CPU. Nothing is lost by that: what the CPU computed is uploaded
/// once the device is there.
#[derive(Default)]
pub struct DeferredExecutor {
    gpu: Option<GpuExecutor>,
    cpu: CpuExecutor,
}

impl DeferredExecutor {
    pub fn attach(&mut self, device: wgpu::Device, queue: wgpu::Queue) {
        self.gpu = Some(GpuExecutor::new(device, queue));
    }

    pub fn gpu(&self) -> Option<&GpuExecutor> {
        self.gpu.as_ref()
    }

    pub fn gpu_mut(&mut self) -> Option<&mut GpuExecutor> {
        self.gpu.as_mut()
    }
}

impl Executor for DeferredExecutor {
    fn execute(&mut self, store: &mut Store, work: &[Work]) {
        match &mut self.gpu {
            Some(gpu) => gpu.execute(store, work),
            None => self.cpu.execute(store, work),
        }
    }

    fn download(&mut self, store: &mut Store, keys: &[ContentKey]) {
        if let Some(gpu) = &mut self.gpu {
            gpu.download(store, keys);
        }
    }

    fn gather(&mut self, store: &mut Store, parts: &[ContentKey], output: ContentKey) {
        match &mut self.gpu {
            Some(gpu) => gpu.gather(store, parts, output),
            None => self.cpu.gather(store, parts, output),
        }
    }
}
