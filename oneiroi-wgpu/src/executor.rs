//! Runs nodes on the GPU.

use std::{collections::HashMap, num::NonZeroU32};

use oneiroi_core::{
    exec::{CpuExecutor, Executor, Phases, Work},
    node::{Caps, GpuKernel, Inputs, NodeDef},
    shader::{self, CALL_WINDOW, GATHER, GATHER_CALL_WORDS, ShaderError, WORKGROUP_SIZE},
    store::{ContentKey, Handle, KeyMap, Store},
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
    /// One per pass: every stage, then every output, offsets included.
    /// `None` for what the CPU makes.
    pipelines: Vec<Option<wgpu::ComputePipeline>>,
    call_words: usize,
}

/// What one pass of a kernel makes.
#[derive(Clone, Copy)]
struct Pass {
    /// The words of one element.
    words: usize,
    len: fn(&Inputs<'_>) -> usize,
    /// Made on the CPU instead.
    words_of: Option<fn(&Inputs<'_>) -> Vec<u32>>,
    /// A stage, which every pass reads, or else an output.
    stage: bool,
}

/// The passes of `kernel` of `node` in the order they run.
fn passes(node: &NodeDef, kernel: &GpuKernel) -> Vec<Pass> {
    let stages = kernel.stages.iter().map(|stage| Pass {
        words: stage.ty.words as usize,
        len: stage.len,
        words_of: stage.words,
        stage: true,
    });
    let words = node
        .outputs
        .iter()
        .map(|port| port.ty.words as usize)
        .chain(node.hidden_outputs().map(|_| 1))
        .chain(node.total_outputs().map(|_| 1));
    let outputs = kernel.outputs.iter().zip(words).map(|(output, words)| Pass {
        words,
        len: output.len,
        words_of: output.words,
        stage: false,
    });
    stages.chain(outputs).collect()
}

/// One node to run on the GPU, with what is known before it does.
struct Call<'a> {
    item: &'a Work,
    /// Its inputs and the offsets of the grouped ones.
    inputs: Vec<Handle>,
    /// How many elements each of its passes makes.
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
    /// Timestamps around every compute pass, while profiling.
    profiler: Option<Profiler>,
    phases: Phases,
}

/// Measures how long the device works on compute passes, with a timestamp
/// at the start and at the end of each.
struct Profiler {
    queries: wgpu::QuerySet,
    resolved: wgpu::Buffer,
    read: wgpu::Buffer,
    /// Timestamps written since the last submit.
    used: u32,
    /// Nanoseconds per tick of a timestamp.
    period: f64,
    /// The time of all passes measured so far.
    nanoseconds: f64,
}

/// How many passes one submit can measure. More of them submit early.
const PROFILED_PASSES: u32 = 512;

/// Whether `ONEIROI_COMPUTE=cpu` asks for every node to run on the CPU.
/// Values are still drawn from the device.
pub fn compute_disabled() -> bool {
    std::env::var("ONEIROI_COMPUTE").is_ok_and(|value| value.eq_ignore_ascii_case("cpu"))
}

impl GpuExecutor {
    /// Whether a device has what an executor needs to run anything on it.
    pub fn supports(device: &wgpu::Device) -> bool {
        device.features().contains(FEATURES)
            && device.limits().max_binding_array_elements_per_shader_stage > 0
    }

    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        // Without it a lost device shows up only later, as some buffer
        // which is invalid, far away from why.
        device.set_device_lost_callback(|reason, message| {
            eprintln!("oneiroi: the GPU device was lost ({reason:?}): {message}");
        });
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
        let bindless = (Self::supports(&device) && !compute_disabled())
            .then(|| Self::bindless(&device, &ring))
            .flatten();
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
            profiler: None,
            phases: Phases::default(),
        }
    }

    /// How long the executor spent since the last call on uploading,
    /// recording passes, submitting, downloading, and running nodes on the
    /// CPU. The rest of its time goes to finding out sizes, making what
    /// the CPU makes, and making room in the store.
    pub fn take_phases(&mut self) -> Vec<(&'static str, std::time::Duration)> {
        self.phases.take()
    }

    /// Starts measuring how long the device takes for the compute passes,
    /// see [`GpuExecutor::take_gpu_time`]. Every submit then waits for the
    /// device, so wall clock times measured meanwhile mean little. Returns
    /// whether the device can measure it.
    pub fn enable_profiling(&mut self) -> bool {
        if !self.device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return false;
        }
        if self.profiler.is_none() {
            let count = PROFILED_PASSES * 2;
            let bytes = count as u64 * 8;
            self.profiler = Some(Profiler {
                queries: self.device.create_query_set(&wgpu::QuerySetDescriptor {
                    label: Some("Oneiroi Passes"),
                    ty: wgpu::QueryType::Timestamp,
                    count,
                }),
                resolved: self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Oneiroi Passes"),
                    size: bytes,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                read: self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Oneiroi Passes"),
                    size: bytes,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                used: 0,
                period: self.queue.get_timestamp_period() as f64,
                nanoseconds: 0.0,
            });
        }
        true
    }

    /// How long the device worked on compute passes since the last call,
    /// if profiling is on.
    pub fn take_gpu_time(&mut self) -> Option<std::time::Duration> {
        self.submit();
        let profiler = self.profiler.as_mut()?;
        let nanoseconds = std::mem::take(&mut profiler.nanoseconds);
        Some(std::time::Duration::from_nanos(nanoseconds as u64))
    }

    /// Hands over what was recorded and waits until the device is done
    /// with all of it.
    pub fn wait(&mut self) {
        self.submit();
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device is alive while its executor is");
    }

    fn bindless(device: &wgpu::Device, ring: &wgpu::Buffer) -> Option<Bindless> {
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
        // Without it nothing runs on the device, since what is taken apart
        // there has to be put together there as well.
        let gather =
            Self::compute_pipeline(device, &pipeline_layout, "Oneiroi Gather", GATHER, "main")
                .inspect_err(|error| eprintln!("oneiroi: no kernels on this device: {error}"))
                .ok()?;
        Some(Bindless {
            layout,
            calls,
            pipeline_layout,
            gather,
            max_pages,
            bind_group: None,
        })
    }

    /// A pipeline of one entry of `wgsl`, or the error the device gave.
    /// A shader can be fine for naga and still be refused by the compiler
    /// of the backend, which only shows here.
    fn compute_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        label: &str,
        wgsl: &str,
        entry: &str,
    ) -> Result<wgpu::ComputePipeline, String> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(wgsl.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(label),
            layout: Some(layout),
            module: &module,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            cache: None,
        });
        match pollster::block_on(scope.pop()) {
            Some(error) => Err(error.to_string()),
            None => Ok(pipeline),
        }
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
                .and_then(|(bindless, kernel)| {
                    let shader = shader::compute(node, kernel).and_then(|shader| {
                        let stages = kernel.stages.iter().enumerate().map(|(index, stage)| {
                            stage.words.is_none().then(|| shader::stage_entry(index))
                        });
                        let outputs = kernel.outputs.iter().enumerate().map(|(index, output)| {
                            output.words.is_none().then(|| format!("main_{index}"))
                        });
                        let pipelines = stages
                            .chain(outputs)
                            .map(|entry| {
                                entry
                                    .map(|entry| {
                                        Self::compute_pipeline(
                                            &self.device,
                                            &bindless.pipeline_layout,
                                            node.id,
                                            &shader.wgsl,
                                            &entry,
                                        )
                                    })
                                    .transpose()
                            })
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|message| ShaderError::Invalid {
                                node: node.id,
                                message,
                            })?;
                        Ok(Pipeline {
                            kernel,
                            pipelines,
                            call_words: shader.call_words,
                        })
                    });
                    match shader {
                        Ok(pipeline) => Some(pipeline),
                        // The node runs on the CPU instead.
                        Err(error) => {
                            eprintln!("oneiroi: {error}");
                            self.errors.push(error);
                            None
                        }
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
        let start = std::time::Instant::now();
        self.upload_untimed(store);
        self.phases.since("upload", start);
    }

    fn upload_untimed(&mut self, store: &mut Store) {
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
        let start = std::time::Instant::now();
        self.submit_untimed();
        self.phases.since("submit", start);
    }

    fn submit_untimed(&mut self) {
        if let Some(mut encoder) = self.encoder.take() {
            let measured = self.profiler.as_ref().map_or(0, |profiler| profiler.used);
            if let Some(profiler) = self.profiler.as_ref().filter(|_| measured > 0) {
                encoder.resolve_query_set(&profiler.queries, 0..measured, &profiler.resolved, 0);
                encoder.copy_buffer_to_buffer(
                    &profiler.resolved,
                    0,
                    &profiler.read,
                    0,
                    measured as u64 * 8,
                );
            }
            self.queue.submit([encoder.finish()]);
            self.stats.submits += 1;
            if measured > 0 {
                self.read_timestamps(measured);
            }
        }
        self.ring_cursor = 0;
    }

    /// Adds up the time between the timestamps of each pass. Waits for the
    /// device to get there.
    fn read_timestamps(&mut self, measured: u32) {
        let profiler = self.profiler.as_mut().expect("only called while profiling");
        let bytes = measured as u64 * 8;
        profiler.read.map_async(wgpu::MapMode::Read, ..bytes, |result| {
            result.expect("the timestamps are only read here")
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device is alive while its executor is");
        {
            let mapped = profiler
                .read
                .get_mapped_range(..bytes)
                .expect("the buffer was mapped right above");
            // Mapped memory is only as aligned as the driver placed the
            // buffer, which can be less than a tick needs.
            let ticks: Vec<u64> = bytemuck::pod_collect_to_vec(&mapped);
            for pass in ticks.chunks_exact(2) {
                profiler.nanoseconds += pass[1].saturating_sub(pass[0]) as f64 * profiler.period;
            }
        }
        profiler.read.unmap();
        profiler.used = 0;
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
        let start = std::time::Instant::now();
        let submitted = self.phases.get("submit");
        self.record_untimed(label, pipeline, calls, groups);
        // A full ring is handed over on the way, which counts as submitting.
        let submitting = self.phases.get("submit") - submitted;
        self.phases.add("record", start.elapsed().saturating_sub(submitting));
    }

    fn record_untimed(&mut self, label: &str, pipeline: &wgpu::ComputePipeline, calls: &[u32], groups: (u32, u32)) {
        let offset = self.push_calls(calls);
        let bind_group = self.bind_group();
        let calls = self.bindless.as_ref().unwrap().calls.clone();
        if self.profiler.as_ref().is_some_and(|profiler| profiler.used == PROFILED_PASSES * 2) {
            self.submit();
        }
        let encoder = self.encoder.get_or_insert_with(|| {
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Oneiroi"),
                })
        });
        let timestamp_writes = self.profiler.as_mut().map(|profiler| {
            let first = profiler.used;
            profiler.used += 2;
            wgpu::ComputePassTimestampWrites {
                query_set: &profiler.queries,
                beginning_of_pass_write_index: Some(first),
                end_of_pass_write_index: Some(first + 1),
            }
        });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some(label),
            timestamp_writes,
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
        let passes = passes(node, kernel);
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
            let elements: Vec<usize> = passes.iter().map(|pass| (pass.len)(&view)).collect();
            let fits = elements.iter().zip(&passes).all(|(elements, pass)| {
                (*elements * pass.words) as u64 * 4 <= max_bytes
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

    /// A compute pass per stage and per output for all of `batch`.
    fn dispatch(&mut self, store: &mut Store, node: &'static NodeDef, batch: &[Call<'_>]) {
        if batch.is_empty() {
            return;
        }
        let kernel = self.pipelines[node.id].as_ref().unwrap().kernel;
        let passes = passes(node, kernel);

        // Make room for what the passes make, or write what the CPU knows
        // already, then bring the device up to date with what the CPU wrote
        // and the new pages. A stage is kept in the store under a key of
        // its own, for every pass after it to read.
        let targets: Vec<Vec<Option<Handle>>> = batch
            .iter()
            .map(|call| {
                let mut output = 0;
                passes
                    .iter()
                    .enumerate()
                    .map(|(index, pass)| {
                        let key = match pass.stage {
                            true => ContentKey::derive("stage", index as u32, &[call.item.outputs[0]]),
                            false => {
                                output += 1;
                                call.item.outputs[output - 1]
                            }
                        };
                        match pass.words_of {
                            Some(make) => {
                                let values = make(&Inputs::new(store, node, &call.inputs));
                                let handle = store.insert(key, &values);
                                pass.stage.then_some(handle)
                            }
                            None => {
                                let len = (call.elements[index] * pass.words) as u32;
                                Some(store.reserve(key, len))
                            }
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

        for (index, pass) in passes.iter().enumerate() {
            if pass.words_of.is_some() {
                continue;
            }
            let mut calls = Vec::new();
            for (call, targets) in batch.iter().zip(&targets) {
                for input in &call.inputs {
                    calls.extend([input.page, input.offset, input.len]);
                }
                for (stage, handle) in passes.iter().zip(targets).filter(|(pass, _)| pass.stage) {
                    let handle = handle.expect("every stage has a place");
                    debug_assert!(stage.stage);
                    calls.extend([handle.page, handle.offset, handle.len]);
                }
                let target = targets[index].unwrap();
                calls.extend([target.page, target.offset, call.elements[index] as u32]);
            }
            let most = batch.iter().map(|call| call.elements[index]).max().unwrap_or(0);
            if most == 0 {
                continue;
            }
            let groups = (
                most.div_ceil(WORKGROUP_SIZE as usize) as u32,
                batch.len() as u32,
            );
            let pipeline = self.pipelines[node.id].as_ref().unwrap().pipelines[index]
                .clone()
                .expect("passes made on the GPU have a pipeline");
            self.record(node.id, &pipeline, &calls, groups);
        }
        self.stats.gpu += batch.len();
    }

    fn timed_download(&mut self, store: &mut Store, keys: &[ContentKey]) {
        let start = std::time::Instant::now();
        let submitted = self.phases.get("submit");
        self.download_untimed(store, keys);
        // What was recorded is handed over first, which counts as submitting.
        let submitting = self.phases.get("submit") - submitted;
        self.phases.add("download", start.elapsed().saturating_sub(submitting));
    }

    fn download_untimed(&mut self, store: &mut Store, keys: &[ContentKey]) {
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
            // Mapped memory is only as aligned as the driver placed the
            // buffer: copied where that is less than a word needs.
            let copied: Vec<u32>;
            let words: &[u32] = match bytemuck::try_cast_slice(&bytes) {
                Ok(words) => words,
                Err(_) => {
                    copied = bytemuck::pod_collect_to_vec(&bytes);
                    &copied
                }
            };

            let mut offset = 0;
            for (key, handle) in &remote {
                store.fill(*key, &words[offset..offset + handle.len as usize]);
                offset += handle.len as usize;
                self.stats.downloads += 1;
            }
        }
        readback.unmap();
    }

    fn run_on_cpu(&mut self, store: &mut Store, item: &Work) {
        // A node on the CPU reads all of its inputs.
        self.download(store, &item.inputs);
        let start = std::time::Instant::now();
        self.cpu.execute(store, std::slice::from_ref(item));
        self.phases.since("cpu", start);
        self.stats.cpu += 1;
    }
}

impl Executor for GpuExecutor {
    fn execute(&mut self, store: &mut Store, work: &[Work]) {
        // Work which reads nothing of this batch is level 0, work which
        // reads from level 0 is level 1 and so on. Whatever shares a level
        // can run in any order, and therefore together.
        let mut produced: KeyMap<usize> = KeyMap::default();
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
        self.timed_download(store, keys);
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

    /// See [`GpuExecutor::take_phases`] and [`CpuExecutor::take_phases`].
    pub fn take_phases(&mut self) -> Vec<(&'static str, std::time::Duration)> {
        match &mut self.gpu {
            Some(gpu) => gpu.take_phases(),
            None => self.cpu.take_phases(),
        }
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
