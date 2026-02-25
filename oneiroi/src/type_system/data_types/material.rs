use serde::{Deserialize, Serialize};

use crate::{
    type_system::data_types::{Color, DataType, DataTypeKind},
    type_system::variants::{TypeRef, OwnedDataType},
};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Material {
    albedo: Color,
}

impl Material {
    pub fn get_albedo(&self) -> Color {
        println!("{self:?}");
        self.albedo
    }
}

impl DataType for Material {
    const DATA_TYPE_TYPE: super::DataTypeKind = DataTypeKind::Material;

    fn intrinsic_attributes() -> Option<Box<[super::ArributeMetadata]>> {
        None
    }

    type ConfigurationOptions = ();

    fn get_type_ref(value: TypeRef) -> &Self {
        match value {
            TypeRef::Material(value) => value,
            _ => unreachable!(),
        }
    }

    fn get_type(value: OwnedDataType) -> Self {
        match value {
            OwnedDataType::Material(value) => *value,
            _ => unreachable!(),
        }
    }

    fn to_data_type_value(&self) -> OwnedDataType {
        OwnedDataType::Material(Box::new(self.clone()))
    }

    fn to_data_type_ref(&self) -> TypeRef {
        TypeRef::Material(self)
    }
}
