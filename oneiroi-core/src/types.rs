use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec2, Vec3};

/// What the 32-bit words of a [`DataType`] are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scalar {
    F32,
    U32,
}

/// One part of a [`DataType`], in the order it is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    F32,
    U32,
    Vec2,
    Vec3,
    Vec4,
    /// Four columns of four floats.
    Mat4,
    /// Words which are stored but mean nothing.
    Pad(u32),
    /// Another struct, with its fields right in place.
    Struct {
        wgsl: &'static str,
        fields: &'static [Field],
    },
}

impl FieldType {
    pub const fn words(&self) -> u32 {
        match self {
            Self::F32 | Self::U32 => 1,
            Self::Vec2 => 2,
            Self::Vec3 => 3,
            Self::Vec4 => 4,
            Self::Mat4 => 16,
            Self::Pad(words) => *words,
            Self::Struct { fields, .. } => Layout::Struct(fields).words(),
        }
    }

    /// The WGSL type, which padding has none of.
    pub const fn wgsl(&self) -> Option<&'static str> {
        Some(match self {
            Self::F32 => "f32",
            Self::U32 => "u32",
            Self::Vec2 => "vec2<f32>",
            Self::Vec3 => "vec3<f32>",
            Self::Vec4 => "vec4<f32>",
            Self::Mat4 => "mat4x4<f32>",
            Self::Struct { wgsl, .. } => wgsl,
            Self::Pad(_) => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field {
    pub name: &'static str,
    pub ty: FieldType,
}

impl Field {
    pub const fn new(name: &'static str, ty: FieldType) -> Self {
        Self { name, ty }
    }

    pub const fn pad(words: u32) -> Self {
        Self::new("", FieldType::Pad(words))
    }

    /// A field which is a whole other data type.
    pub const fn nested(name: &'static str, ty: TypeInfo) -> Self {
        match ty.layout {
            Layout::Value(field) => Self::new(name, field),
            Layout::Struct(fields) => Self::new(
                name,
                FieldType::Struct {
                    wgsl: ty.wgsl,
                    fields,
                },
            ),
        }
    }
}

/// How the words of a [`DataType`] are laid out. Shaders are generated
/// from this, so a type never has to be described twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// A type WGSL has built in.
    Value(FieldType),
    /// A struct with these fields, one right after the other.
    Struct(&'static [Field]),
}

impl Layout {
    pub const fn words(&self) -> u32 {
        match self {
            Self::Value(ty) => ty.words(),
            Self::Struct(fields) => {
                let mut words = 0;
                let mut index = 0;
                while index < fields.len() {
                    words += fields[index].ty.words();
                    index += 1;
                }
                words
            }
        }
    }
}

/// Layout information of a [`DataType`] shared by every backend.
#[derive(Debug, Clone, Copy)]
pub struct TypeInfo {
    /// Unique name. Ports are type-checked by comparing it.
    pub name: &'static str,
    /// The WGSL type the generated page loader decodes into.
    pub wgsl: &'static str,
    /// Size of one element in 32-bit words.
    pub words: u32,
    /// What those words are, which is how a value is written by hand.
    pub scalar: Scalar,
    pub layout: Layout,
    /// For instances of a shape: the type of the shape they refer to. A
    /// value of such a type is a table of [`InstanceRow`]s, and the shape
    /// comes along with it as a value of its own, see
    /// [`TypeInfo::instances`].
    pub prototype: Option<&'static TypeInfo>,
}

impl TypeInfo {
    /// The type of instances of `prototype`: rows which each put a group of
    /// a value of `prototype` somewhere. Nothing of the shape is copied,
    /// it is passed along by reference, behind the rows. `name` has to be
    /// unique like any other, `instances<name of the prototype>` by
    /// convention.
    pub const fn instances(name: &'static str, prototype: &'static TypeInfo) -> TypeInfo {
        TypeInfo {
            name,
            prototype: Some(prototype),
            ..InstanceRow::INFO
        }
    }
}

impl TypeInfo {
    /// Whether values of this type are stored as elements of `element`:
    /// of the type itself, or instance rows for instances.
    pub fn stored_as(&self, element: &TypeInfo) -> bool {
        self == element || (self.prototype.is_some() && *element == InstanceRow::INFO)
    }
}

impl PartialEq for TypeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.words == other.words
    }
}

/// A set of data types which can stand in for each other at a port.
///
/// A generic node names a trait instead of a type at such a port. Which
/// data types implement it is decided by the node versions registered for
/// them, see [`NodeRegistry::register_instance`].
///
/// [`NodeRegistry::register_instance`]: crate::node::NodeRegistry::register_instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Trait {
    /// Unique name.
    pub name: &'static str,
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
    const SCALAR: Scalar;
    const LAYOUT: Layout;

    const INFO: TypeInfo = {
        assert!(
            align_of::<Self>() <= 4 && size_of::<Self>() % 4 == 0 && size_of::<Self>() > 0,
            "a DataType must be made of whole 32-bit words"
        );
        assert!(
            Self::LAYOUT.words() as usize * 4 == size_of::<Self>(),
            "the layout of a DataType must cover all of it"
        );
        TypeInfo {
            name: Self::NAME,
            wgsl: Self::WGSL,
            words: (size_of::<Self>() / 4) as u32,
            scalar: Self::SCALAR,
            layout: Self::LAYOUT,
            prototype: None,
        }
    };
}

