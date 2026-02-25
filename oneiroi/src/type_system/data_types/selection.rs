use serde::{Deserialize, Serialize};

use crate::type_system::variants::{TypeRef, OwnedDataType};

use super::{ArributeMetadata, DataType, DataTypeKind, mesh::FaceHandle};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Selection {
    literal: String,
}

impl DataType for Selection {
    const DATA_TYPE_TYPE: DataTypeKind = DataTypeKind::Selection;

    fn intrinsic_attributes() -> Option<Box<[ArributeMetadata]>> {
        None
    }

    fn get_type(value: OwnedDataType) -> Self {
        todo!()
    }

    fn to_data_type_value(&self) -> OwnedDataType {
        todo!()
    }

    fn get_type_ref(value: TypeRef) -> &Self {
        todo!()
    }

    fn to_data_type_ref(&self) -> TypeRef {
        todo!()
    }

    type ConfigurationOptions = ();
}

impl Selection {
    pub fn new(selection: &str) -> Self {
        Selection {
            literal: selection.into(),
        }
    }

    pub fn get_literal(&self) -> &str {
        &self.literal
    }

    //TODO this should be a slice
    pub fn try_get(&self) -> Result<Vec<FaceHandle>, ()> {
        let parsed = self.literal.parse::<usize>();
        println!("{parsed:?}");
        Ok(vec![FaceHandle::new(parsed.unwrap())])
    }
}
