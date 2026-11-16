use bytemuck::Pod;
use glam::{Vec2, Vec3};

/// Layout information of a [`DataType`] shared by every backend.
#[derive(Debug, Clone, Copy)]
pub struct TypeInfo {
    /// Unique name. Ports are type-checked by comparing it.
    pub name: &'static str,
    /// The WGSL type the generated page loader decodes into.
    pub wgsl: &'static str,
    /// Size of one element in 32-bit words.
    pub words: u32,
}

impl PartialEq for TypeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.words == other.words
    }
}

/// The main way to declare a DataType.
/// Can afterwards be used as a `Property<T>` and flow between nodes.
///
/// Every value is stored as a slice of `Self` inside the word pages of the
/// [`Store`](crate::store::Store), so a type must be made of whole 32-bit
/// words and must not need more than 4 byte alignment.
pub trait DataType: Pod {
    type ConfigurationOptions: Clone;

    const NAME: &'static str;
    const WGSL: &'static str;

    const INFO: TypeInfo = {
        assert!(
            align_of::<Self>() <= 4 && size_of::<Self>() % 4 == 0 && size_of::<Self>() > 0,
            "a DataType must be made of whole 32-bit words"
        );
        TypeInfo {
            name: Self::NAME,
            wgsl: Self::WGSL,
            words: (size_of::<Self>() / 4) as u32,
        }
    };
}

macro_rules! data_type {
    ($ty:ty, $name:literal, $wgsl:literal) => {
        impl DataType for $ty {
            type ConfigurationOptions = ();

            const NAME: &'static str = $name;
            const WGSL: &'static str = $wgsl;
        }
    };
}

data_type!(u32, "u32", "u32");
data_type!(f32, "f32", "f32");
data_type!(Vec2, "vec2", "vec2<f32>");
data_type!(Vec3, "vec3", "vec3<f32>");
// glam's Vec4 is 16 byte aligned, so the plain array is the storage form.
data_type!([f32; 4], "vec4", "vec4<f32>");