macro_rules! data_type {
    ($ty:ty, $name:literal, $wgsl:literal, $scalar:ident, $field:ident) => {
        impl DataType for $ty {
            type ConfigurationOptions = ();

            const NAME: &'static str = $name;
            const WGSL: &'static str = $wgsl;
            const SCALAR: Scalar = Scalar::$scalar;
            const LAYOUT: Layout = Layout::Value(FieldType::$field);
        }
    };
}

data_type!(u32, "u32", "u32", U32, U32);
data_type!(f32, "f32", "f32", F32, F32);
data_type!(Vec2, "vec2", "vec2<f32>", F32, Vec2);
data_type!(Vec3, "vec3", "vec3<f32>", F32, Vec3);
// glam's Vec4 is 16 byte aligned, so the plain array is the storage form.
data_type!([f32; 4], "vec4", "vec4<f32>", F32, Vec4);

/// Turning, scaling by the same amount in every direction, and then
/// moving. Lengths only change by the scale and angles not at all, so
/// everything a shape keeps about itself, like the lengths and frames of
/// a curve, stays true of it once it is placed.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Similarity {
    /// A unit quaternion, `x`, `y`, `z` and then `w`.
    pub rotation: [f32; 4],
    pub offset: [f32; 3],
    pub scale: f32,
}

impl Similarity {
    pub const IDENTITY: Self = Self {
        rotation: [0.0, 0.0, 0.0, 1.0],
        offset: [0.0; 3],
        scale: 1.0,
    };

    pub fn new(rotation: Quat, scale: f32, offset: Vec3) -> Self {
        Self {
            rotation: rotation.normalize().to_array(),
            offset: offset.to_array(),
            scale,
        }
    }

    pub fn rotation(&self) -> Quat {
        Quat::from_array(self.rotation)
    }

    pub fn offset(&self) -> Vec3 {
        Vec3::from_array(self.offset)
    }

    /// Where `point` goes.
    pub fn point(&self, point: Vec3) -> Vec3 {
        self.rotation() * point * self.scale + self.offset()
    }

    /// Where a direction goes: only turned.
    pub fn direction(&self, direction: Vec3) -> Vec3 {
        self.rotation() * direction
    }

    /// `inner` first and then `self`.
    pub fn then(&self, inner: &Similarity) -> Similarity {
        let rotation = self.rotation() * inner.rotation();
        Similarity {
            rotation: rotation.normalize().to_array(),
            offset: self.point(inner.offset()).to_array(),
            scale: self.scale * inner.scale,
        }
    }
}

impl DataType for Similarity {
    type ConfigurationOptions = ();

    const NAME: &'static str = "similarity";
    const WGSL: &'static str = "Similarity";
    const SCALAR: Scalar = Scalar::F32;
    const LAYOUT: Layout = Layout::Struct(&[
        Field::new("rotation", FieldType::Vec4),
        Field::new("offset", FieldType::Vec3),
        Field::new("scale", FieldType::F32),
    ]);
}

/// One instance of a shape: where a group of the shape goes.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct InstanceRow {
    pub transform: Similarity,
    /// Which group of the shape.
    pub group: u32,
    /// How many elements of the shape all rows before this one have
    /// together. Every element of all instances has an index of its own
    /// that way, and the row of one is found by a binary search.
    pub first: u32,
}

/// How many elements all copies of instances have together, and how their
/// rows repeat: in blocks of `block_rows` rows which cover `block_elements`
/// elements each, every block with the same groups in the same order.
///
/// Instances on points are a block per point, for example. The row of an
/// element of all copies is then found by dividing, and by looking among
/// the rows of one block only. Without any blocks (`block_rows` of zero)
/// all rows are a single one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InstanceTotal {
    pub elements: u32,
    pub block_rows: u32,
    pub block_elements: u32,
}

impl InstanceTotal {
    /// Instances which are a single block.
    pub fn single(rows: u32, elements: u32) -> Self {
        Self {
            elements,
            block_rows: rows,
            block_elements: elements,
        }
    }

    /// `count` times the instances of `block`, one after the other.
    pub fn repeated(count: u32, block: InstanceTotal, rows: u32) -> Self {
        Self {
            elements: count * block.elements,
            block_rows: rows,
            block_elements: block.elements,
        }
    }

    pub fn words(&self) -> Vec<u32> {
        vec![self.elements, self.block_rows, self.block_elements]
    }

    pub fn of_words(words: &[u32]) -> Self {
        let word = |index: usize| words.get(index).copied().unwrap_or(0);
        Self {
            elements: word(0),
            block_rows: word(1),
            block_elements: word(2),
        }
    }
}

impl DataType for InstanceRow {
    type ConfigurationOptions = ();

    const NAME: &'static str = "instance_row";
    const WGSL: &'static str = "InstanceRow";
    const SCALAR: Scalar = Scalar::F32;
    const LAYOUT: Layout = Layout::Struct(&[
        Field::nested("transform", Similarity::INFO),
        Field::new("group", FieldType::U32),
        Field::new("first", FieldType::U32),
    ]);
}

/// The value an input has while nothing is connected to it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PortDefault {
    /// No elements at all.
    Empty,
    F32(&'static [f32]),
    U32(&'static [u32]),
}

impl PortDefault {
    pub fn words(&self) -> &'static [u32] {
        match self {
            Self::Empty => &[],
            Self::F32(values) => bytemuck::cast_slice(values),
            Self::U32(values) => values,
        }
    }
}
