//! Keeping results, comparing them with the ones of before, and showing
//! them as a table.

use std::{fmt::Write, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    machine::Machine,
    measure::{Backend, Measurement, Stats},
    scenario::{SCENARIOS, Size},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub scenario: String,
    pub size: Size,
    pub backend: Backend,
    #[serde(flatten)]
    pub measurement: Measurement,
}

/// Everything one run of the benchmarks measured on one machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Results {
    pub machine: Machine,
    /// The commit the numbers were measured at.
    pub commit: String,
    /// Seconds since 1970.
    pub measured_at: u64,
    pub entries: Vec<Entry>,
}

impl Results {
    pub fn find(&self, scenario: &str, size: Size, backend: Backend) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|entry| entry.scenario == scenario && entry.size == size && entry.backend == backend)
    }

    /// Takes over what `other` measured, replacing what it measured anew.
    /// A run of only some scenarios keeps the others in the baseline.
    pub fn merge(mut self, other: &Results) -> Results {
        for entry in &other.entries {
            if self.find(&entry.scenario, entry.size, entry.backend).is_none() {
                self.entries.push(entry.clone());
            }
        }
        self
    }
}

/// Where the baselines are kept: `benchmarks/results` in the workspace.
pub fn results_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("benchmarks")
        .join("results")
}

pub fn baseline_path(machine: &Machine) -> PathBuf {
    results_dir().join(format!("{}.json", machine.id()))
}

