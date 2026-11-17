//! What the GPU executor does for one edit of every scenario: which nodes
//! run where, how many passes, submits and downloads. These numbers do not
//! depend on how fast the machine is, so they are kept as a snapshot. A
//! change shows up here before it shows up in any timing.
//!
//! This needs a graphics adapter which can run kernels. A software one is
//! enough. Without any it passes without having checked anything.

use std::time::Duration;

use oneiroi_bench::{
    machine::gpu_device,
    measure::{Counters, Settings, measure_gpu},
    scenario::{SCENARIOS, Size},
};
use serde::Serialize;

/// A single edit after one to warm up.
const ONCE: Settings = Settings {
    warmup: 1,
    min_runs: 1,
    max_runs: 1,
    min_time: Duration::ZERO,
    max_time: Duration::ZERO,
    profiled: 0,
};

#[derive(Serialize)]
struct Edit {
    scenario: &'static str,
    counters: Counters,
}

#[test]
fn every_edit_does_what_it_did() {
    let Some((device, queue, _)) = gpu_device() else {
        eprintln!("no GPU which can run kernels, nothing was checked");
        return;
    };
    let size = Size::new(4, 32);
    let edits: Vec<Edit> = SCENARIOS
        .iter()
        .map(|scenario| Edit {
            scenario: scenario.name,
            counters: measure_gpu(&device, &queue, scenario, size, ONCE)
                .counters
                .expect("the GPU executor counts"),
        })
        .collect();
    insta::assert_yaml_snapshot!(edits);
}
