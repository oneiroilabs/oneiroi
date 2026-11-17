//! Which machine the numbers come from. Times are only compared with times
//! of the same machine.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gpu {
    pub name: String,
    pub backend: String,
    pub driver: String,
    pub driver_info: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Machine {
    pub os: String,
    pub cpu: String,
    pub threads: usize,
    pub gpu: Option<Gpu>,
}

impl Machine {
    pub fn new(adapter: Option<&wgpu::AdapterInfo>) -> Machine {
        Machine {
            os: std::env::consts::OS.to_string(),
            cpu: cpu_name(),
            threads: std::thread::available_parallelism().map_or(1, |threads| threads.get()),
            gpu: adapter.map(|info| Gpu {
                name: info.name.clone(),
                backend: format!("{:?}", info.backend).to_lowercase(),
                driver: info.driver.clone(),
                driver_info: info.driver_info.clone(),
            }),
        }
    }

    /// A name for the files of this machine: operating system, CPU, GPU
    /// and its backend. The driver is left out, so that its results can be
    /// compared with the ones from before an update.
    pub fn id(&self) -> String {
        let gpu = self
            .gpu
            .as_ref()
            .map_or("no-gpu".to_string(), |gpu| format!("{} {}", gpu.name, gpu.backend));
        slug(&format!("{} {} {}", self.os, self.cpu, gpu))
    }
}

fn slug(text: &str) -> String {
    let mut slug = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}

fn cpu_name() -> String {
    if let Ok(info) = std::fs::read_to_string("/proc/cpuinfo")
        && let Some(line) = info.lines().find(|line| line.starts_with("model name"))
        && let Some((_, name)) = line.split_once(':')
    {
        return name.trim().to_string();
    }
    if let Ok(name) = std::env::var("PROCESSOR_IDENTIFIER") {
        return name;
    }
    std::env::consts::ARCH.to_string()
}

/// A device to measure on, as the tests make it: `WGPU_BACKEND` picks the
/// backend, and shaders are compiled with optimizations unless
/// `WGPU_DEBUG=1`.
pub fn gpu_device() -> Option<(wgpu::Device, wgpu::Queue, wgpu::AdapterInfo)> {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::from_env().unwrap_or(descriptor.backends);
    descriptor.flags = (descriptor.flags - wgpu::InstanceFlags::DEBUG).with_env();
    let instance = wgpu::Instance::new(descriptor);
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
    let wanted = oneiroi_wgpu::FEATURES | wgpu::Features::TIMESTAMP_QUERY;
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("Oneiroi Benchmarks"),
        required_features: adapter.features() & wanted,
        required_limits: adapter.limits(),
        ..Default::default()
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&descriptor)).ok()?;
    if !oneiroi_wgpu::GpuExecutor::supports(&device) {
        return None;
    }
    Some((device, queue, adapter.get_info()))
}
