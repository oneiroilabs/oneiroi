use oneiroi_core::node::NodeRegistry;

pub mod curve;
pub mod primitive;
pub mod property;

/// Makes every node of this crate known to `registry`.
pub fn register(registry: &mut NodeRegistry) {
    curve::register(registry);
}