pub fn load(machine: &Machine) -> Option<Results> {
    let text = std::fs::read_to_string(baseline_path(machine)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Writes `results` as the baseline of its machine, and a table of it next
/// to it.
pub fn accept(results: &Results) -> std::io::Result<PathBuf> {
    let path = baseline_path(&results.machine);
    std::fs::create_dir_all(results_dir())?;
    let json = serde_json::to_string_pretty(results).expect("results are plain data");
    std::fs::write(&path, json + "\n")?;
    std::fs::write(path.with_extension("md"), table(results, None))?;
    Ok(path)
}

/// How a time changed against the baseline, if it clearly did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Change {
    /// Within what the runs vary anyway.
    Same(f64),
    Faster(f64),
    Slower(f64),
}

/// Changes below this are never reported, however steady the runs are.
const MIN_CHANGE: f64 = 0.05;

pub fn change(now: &Stats, before: &Stats) -> Change {
    let ratio = now.median / before.median.max(f64::MIN_POSITIVE) - 1.0;
    let noise = (now.spread().max(before.spread()) / 2.0).max(MIN_CHANGE);
    if ratio > noise {
        Change::Slower(ratio)
    } else if ratio < -noise {
        Change::Faster(ratio)
    } else {
        Change::Same(ratio)
    }
}

/// The changes which are clearly there, to list after the table.
pub fn regressions(results: &Results, baseline: &Results) -> Vec<String> {
    let mut lines = Vec::new();
    for entry in &results.entries {
        let Some(before) = baseline.find(&entry.scenario, entry.size, entry.backend) else {
            continue;
        };
        if let Change::Slower(ratio) = change(&entry.measurement.total, &before.measurement.total) {
            lines.push(format!(
                "{} {} on {}: {:+.0}% ({} → {})",
                entry.scenario,
                entry.size.label(),
                entry.backend.name(),
                ratio * 100.0,
                time(before.measurement.total.median),
                time(entry.measurement.total.median),
            ));
        }
    }
    lines
}

fn time(micros: f64) -> String {
    if micros >= 1e6 {
        format!("{:.2} s", micros / 1e6)
    } else if micros >= 1e3 {
        format!("{:.2} ms", micros / 1e3)
    } else {
        format!("{micros:.0} µs")
    }
}

fn cell(entry: Option<&Entry>, baseline: Option<&Results>) -> String {
    let Some(entry) = entry else {
        return "–".into();
    };
    let mut text = time(entry.measurement.total.median);
    if let Some(before) = baseline.and_then(|b| b.find(&entry.scenario, entry.size, entry.backend)) {
        match change(&entry.measurement.total, &before.measurement.total) {
            Change::Same(_) => {}
            Change::Faster(ratio) => write!(text, " ({:+.0}%)", ratio * 100.0).unwrap(),
            Change::Slower(ratio) => write!(text, " (**{:+.0}%**)", ratio * 100.0).unwrap(),
        }
    }
    text
}

/// The results as markdown: a table per scenario, a row per size, and the
/// time of an edit on every backend next to each other. Against
/// `baseline`, clear changes are noted next to the times.
pub fn table(results: &Results, baseline: Option<&Results>) -> String {
    let mut out = String::new();
    let machine = &results.machine;
    let gpu = machine.gpu.as_ref().map_or("no GPU".to_string(), |gpu| {
        format!("{} on {}, driver {} {}", gpu.name, gpu.backend, gpu.driver, gpu.driver_info)
    });
    writeln!(out, "# Benchmarks\n").unwrap();
    writeln!(
        out,
        "{}, {} with {} threads, {}. Commit {}.\n",
        machine.os, machine.cpu, machine.threads, gpu.trim(), results.commit
    )
    .unwrap();
    writeln!(
        out,
        "The time from an edit until its result is computed, the median of many \
         edits. GPU work is split into planning, recording and handing it over, \
         waiting for the device, and the time the device spends on compute passes."
    )
    .unwrap();

    for scenario in SCENARIOS {
        let sizes: Vec<Size> = scenario
            .sizes
            .iter()
            .copied()
            .filter(|size| results.entries.iter().any(|e| e.scenario == scenario.name && e.size == *size))
            .collect();
        if sizes.is_empty() {
            continue;
        }
        writeln!(out, "\n## {}\n\n{}.\n", scenario.name, scenario.about).unwrap();
        writeln!(
            out,
            "| size | cpu, 1 thread | cpu | gpu | gpu against cpu | gpu: plan / record / wait / device | gpu work |"
        )
        .unwrap();
        writeln!(out, "|---|---:|---:|---:|---:|---|---|").unwrap();
        for size in sizes {
            let find = |backend| results.find(scenario.name, size, backend);
            let (cpu1, cpu, gpu) = (find(Backend::Cpu1), find(Backend::Cpu), find(Backend::Gpu));
            let speedup = match (cpu, gpu) {
                (Some(cpu), Some(gpu)) => {
                    let ratio = cpu.measurement.total.median / gpu.measurement.total.median;
                    match ratio >= 1.0 {
                        true => format!("{ratio:.1}× faster"),
                        false => format!("{:.1}× slower", 1.0 / ratio),
                    }
                }
                _ => "–".into(),
            };
            let breakdown = gpu.map_or("–".into(), |gpu| {
                let m = &gpu.measurement;
                format!(
                    "{} / {} / {} / {}",
                    time(m.plan.median),
                    time(m.execute.median),
                    time(m.wait.median),
                    m.device.map_or("–".into(), |device| time(device.median)),
                )
            });
            let work = gpu.and_then(|gpu| gpu.measurement.counters).map_or("–".into(), |c| {
                format!(
                    "{} on gpu, {} on cpu, {} passes, {} downloads",
                    c.gpu_nodes, c.cpu_nodes, c.dispatches, c.downloads
                )
            });
            writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {} |",
                size.label(),
                cell(cpu1, baseline),
                cell(cpu, baseline),
                cell(gpu, baseline),
                speedup,
                breakdown,
                work,
            )
            .unwrap();
        }
    }

    if let Some(baseline) = baseline {
        let regressions = regressions(results, baseline);
        writeln!(out, "\n## Against the baseline\n").unwrap();
        match regressions.is_empty() {
            true => writeln!(out, "Nothing got clearly slower.").unwrap(),
            false => {
                writeln!(out, "Clearly slower:\n").unwrap();
                for line in regressions {
                    writeln!(out, "- {line}").unwrap();
                }
            }
        }
    }
    out
}
