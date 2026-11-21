//! The engine- and backend-agnostic core of oneiroi.
//!
//! - [`types`]: what can flow between nodes.
//! - [`key`]: what values are known by, so that whatever is the same is
//!   shared.
//! - [`store`]: the content-addressed, paged storage shared by all backends.
//! - [`node`]: how a node describes itself and its kernels.
//! - [`program`]: the static, compiled form of a graph.
//! - [`engine`]: instances, planning, the [`engine::Executor`] interface and
//!   the CPU backend.
//! - [`shader`]: the shaders GPU backends run, generated around the kernels.

pub mod engine;
pub mod key;
pub mod node;
pub mod program;
pub mod shader;
pub mod store;
pub mod types;
