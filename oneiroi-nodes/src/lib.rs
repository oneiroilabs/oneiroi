use oneiroi_core::node::NodeRegistry;

pub mod curve;
mod generic;
pub mod primitive;
pub mod property;
pub mod surface;
pub mod value;

/// Makes every node of this crate known to `registry`.
pub fn register(registry: &mut NodeRegistry) {
    curve::register(registry);
    value::register(registry);
}
