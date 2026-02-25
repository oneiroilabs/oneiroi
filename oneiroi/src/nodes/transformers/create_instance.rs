use glam::{Affine3A, Vec3};
use serde::{Deserialize, Serialize};

use crate::{
    nodes::{
        ContextProvider, Node, PropertyInterface, PropertyNotFound, SetPropertyError,
        SocketInterface, StaticNodeMetadata,
    },
    property::{Property, PropertyMetadata},
    type_system::{
        OwnedDataType, Reference, TypeRef,
        data_types::{DataType, DataTypeKind, Instance, TypeDescriptor},
    },
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateInstanceV1 {
    position: Property<Vec3>,
}

impl Default for CreateInstanceV1 {
    fn default() -> Self {
        Self {
            position: Property::new(Vec3::new(1.0, 1.0, 1.0)),
        }
    }
}

impl PropertyInterface for CreateInstanceV1 {
    fn try_set_property(
        &mut self,
        property_name: &str,
        value: OwnedDataType,
    ) -> Result<(), SetPropertyError> {
        match property_name {
            "position" => {
                self.position.set_value(value.dispatch().unwrap());
                Ok(())
            }
            _ => Err(SetPropertyError::NotFound),
        }
    }

    fn try_get_property(&self, property: &str) -> Result<TypeRef, PropertyNotFound> {
        match property {
            "position" => Ok(self.position.get_literal_value().to_data_type_ref()),
            _ => Err(PropertyNotFound),
        }
    }

    fn get_properties(&self) -> Box<[PropertyMetadata]> {
        let default = Self::default();

        let info = PropertyMetadata {
            name: "position".into(),
            r#type: default.position.get_type(),
            default: default.position.get_literal_value().to_data_type_value(),
            configuration: None,
            //configuration: default.size.get_configuration().to_owned(),
            documentation: "".into(),
        };
        Box::new([info])
    }

    fn try_set_property_index(
        &mut self,
        index: u8,
        value: OwnedDataType,
    ) -> Result<(), SetPropertyError> {
        todo!()
    }

    fn try_get_property_index(&self, index: u8) -> Result<TypeRef, PropertyNotFound> {
        todo!()
    }

    fn set_property_external(
        &mut self,
        index: u8,
        reference: Reference,
    ) -> Result<(), SetPropertyError> {
        todo!()
    }

    /* fn try_get_property_script(&self, property: &str) -> Result<String, PropertyNotFound> {
        todo!()
    } */
}

impl Node for CreateInstanceV1 {
    fn compute(
        &self,
        input_sockets: Option<&[Reference]>,
        context: &impl ContextProvider,
        //TODO explore this
        // write_back: [&mut DataTypeValue],
    ) -> Box<[Option<OwnedDataType>]> {
        //let input = input_sockets.unwrap()[0];
        let position = *self.position.get_value(context);
        let new_instance = Instance::new(
            Affine3A::from_translation(position),
            input_sockets.unwrap()[0],
        );
        Box::new([Some(OwnedDataType::new(new_instance))])
    }

    fn node_metadata(&self) -> crate::nodes::StaticNodeMetadata {
        StaticNodeMetadata { color: "#4338ca" }
    }
}
impl SocketInterface for CreateInstanceV1 {
    fn get_input_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::new([TypeDescriptor {
            r#type: DataTypeKind::Omni,
            mutable: false,
        }])
    }

    fn get_output_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::new([TypeDescriptor {
            r#type: DataTypeKind::Instance,
            mutable: true,
        }])
    }
}
