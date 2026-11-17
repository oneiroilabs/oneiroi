//! Measures how long an edit takes until its result is there, on the CPU
//! with one thread, on the CPU with every core, and on the GPU.
//!
//! Run `cargo run -p oneiroi-bench --release` for a table of all of it,
//! compared with the baseline of the machine. `--accept` makes the numbers
//! the new baseline.

pub mod machine;
pub mod measure;
pub mod report;
pub mod scenario;
