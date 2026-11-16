//! The engine- and backend-agnostic core of oneiroi.
//!
//! - [`types`]: what can flow between nodes.
//! - [`store`]: the content-addressed, paged storage shared by all backends.
//! - [`node`]: how a node describes itself and its kernels.
//! - [`program`]: the static, compiled form of a graph.
//! - [`exec`]: planning, the [`exec::Executor`] interface and the CPU backend.

pub mod exec;
pub mod node;
pub mod program;
pub mod property;
pub mod store;
pub mod types;
