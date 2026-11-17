//! Running a scenario again and again on one backend and timing it.

use std::time::{Duration, Instant};

use oneiroi_core::exec::{CpuExecutor, Engine, Executor, Instance};
use oneiroi_wgpu::{GpuExecutor, GpuStats};
use serde::{Deserialize, Serialize};

use crate::scenario::{Params, Scenario, Size};

/// Where the nodes run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Backend {
    /// On the CPU, on a single thread.
    #[serde(rename = "cpu-1")]
    Cpu1,
    /// On the CPU, on every core.
    #[serde(rename = "cpu")]
    Cpu,
    /// On the GPU where a node has a kernel, else on the CPU.
    #[serde(rename = "gpu")]
    Gpu,
}

impl Backend {
    pub const ALL: [Backend; 3] = [Backend::Cpu1, Backend::Cpu, Backend::Gpu];

    pub fn name(self) -> &'static str {
        match self {
            Backend::Cpu1 => "cpu-1",
            Backend::Cpu => "cpu",
            Backend::Gpu => "gpu",
        }
    }
}

/// How long to measure for.
#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub warmup: u32,
    pub min_runs: u32,
    pub max_runs: u32,
    /// Measures at least this long, if `max_runs` allows.
    pub min_time: Duration,
    /// Stops after this long, once there are a few runs.
    pub max_time: Duration,
    /// Runs with timestamps on the GPU, after the timed ones.
    pub profiled: u32,
}

impl Settings {
    pub const FULL: Settings = Settings {
        warmup: 3,
        min_runs: 10,
        max_runs: 200,
        min_time: Duration::from_millis(500),
        max_time: Duration::from_secs(5),
        profiled: 5,
    };

    pub const QUICK: Settings = Settings {
        warmup: 1,
        min_runs: 5,
        max_runs: 50,
        min_time: Duration::from_millis(100),
        max_time: Duration::from_secs(1),
        profiled: 3,
    };
}

/// The median and the spread of some runs, in microseconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    pub median: f64,
    pub p10: f64,
    pub p90: f64,
}

impl Stats {
    fn of(mut values: Vec<f64>) -> Stats {
        values.sort_by(f64::total_cmp);
        let at = |fraction: f64| {
            let index = ((values.len() - 1) as f64 * fraction).round() as usize;
            values[index]
        };
        Stats {
            median: at(0.5),
            p10: at(0.1),
            p90: at(0.9),
        }
    }

    /// How far apart the runs are, relative to the median.
    pub fn spread(&self) -> f64 {
        (self.p90 - self.p10) / self.median.max(f64::MIN_POSITIVE)
    }
}

/// What the executor did for one edit. Only the GPU executor counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counters {
    /// Nodes run on the GPU.
    pub gpu_nodes: usize,
    /// Nodes run on the CPU.
    pub cpu_nodes: usize,
    pub dispatches: usize,
    pub submits: usize,
    pub downloads: usize,
    /// Buffers made, which should be none once the store has its pages.
    pub buffers: usize,
}

impl Counters {
    fn between(before: GpuStats, after: GpuStats) -> Counters {
        Counters {
            gpu_nodes: after.gpu - before.gpu,
            cpu_nodes: after.cpu - before.cpu,
            dispatches: after.dispatches - before.dispatches,
            submits: after.submits - before.submits,
            downloads: after.downloads - before.downloads,
            buffers: after.buffers - before.buffers,
        }
    }
}

/// The time of one edit, split up.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Measurement {
    /// From setting the new value until the result is computed, on the GPU
    /// included.
    pub total: Stats,
    /// Setting the value and finding out what has to run.
    pub plan: Stats,
    /// Running it on the CPU, and handing it to the GPU.
    pub execute: Stats,
    /// Waiting for the GPU to finish.
    pub wait: Stats,
    /// How long the GPU worked on compute passes, by its timestamps.
    pub device: Option<Stats>,
    pub runs: u32,
    /// What the GPU executor did for the first measured edit.
    pub counters: Option<Counters>,
}

/// What the measuring needs from an executor besides running work.
pub trait Measured: Executor {
    fn wait(&mut self) {}

    fn stats(&self) -> Option<GpuStats> {
        None
    }

    /// Turns on timestamps, if the backend has any.
    fn profile(&mut self) -> bool {
        false
    }

    fn device_time(&mut self) -> Option<Duration> {
        None
    }
}

impl Measured for CpuExecutor {}

impl Measured for GpuExecutor {
    fn wait(&mut self) {
        GpuExecutor::wait(self);
    }

