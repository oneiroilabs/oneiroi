//! Profiling hooks (Tracy). Everything here compiles to nothing unless the
//! `tracy` feature is enabled, so call sites need no `cfg`.
//!
//! * CPU: `cpu_zone!("name");` opens a zone until the end of the enclosing block.
//! * GPU: wrap each pass with [`Profiler::begin_pass`] / [`Profiler::end_pass`]
//!   and hand the timestamp writes to the pass descriptor. Passes show up on a
//!   GPU track in Tracy, correlated with the CPU timeline.
//! Usage: start the Tracy GUI, then
//! `cargo run -p oneiroi-wgpu --profile profiling --features tracy`.
//! For Radeon GPU Profiler no feature is needed: launch the normal
//! `--profile profiling` binary from the Radeon Developer Panel; the pass
//! labels used here appear as markers.
//!
//! * Per frame: [`Profiler::resolve`] before `submit`, [`Profiler::end_frame`] after.

/// Open a Tracy CPU zone lasting until the end of the enclosing block.
#[cfg(feature = "tracy")]
#[macro_export]
macro_rules! cpu_zone {
    ($name:literal) => {
        let _cpu_zone = $crate::profiling::tracy_client::span!($name);
    };
}
#[cfg(not(feature = "tracy"))]
#[macro_export]
macro_rules! cpu_zone {
    ($name:literal) => {};
}

#[cfg(feature = "tracy")]
pub use tracy_client;

#[cfg(feature = "tracy")]
mod imp {
    use wgpu_profiler::{GpuProfiler, GpuProfilerQuery, GpuProfilerSettings};

    pub struct Profiler {
        // Keeps the Tracy client alive; dropped last.
        _client: tracy_client::Client,
        gpu: GpuProfiler,
    }

    pub struct PassScope(GpuProfilerQuery);

    impl Profiler {
        /// Timer features to request on the device (only those the adapter has).
        pub fn wanted_features(adapter: &wgpu::Adapter) -> wgpu::Features {
            GpuProfiler::ALL_WGPU_TIMER_FEATURES & adapter.features()
        }

        pub fn new(adapter: &wgpu::Adapter, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
            let client = tracy_client::Client::start();
            let gpu = GpuProfiler::new_with_tracy_client(
                GpuProfilerSettings::default(),
                adapter.get_info().backend,
                device,
                queue,
            )
            .expect("failed to create GPU profiler (timestamp queries unsupported?)");
            Self {
                _client: client,
                gpu,
            }
        }

        pub fn begin_pass(&self, label: &str, encoder: &mut wgpu::CommandEncoder) -> PassScope {
            PassScope(self.gpu.begin_pass_query(label, encoder))
        }

        pub fn end_pass(&self, encoder: &mut wgpu::CommandEncoder, scope: PassScope) {
            self.gpu.end_query(encoder, scope.0);
        }

        /// Call after the last pass, before `encoder.finish()`.
        pub fn resolve(&mut self, encoder: &mut wgpu::CommandEncoder) {
            self.gpu.resolve_queries(encoder);
        }

        /// Call after `queue.submit`. Results are forwarded to Tracy as they become available.
        pub fn end_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
            if let Err(e) = self.gpu.end_frame() {
                eprintln!("gpu profiler end_frame: {e}");
            }
            let _ = device.poll(wgpu::PollType::Poll);
            while self.gpu.process_finished_frame(queue.get_timestamp_period()).is_some() {}
            tracy_client::frame_mark();
        }
    }

    impl PassScope {
        pub fn compute_writes(&self) -> Option<wgpu::ComputePassTimestampWrites<'_>> {
            self.0.compute_pass_timestamp_writes()
        }
        pub fn render_writes(&self) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
            self.0.render_pass_timestamp_writes()
        }
    }
}

#[cfg(not(feature = "tracy"))]
mod imp {
    pub struct Profiler;
    pub struct PassScope;

    impl Profiler {
        #[inline(always)]
        pub fn wanted_features(_: &wgpu::Adapter) -> wgpu::Features {
            wgpu::Features::empty()
        }
        #[inline(always)]
        pub fn new(_: &wgpu::Adapter, _: &wgpu::Device, _: &wgpu::Queue) -> Self {
            Self
        }
        #[inline(always)]
        pub fn begin_pass(&self, _: &str, _: &mut wgpu::CommandEncoder) -> PassScope {
            PassScope
        }
        #[inline(always)]
        pub fn end_pass(&self, _: &mut wgpu::CommandEncoder, _: PassScope) {}
        #[inline(always)]
        pub fn resolve(&mut self, _: &mut wgpu::CommandEncoder) {}
        #[inline(always)]
        pub fn end_frame(&mut self, _: &wgpu::Device, _: &wgpu::Queue) {}
    }

    impl PassScope {
        #[inline(always)]
        pub fn compute_writes(&self) -> Option<wgpu::ComputePassTimestampWrites<'_>> {
            None
        }
        #[inline(always)]
        pub fn render_writes(&self) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
            None
        }
    }
}

pub use imp::{PassScope, Profiler};
