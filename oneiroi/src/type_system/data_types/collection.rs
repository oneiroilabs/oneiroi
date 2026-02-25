use serde::{Deserialize, Serialize};

use crate::type_system::variants::{OwnedDataType, TypeRef};

use super::{DataType, DataTypeKind};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Collection {
    r#type: DataTypeKind,
    data: Vec<OwnedDataType>,
}

impl Collection {
    pub fn new(r#type: DataTypeKind) -> Self {
        Self {
            r#type,
            data: Vec::new(),
        }
    }

    pub(crate) fn get_type(&self) -> DataTypeKind {
        self.r#type
    }

    /* pub fn with_capacity(r#type: DataTypeType, capacity: usize) -> Self {
        Self {
            r#type,
            data: Vec::with_capacity(capacity),
        }
    } */

    //TODO
    pub(crate) fn push(&mut self, value: OwnedDataType) {
        if value.get_data_type() != self.r#type {
            panic!()
        }
        self.data.push(value);
    }

    pub fn iterate(&self) -> impl Iterator<Item = &OwnedDataType> {
        self.data.iter()
    }

    pub fn length(&self) -> usize {
        self.data.len()
    }
}

impl DataType for Collection {
    const DATA_TYPE_TYPE: DataTypeKind = DataTypeKind::Collection;

    fn intrinsic_attributes() -> Option<Box<[super::ArributeMetadata]>> {
        todo!()
    }

    fn get_type_ref(value: TypeRef) -> &Self {
        match value {
            TypeRef::Collection(val) => val,
            _ => unreachable!(),
        }
    }

    type ConfigurationOptions = ();

    fn get_type(value: OwnedDataType) -> Self {
        match value {
            OwnedDataType::Collection(val) => *val,
            _ => unreachable!(),
        }
    }

    fn to_data_type_value(&self) -> OwnedDataType {
        OwnedDataType::Collection(Box::new(self.clone()))
    }

    fn to_data_type_ref(&self) -> TypeRef {
        TypeRef::Collection(self)
    }
}
