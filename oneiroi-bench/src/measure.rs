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
    fn zero() -> Stats {
        Stats {
            median: 0.0,
            p10: 0.0,
            p90: 0.0,
        }
    }

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

/// How long one part of the work of the executor took.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Phase {
    pub name: String,
    pub time: Stats,
}

/// The parts of the work of an executor, in the order they are shown. What
/// is left of `flush` is `rest`: on the GPU finding out sizes, making what
/// the CPU makes and making room in the store, on the CPU scheduling.
pub const PHASES: [&str; 8] = ["kernels", "store", "upload", "record", "submit", "download", "cpu", "rest"];

/// The time of one edit, split up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Measurement {
    /// From setting the new value until the result is computed, on the GPU
    /// included.
    pub total: Stats,
    /// Setting the new value, which hashes it. The same for every backend.
    #[serde(default = "Stats::zero")]
    pub set: Stats,
    /// Finding out what has to run. The same for every backend.
    pub plan: Stats,
    /// Running it on the CPU, and handing it to the GPU: `flush`.
    pub execute: Stats,
    /// What `execute` was spent on.
    #[serde(default)]
    pub phases: Vec<Phase>,
    /// Waiting for the GPU to finish.
    pub wait: Stats,
    /// How long the GPU worked on compute passes, by its timestamps.
    pub device: Option<Stats>,
    /// Ending the frame afterwards, which frees what is not used anymore.
    /// It is not part of `total`, the result is there before it.
    #[serde(default = "Stats::zero")]
    pub end_frame: Stats,
    pub runs: u32,
    /// What the GPU executor did for the first measured edit.
    pub counters: Option<Counters>,
}

/// What the measuring needs from an executor besides running work.
pub trait Measured: Executor {
    fn wait(&mut self) {}

    /// The time spent on parts of the work since the last call.
    fn phases(&mut self) -> Vec<(&'static str, Duration)>;

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

impl Measured for CpuExecutor {
    fn phases(&mut self) -> Vec<(&'static str, Duration)> {
        self.take_phases()
    }
}

impl Measured for GpuExecutor {
    fn wait(&mut self) {
        GpuExecutor::wait(self);
    }

    fn phases(&mut self) -> Vec<(&'static str, Duration)> {
        self.take_phases()
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

/// Sets up `scenario` at `size` and evaluates it once, ready for edits:
/// an instance of each of its graphs.
pub fn prepare<E: Measured>(engine: &mut Engine<E>, scenario: &Scenario, size: Size) -> Vec<Instance> {
    let mut registry = oneiroi_core::node::NodeRegistry::new();
    oneiroi_nodes::register(&mut registry);
    let scene = scenario.work.scene(&registry, engine.programs_mut(), size);
    let ids: Vec<_> = scene.graphs.into_iter().map(|graph| engine.add_program(graph)).collect();
    let mut instances: Vec<Instance> = scene
        .instances
        .iter()
        .map(|(graph, params)| {
            let mut instance = engine.instantiate(ids[*graph]);
            set(engine, &mut instance, params);
            instance
        })
        .collect();
    for (target, param, source, output) in scene.links {
        let (target, source) = match target < source {
            true => {
                let (before, after) = instances.split_at_mut(source);
                (&mut before[target], &after[0])
            }
            false => {
                let (before, after) = instances.split_at_mut(target);
                (&mut after[0], &before[source])
            }
        };
        engine.link(target, param, source, output).unwrap_or_else(|error| panic!("{error}"));
    }
    engine.evaluate(instances.iter_mut());
    engine.executor_mut().wait();
    engine.end_frame(0);
    instances
}

/// One edit, timed.
struct Run {
    set: Duration,
    plan: Duration,
    execute: Duration,
    wait: Duration,
    end_frame: Duration,
    /// Of `execute`, by the names of [`PHASES`].
    phases: [Duration; PHASES.len()],
}

/// Sets what an edit changes, and evaluates every instance, as a host
/// does every frame with all it shows.
fn edit<E: Measured>(engine: &mut Engine<E>, instances: &mut [Instance], edits: &[(usize, Params)]) -> Run {
    // Whatever was timed before belongs to no edit.
    engine.executor_mut().phases();
    let start = Instant::now();
    for (index, params) in edits {
        set(engine, &mut instances[*index], params);
    }
    let set_done = Instant::now();
    engine.plan(instances.iter_mut());
    let planned = Instant::now();
    engine.flush();
    let executed = Instant::now();
    let timed = engine.executor_mut().phases();
    engine.executor_mut().wait();
    let done = Instant::now();
    // What the edit made before is of no use anymore, what it reused stays.
    engine.end_frame(0);
    let ended = Instant::now();

    let execute = executed - planned;
    let mut phases = [Duration::ZERO; PHASES.len()];
    for (name, time) in timed {
        if let Some(index) = PHASES.iter().position(|phase| *phase == name) {
            phases[index] += time;
        }
    }
    let rest = PHASES.len() - 1;
    phases[rest] = execute.saturating_sub(phases.iter().sum());
    Run {
        set: set_done - start,
        plan: planned - set_done,
        execute,
        wait: done - executed,
        end_frame: ended - done,
        phases,
    }
}

/// Edits `scenario` at `size` again and again on `engine`.
pub fn measure<E: Measured>(
    mut engine: Engine<E>,
    scenario: &Scenario,
    size: Size,
    settings: Settings,
) -> Measurement {
    let mut instances = prepare(&mut engine, scenario, size);
    let mut n = 0;
    let next = |n: &mut u32| {
        *n += 1;
        scenario.work.edit(size, *n)
    };

    for _ in 0..settings.warmup {
        let params = next(&mut n);
        edit(&mut engine, &mut instances, &params);
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
        runs.push(edit(&mut engine, &mut instances, &params));
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
            edit(&mut engine, &mut instances, &params);
            if let Some(time) = engine.executor_mut().device_time() {
                device.push(micros(time));
            }
        }
    }

    let stats = |part: &dyn Fn(&Run) -> Duration| Stats::of(runs.iter().map(|run| micros(part(run))).collect());
    let phases = PHASES
        .iter()
        .enumerate()
        .filter(|(index, _)| runs.iter().any(|run| !run.phases[*index].is_zero()))
        .map(|(index, name)| Phase {
            name: name.to_string(),
            time: stats(&|run| run.phases[index]),
        })
        .collect();
    Measurement {
        total: stats(&|run| run.set + run.plan + run.execute + run.wait),
        set: stats(&|run| run.set),
        plan: stats(&|run| run.plan),
        execute: stats(&|run| run.execute),
        phases,
        wait: stats(&|run| run.wait),
        device: (!device.is_empty()).then(|| Stats::of(device)),
        end_frame: stats(&|run| run.end_frame),
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
