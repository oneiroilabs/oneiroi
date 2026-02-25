use std::fmt::Debug;

//TODO maybe there is a better way
pub use glam::Vec3A;

use serde::{Deserialize, Serialize};

mod collection;
mod collider;
mod curve;
mod instance;
mod material;
mod mesh;
mod outline;
mod primitives;
mod selection;
mod texture;

pub use collection::Collection;
pub use collider::Collider;
pub use curve::CubicBezier;
pub use curve::Curve;
pub use instance::Instance;
pub use material::Material;
pub use mesh::{IndexedMeshBuffers, Mesh};
pub use outline::Outline;
pub use primitives::{Color, Float, Int, Transform, Vec3};
pub use selection::Selection;
pub use texture::Texture;

use crate::type_system::variants::{OwnedDataType, TypeRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataTypeKind {
    Omni,

    Selection,

    //Processable
    Mesh,
    Collider,

    Curve,
    CubicBezier,

    Instance,

    Texture,
    Material,

    //Maybe Processable
    Collection,

    //Primitives
    Vec3,
    Bool,
    Int,
    Float,
    Color,
    Transform,

    Outline,
}

//This is done to handle the collections since they are relying on a DataTypeType
//Omni gets abused as Uninitialized here
impl Default for DataTypeKind {
    fn default() -> Self {
        Self::Omni
    }
}

impl DataTypeKind {
    /* pub fn get_color(&self) -> Vec3 {
        match self {
            DataTypeType::Omni => Vec3::new(134., 25., 143.).map(|f| f / 256.),
            DataTypeType::Mesh => Vec3::new(59.0, 230.0, 121.0).map(|f| f / 256.),
            DataTypeType::Collection => Vec3::new(125.0, 80.0, 70.0).map(|f| f / 256.),
            DataTypeType::Instance => Vec3::new(0.0, 0.0, 55.0).map(|f| f / 256.),
            DataTypeType::Curve => todo!(),
            DataTypeType::Collider => todo!(),
            DataTypeType::Vec3 => todo!(),
            DataTypeType::Int => todo!(),
            DataTypeType::Float => todo!(),
            DataTypeType::Bool => todo!(),
            DataTypeType::Selection => todo!(),
            DataTypeType::Material => todo!(),
            DataTypeType::CubicBezier => todo!(),
            DataTypeType::Transform => todo!(),
            DataTypeType::Color => todo!(),
            DataTypeType::Polygon2d => todo!(),
            DataTypeType::Texture => todo!(),
        }
    } */

    pub fn is_processable(&self) -> bool {
        //TODO add all processable DataTypes
        matches!(self, DataTypeKind::Mesh | DataTypeKind::Instance)
    }
}

pub(crate) struct ArributeMetadata {
    name: String,
    r#type: DataTypeKind,
}

/// The main way to declare a DataType.
/// Can afterwards be used as a Property<T>.
pub(crate) trait DataType: Clone + Default {
    //fn get_value_string() -> String;
    //type ParsingType: DataType;

    /// Which identifier it has in the Type System.
    const DATA_TYPE_TYPE: DataTypeKind;

    /// Config and Restrictions of the Data Type for Properties.
    type ConfigurationOptions: Debug + Clone;

    fn generate_script(&self) -> String {
        unimplemented!()
    }

    //This could be constant but rust trolls idk if temporary or technical limitation
    fn intrinsic_attributes() -> Option<Box<[ArributeMetadata]>>;

    fn get_type_ref(value: TypeRef) -> &Self;

    fn get_type(value: OwnedDataType) -> Self;

    fn to_data_type_value(&self) -> OwnedDataType;

    fn to_data_type_ref(&self) -> TypeRef;

    // Retrieves all the References necessary to properly
    // compute the DataType.
    // If a DataType cant store any References this function returns None.
    /* fn get_references(&self) -> Option<Box<[Reference]>> {
        None
    } */

    //TODO need to implement defualt string representation
}

#[derive(Debug, Clone)]
pub(crate) enum DataTypeConfiguration {
    Int(),
    Float(),
}

//TODO evalutate is the fields should be private and which functions should be there
#[derive(Clone, Debug)]
pub struct TypeDescriptor {
    pub r#type: DataTypeKind,
    pub mutable: bool,
}

impl TypeDescriptor {
    pub fn is_processable(&self) -> bool {
        self.r#type.is_processable()
    }

    pub fn get_type(&self) -> DataTypeKind {
        self.r#type
    }
}

/* impl TypeDescriptor {
    pub(crate) fn new(r#type: DataTypeType, mutable: bool) -> Self {
        Self { r#type, mutable }
    }
} */
