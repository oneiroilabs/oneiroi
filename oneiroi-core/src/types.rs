use glam::{Vec2, Vec3};

/// The main way to declare a DataType.
/// Can afterwards be used as a Property<T>.
pub trait DataType: Clone + Default {
    type ConfigurationOptions: Clone;
}

impl DataType for Vec3 {
    type ConfigurationOptions = ();
}

impl DataType for Vec2 {
    type ConfigurationOptions = ();
}
