use std::{
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use oneiroi_bench::{
    machine::{Machine, gpu_device},
    measure::{Backend, Settings, measure_cpu, measure_gpu},
    report::{self, Entry, Results},
    scenario::SCENARIOS,
};

const USAGE: &str = "\
Measures how long edits take on the CPU and on the GPU.

Usage: oneiroi-bench [options]

  --quick              fewer sizes and runs, for a quick look
  --only <scenario>    only this scenario, can be given several times
  --backends <list>    which of cpu-1, cpu and gpu, separated by commas
  --accept             make the results the baseline of this machine

The baseline lives in benchmarks/results, a file per machine. Every run is
compared with it. WGPU_BACKEND picks the backend of the GPU.";

struct Options {
    quick: bool,
    accept: bool,
    only: Vec<String>,
    backends: Vec<Backend>,
}

fn options() -> Options {
    let mut options = Options {
        quick: false,
        accept: false,
        only: Vec::new(),
        backends: Backend::ALL.to_vec(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--quick" => options.quick = true,
            "--accept" => options.accept = true,
            "--only" => options.only.push(args.next().expect("--only needs a scenario")),
            "--backends" => {
                let list = args.next().expect("--backends needs a list");
                options.backends = list
                    .split(',')
                    .map(|name| {
                        Backend::ALL
                            .into_iter()
                            .find(|backend| backend.name() == name.trim())
                            .unwrap_or_else(|| panic!("there is no backend `{name}`"))
                    })
                    .collect();
            }
            "--help" | "-h" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            // What `cargo bench` passes along.
            "--bench" => {}
            other => panic!("unknown option `{other}`\n\n{USAGE}"),
        }
    }
    for name in &options.only {
        assert!(
            SCENARIOS.iter().any(|scenario| scenario.name == name),
            "there is no scenario `{name}`"
        );
    }
    options
}

fn commit() -> String {
    Command::new("git")
        .args(["describe", "--always", "--dirty"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map_or("unknown".into(), |output| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn main() {
    let options = options();
    if cfg!(debug_assertions) {
        eprintln!("warning: a debug build, the numbers mean little. Use --release.");
    }
    let settings = if options.quick { Settings::QUICK } else { Settings::FULL };

    let device = options
        .backends
        .contains(&Backend::Gpu)
        .then(gpu_device)
        .flatten();
    if options.backends.contains(&Backend::Gpu) && device.is_none() {
        eprintln!("no GPU which can run kernels, measuring the CPU only");
    }
    let machine = Machine::new(device.as_ref().map(|(_, _, info)| info));
    let baseline = report::load(&machine);
    eprintln!("machine {}", machine.id());
    match &baseline {
        Some(baseline) => eprintln!("comparing with the baseline of {}", baseline.commit),
        None => eprintln!("no baseline for this machine yet"),
    }

    let mut entries = Vec::new();
    for scenario in SCENARIOS {
        if !options.only.is_empty() && !options.only.iter().any(|name| name == scenario.name) {
            continue;
        }
        let sizes = if options.quick { scenario.quick } else { scenario.sizes };
        for &size in sizes {
            for &backend in &options.backends {
                let measurement = match backend {
                    Backend::Cpu1 => measure_cpu(scenario, size, settings, Some(1)),
                    Backend::Cpu => measure_cpu(scenario, size, settings, None),
                    Backend::Gpu => match &device {
                        Some((device, queue, _)) => measure_gpu(device, queue, scenario, size, settings),
                        None => continue,
                    },
                };
                eprintln!(
                    "{:>12} {:>12} {:>5}: {:>10.1} µs ({} runs)",
                    scenario.name,
                    size.label(),
                    backend.name(),
                    measurement.total.median,
                    measurement.runs
                );
                entries.push(Entry {
                    scenario: scenario.name.into(),
                    size,
                    backend,
                    measurement,
                });
            }
        }
    }

    let results = Results {
        machine,
        commit: commit(),
        measured_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs()),
        entries,
    };
    println!("{}", report::table(&results, baseline.as_ref()));

    if options.accept {
        let results = match &baseline {
            Some(baseline) => results.merge(baseline),
            None => results,
        };
        match report::accept(&results) {
            Ok(path) => eprintln!("the new baseline is {}", path.display()),
            Err(error) => eprintln!("could not write the baseline: {error}"),
        }
    }
}