    fn stats(&self) -> Option<GpuStats> {
        Some(GpuExecutor::stats(self))
    }

    fn profile(&mut self) -> bool {
        self.enable_profiling()
    }

    fn device_time(&mut self) -> Option<Duration> {
        self.take_gpu_time()
    }
}

fn set<E: Executor>(engine: &mut Engine<E>, instance: &mut Instance, params: &Params) {
    for (name, words) in params {
        engine
            .set_param_words(instance, name, words)
            .unwrap_or_else(|error| panic!("{error}"));
    }
}

fn micros(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e6
}

/// Sets up `scenario` at `size` and evaluates it once, ready for edits.
pub fn prepare<E: Measured>(engine: &mut Engine<E>, scenario: &Scenario, size: Size) -> Instance {
    let mut registry = oneiroi_core::node::NodeRegistry::new();
    oneiroi_nodes::register(&mut registry);
    let id = engine.add_program((scenario.build)(&registry));
    let mut instance = engine.instantiate(id);
    set(engine, &mut instance, &(scenario.setup)(size));
    engine.evaluate([&mut instance]);
    engine.executor_mut().wait();
    engine.end_frame(0);
    instance
}

/// One edit, timed.
struct Run {
    plan: Duration,
    execute: Duration,
    wait: Duration,
}

fn edit<E: Measured>(engine: &mut Engine<E>, instance: &mut Instance, params: &Params) -> Run {
    let start = Instant::now();
    set(engine, instance, params);
    engine.plan([&mut *instance]);
    let planned = Instant::now();
    engine.flush();
    let executed = Instant::now();
    engine.executor_mut().wait();
    let done = Instant::now();
    // What the edit made before is of no use anymore, what it reused stays.
    engine.end_frame(0);
    Run {
        plan: planned - start,
        execute: executed - planned,
        wait: done - executed,
    }
}

/// Edits `scenario` at `size` again and again on `engine`.
pub fn measure<E: Measured>(
    mut engine: Engine<E>,
    scenario: &Scenario,
    size: Size,
    settings: Settings,
) -> Measurement {
    let mut instance = prepare(&mut engine, scenario, size);
    let mut n = 0;
    let next = |n: &mut u32| {
        *n += 1;
        (scenario.edit)(size, *n)
    };

    for _ in 0..settings.warmup {
        let params = next(&mut n);
        edit(&mut engine, &mut instance, &params);
    }

    let mut runs = Vec::new();
    let mut counters = None;
    let started = Instant::now();
    loop {
        let elapsed = started.elapsed();
        let enough = runs.len() as u32 >= settings.max_runs
            || (runs.len() as u32 >= settings.min_runs && elapsed >= settings.min_time)
            || (runs.len() >= 3 && elapsed >= settings.max_time);
        if enough {
            break;
        }
        let params = next(&mut n);
        let before = engine.executor().stats();
        runs.push(edit(&mut engine, &mut instance, &params));
        if counters.is_none() {
            counters = before.zip(engine.executor().stats()).map(|(a, b)| Counters::between(a, b));
        }
    }

    // With timestamps every submit waits, so these only count for the GPU.
    let mut device = Vec::new();
    if settings.profiled > 0 && engine.executor_mut().profile() {
        engine.executor_mut().device_time();
        for _ in 0..settings.profiled {
            let params = next(&mut n);
            edit(&mut engine, &mut instance, &params);
            if let Some(time) = engine.executor_mut().device_time() {
                device.push(micros(time));
            }
        }
    }

    let stats = |part: fn(&Run) -> Duration| Stats::of(runs.iter().map(|run| micros(part(run))).collect());
    Measurement {
        total: stats(|run| run.plan + run.execute + run.wait),
        plan: stats(|run| run.plan),
        execute: stats(|run| run.execute),
        wait: stats(|run| run.wait),
        device: (!device.is_empty()).then(|| Stats::of(device)),
        runs: runs.len() as u32,
        counters,
    }
}

/// [`measure`] on the CPU, on `threads` threads or on every core.
pub fn measure_cpu(scenario: &Scenario, size: Size, settings: Settings, threads: Option<usize>) -> Measurement {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads.unwrap_or(0))
        .build()
        .expect("a thread pool can be made");
    pool.install(|| measure(Engine::new(CpuExecutor::default()), scenario, size, settings))
}

/// [`measure`] on `device`.
pub fn measure_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scenario: &Scenario,
    size: Size,
    settings: Settings,
) -> Measurement {
    let executor = GpuExecutor::new(device.clone(), queue.clone());
    measure(Engine::new(executor), scenario, size, settings)
}
