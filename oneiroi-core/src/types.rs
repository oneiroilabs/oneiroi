use glam::{Vec2, Vec3};

pub enum DataTypeVariant {
    CubicNurbs,
    Vec3,
    Vec2,
    U32,
}

/// The main way to declare a DataType.
/// Can afterwards be used as a Property<T>.
pub trait DataType: Clone + Default {
    type ConfigurationOptions: Clone;

    const DATA_TYPE_VARIANT: DataTypeVariant;
}

impl DataType for Vec3 {
    type ConfigurationOptions = ();

    const DATA_TYPE_VARIANT: DataTypeVariant = DataTypeVariant::Vec3;
}

impl DataType for Vec2 {
    type ConfigurationOptions = ();

    const DATA_TYPE_VARIANT: DataTypeVariant = DataTypeVariant::Vec2;
}

impl DataType for u32 {
    type ConfigurationOptions = ();

    const DATA_TYPE_VARIANT: DataTypeVariant = DataTypeVariant::U32;
}
