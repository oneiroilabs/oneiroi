//! Generates the shaders GPU backends run.
//!
//! A node only brings the function which computes one element of its
//! output. Everything around it is written here: the declarations of the
//! data types, the functions which read the inputs out of the pages of the
//! store, and the entry point which writes the output.

mod args;
mod compute;
mod draw;
mod layout;
mod lod;
#[cfg(test)]
mod tests;

pub use args::{CULL_COUNT, CULL_DRAW, CULL_TASKS, CULL_WORDS, CullArgs, DrawArgs, InstanceArgs, LodArgs};
pub use compute::{ComputeShader, GATHER, GATHER_CALL_WORDS, compute, stage_entry};
pub use draw::{DrawShader, cull, draw};
pub(crate) use layout::{declare, read_element};
pub use lod::{LodColors, LodMethod, LodShader, MAX_LOD_LEVEL, draw_lod};

use std::fmt;

/// How many elements one workgroup of a generated compute shader covers.
pub const WORKGROUP_SIZE: u32 = 64;

/// How many words of calls one dispatch can see.
pub const CALL_WINDOW: u32 = 1 << 14;

/// What a device needs for the shaders of this module, as `naga` calls it.
fn bindless() -> naga::valid::Capabilities {
    naga::valid::Capabilities::BUFFER_BINDING_ARRAY
        | naga::valid::Capabilities::STORAGE_BUFFER_BINDING_ARRAY
        | naga::valid::Capabilities::STORAGE_BUFFER_BINDING_ARRAY_NON_UNIFORM_INDEXING
}

const BINDLESS: &str = include_str!("shader/wgsl/bindless.wgsl");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShaderError {
    /// A kernel needs an entry for every output, offsets included.
    Outputs { node: &'static str, found: usize },
    /// The shader does not compile. This is a bug in the kernel of the node.
    Invalid { node: &'static str, message: String },
}

impl fmt::Display for ShaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Outputs { node, found } => {
                write!(f, "the kernel of `{node}` has {found} outputs, not one for each")
            }
            Self::Invalid { node, message } => {
                write!(f, "the shader of `{node}` is invalid:\n{message}")
            }
        }
    }
}

impl std::error::Error for ShaderError {}

/// How many workgroups one dimension of a dispatch takes, the rest goes on
/// in the next one: see `FOLD` in the shaders.
pub const FOLD: u32 = 32768;

fn validate(node: &'static str, wgsl: &str) -> Result<(), ShaderError> {
    validate_with(node, wgsl, naga::valid::Capabilities::empty())
}

fn validate_with(
    node: &'static str,
    wgsl: &str,
    capabilities: naga::valid::Capabilities,
) -> Result<(), ShaderError> {
    let invalid = |message| ShaderError::Invalid { node, message };
    let module =
        naga::front::wgsl::parse_str(wgsl).map_err(|error| invalid(error.emit_to_string(wgsl)))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        capabilities,
    )
    .validate(&module)
    .map_err(|error| invalid(error.emit_to_string(wgsl)))?;
    Ok(())
}
