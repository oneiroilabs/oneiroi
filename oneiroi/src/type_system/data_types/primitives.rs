use color::{AlphaColor, DynamicColor, Srgb};
use glam::{Affine3A, Vec3 as MathVec3};
use serde::{Deserialize, Serialize};

use crate::type_system::{
    data_types::{ArributeMetadata, DataType, DataTypeKind},
    variants::{TypeRef, OwnedDataType},
};

pub type Vec3 = MathVec3;

impl DataType for Vec3 {
    fn get_type_ref(value: TypeRef) -> &Self {
        match value {
            TypeRef::Vec3(val) => val,
            _ => unreachable!(),
        }
    }

    type ConfigurationOptions = ();

    fn get_type(value: OwnedDataType) -> Self {
        match value {
            OwnedDataType::Vec3(val) => val,
            _ => unreachable!(),
        }
    }

    fn generate_script(&self) -> String {
        "Vec3(".to_owned()
            + &self.x.to_string()
            + ","
            + &self.y.to_string()
            + ","
            + &self.z.to_string()
            + ")"
    }

    const DATA_TYPE_TYPE: DataTypeKind = DataTypeKind::Vec3;

    fn intrinsic_attributes() -> Option<Box<[ArributeMetadata]>> {
        Some(
            vec![
                ArributeMetadata {
                    name: "x".to_string(),
                    r#type: DataTypeKind::Float,
                },
                ArributeMetadata {
                    name: "y".to_string(),
                    r#type: DataTypeKind::Float,
                },
                ArributeMetadata {
                    name: "z".to_string(),
                    r#type: DataTypeKind::Float,
                },
            ]
            .into_boxed_slice(),
        )
    }

    fn to_data_type_value(&self) -> OwnedDataType {
        OwnedDataType::Vec3(*self)
    }

    fn to_data_type_ref(&self) -> TypeRef {
        TypeRef::Vec3(self)
    }
}

pub type Float = f32;
impl DataType for Float {
    /*  fn get_type() -> PropertyType {
    PropertyType::Float
    } */
    fn get_type_ref(value: TypeRef) -> &Self {
        match value {
            TypeRef::Float(val) => val,
            _ => unreachable!(),
        }
    }

    type ConfigurationOptions = ();

    fn get_type(value: OwnedDataType) -> Self {
        match value {
            OwnedDataType::Float(val) => val,
            _ => unreachable!(),
        }
    }

    // type ParsingType = f32;
    fn generate_script(&self) -> String {
        self.to_string()
    }

    const DATA_TYPE_TYPE: DataTypeKind = DataTypeKind::Float;

    fn intrinsic_attributes() -> Option<Box<[ArributeMetadata]>> {
        None
    }

    fn to_data_type_value(&self) -> OwnedDataType {
        OwnedDataType::Float(*self)
    }

    fn to_data_type_ref(&self) -> TypeRef {
        TypeRef::Float(self)
    }

    /* fn to_data_type_value(&self) -> DataTypeRef {
    todo!()
    } */
}

impl DataType for bool {
    /*  fn get_type() -> PropertyType {
    PropertyType::Float
    } */
    fn get_type_ref(value: TypeRef) -> &Self {
        match value {
            TypeRef::Bool(val) => &val,
            _ => unreachable!(),
        }
    }

    fn get_type(value: OwnedDataType) -> Self {
        match value {
            OwnedDataType::Bool(val) => val,
            _ => unreachable!(),
        }
    }

    // type ParsingType = f32;
    fn generate_script(&self) -> String {
        self.to_string()
    }

    const DATA_TYPE_TYPE: DataTypeKind = DataTypeKind::Bool;

    fn intrinsic_attributes() -> Option<Box<[ArributeMetadata]>> {
        None
    }

    fn to_data_type_value(&self) -> OwnedDataType {
        OwnedDataType::Bool(*self)
    }

    fn to_data_type_ref(&self) -> TypeRef {
        TypeRef::Bool(self)
    }
    type ConfigurationOptions = ();
}

pub type Int = i64;
impl DataType for Int {
    fn generate_script(&self) -> String {
        self.to_string()
    }
    const DATA_TYPE_TYPE: DataTypeKind = DataTypeKind::Int;

    fn intrinsic_attributes() -> Option<Box<[ArributeMetadata]>> {
        None
    }

    fn get_type_ref(value: TypeRef) -> &Self {
        match value {
            TypeRef::Int(val) => val,
            _ => unreachable!(),
        }
    }

    fn get_type(value: OwnedDataType) -> Self {
        match value {
            OwnedDataType::Int(val) => val,
            _ => unreachable!(),
        }
    }
    type ConfigurationOptions = ();

    fn to_data_type_value(&self) -> OwnedDataType {
        OwnedDataType::Int(*self)
    }

    fn to_data_type_ref(&self) -> TypeRef {
        TypeRef::Int(self)
    }
}

pub type Transform = Affine3A;
impl DataType for Transform {
    const DATA_TYPE_TYPE: DataTypeKind = DataTypeKind::Transform;

    fn intrinsic_attributes() -> Option<Box<[ArributeMetadata]>> {
        None
    }

    fn get_type_ref(value: TypeRef) -> &Self {
        match value {
            TypeRef::Transform(val) => val,
            _ => unreachable!(),
        }
    }

    fn get_type(value: OwnedDataType) -> Self {
        match value {
            OwnedDataType::Transform(val) => *val,
            _ => unreachable!(),
        }
    }

    fn to_data_type_value(&self) -> OwnedDataType {
        OwnedDataType::Transform(Box::new(*self))
    }

    fn to_data_type_ref(&self) -> TypeRef {
        TypeRef::Transform(self)
    }

    type ConfigurationOptions = ();
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color(DynamicColor);

impl Default for Color {
    fn default() -> Self {
        Color(AlphaColor::from_rgb8(0, 0, 0).into())
    }
}

impl Color {
    pub fn to_rgba(&self) -> [f32; 4] {
        self.0.convert(color::ColorSpaceTag::Srgb).components
    }

    pub fn from_srgb(rgba: [f32; 4]) -> Self {
        Color(AlphaColor::<Srgb>::new(rgba).into())
    }
}

impl DataType for Color {
    const DATA_TYPE_TYPE: DataTypeKind = DataTypeKind::Color;

    fn intrinsic_attributes() -> Option<Box<[ArributeMetadata]>> {
        None
    }

    fn get_type_ref(value: TypeRef) -> &Self {
        match value {
            TypeRef::Color(value) => value,
            _ => unreachable!(),
        }
    }

    fn get_type(value: OwnedDataType) -> Self {
        match value {
            OwnedDataType::Color(value) => *value,
            _ => unreachable!(),
        }
    }

    fn to_data_type_value(&self) -> OwnedDataType {
        OwnedDataType::Color(Box::new(self.clone()))
    }

    fn to_data_type_ref(&self) -> TypeRef {
        TypeRef::Color(self)
    }
    type ConfigurationOptions = ();
}
