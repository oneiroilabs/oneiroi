use serde::{Deserialize, Serialize};

use crate::type_system::{
    data_types::Transform,
    variants::{TypeRef, OwnedDataType},
    reference::Reference,
};

use super::{DataType, DataTypeKind};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Instance {
    #[serde(skip)]
    reference: Reference,
    transform: Transform,
    //maybe only shader but could also be just attributes
    //param_override: HashMap
}

impl Instance {
    pub fn new(transform: Transform, reference: Reference) -> Self {
        Self {
            reference,
            transform, //transform: Default::default(),
        }
    }

    pub fn get_transform(&self) -> Transform {
        self.transform
    }

    /* pub fn get_position(&self) -> Vec3 {
        self.transform.translation.into()
    } */

    pub fn get_reference(&self) -> Reference {
        self.reference
    }

    /* pub fn set_unique(&mut self, reference: Reference) {
        self.reference = reference
    } */
}

impl DataType for Instance {
    const DATA_TYPE_TYPE: DataTypeKind = DataTypeKind::Instance;

    fn intrinsic_attributes() -> Option<Box<[super::ArributeMetadata]>> {
        todo!()
    }

    type ConfigurationOptions = ();

    fn get_type_ref(value: TypeRef) -> &Self {
        match value {
            TypeRef::Instance(val) => &val,
            _ => unreachable!(),
        }
    }

    fn get_type(value: OwnedDataType) -> Self {
        match value {
            OwnedDataType::Instance(val) => *val,
            _ => unreachable!(),
        }
    }

    fn to_data_type_value(&self) -> OwnedDataType {
        OwnedDataType::Instance(Box::new(self.clone()))
    }

    fn to_data_type_ref(&self) -> TypeRef {
        TypeRef::Instance(self)
    }
}
