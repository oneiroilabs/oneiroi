use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::{
    nodes::{
        ContextProvider, Node, PropertyInterface, PropertyNotFound, SetPropertyError,
        SocketInterface, StaticNodeMetadata,
    },
    property::{Property, PropertyMetadata},
    type_system::{
        OwnedDataType, Reference, TypeRef,
        data_types::{Collection, DataType, DataTypeKind, TypeDescriptor},
    },
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DistributePointsV1 {
    amount: Property<i64>,
}

impl Default for DistributePointsV1 {
    fn default() -> Self {
        Self {
            amount: Property::new(5),
        }
    }
}

impl PropertyInterface for DistributePointsV1 {
    fn try_set_property(
        &mut self,
        property: &str,
        value: OwnedDataType,
    ) -> Result<(), SetPropertyError> {
        match property {
            "amount" => {
                self.amount
                    .set_value(value.dispatch().expect("TODO error handling"));
                Ok(())
            }

            _ => {
                println!("called set_prop with {:?}", property);
                Err(SetPropertyError::NotFound)
            }
        }
    }
    fn try_get_property(&self, property: &str) -> Result<TypeRef, PropertyNotFound> {
        match property {
            "amount" => Ok(self.amount.get_literal_value().to_data_type_ref()),

            _ => {
                //println!("called get_prop with {:?}", property);
                Err(PropertyNotFound)
            }
        }
    }

    fn get_properties(&self) -> Box<[PropertyMetadata]> {
        println!("Get property called on Box");

        //TODO this can be most likely optimized
        let default = Self::default();

        let info = PropertyMetadata {
            name: "amount".into(),
            r#type: default.amount.get_type(),
            default: default.amount.get_literal_value().to_data_type_value(),
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
        let string = match property {
            "amount" => match &self.amount {
                Property::Script(oneiroi_script) => oneiroi_script.get_string(),
                Property::Value(v) => v.generate_script(),
            },

            _ => return Err(PropertyNotFound),
        };
        Ok(string)
    } */

    /* fn try_get_property_metadata(&self, property: &str) -> Result<PropertyMetadata, ()> {
        todo!()
    } */
}

impl Node for DistributePointsV1 {
    fn compute(
        &self,
        input_sockets: Option<&[Reference]>,
        context: &impl ContextProvider,
        //TODO explore this
        // write_back: [&mut DataTypeValue],
    ) -> Box<[Option<OwnedDataType>]> {
        let mut points = Collection::new(DataTypeKind::Vec3);

        points.push(OwnedDataType::Vec3(Vec3::new(0.0, 0.0, 0.0)));
        points.push(OwnedDataType::Vec3(Vec3::new(3.0, 0.0, 0.0)));
        points.push(OwnedDataType::Vec3(Vec3::new(1.0, 1.0, 1.0)));

        Box::new([Some(OwnedDataType::new(points))])
    }

    /* fn get_sockets(
        &self,
    ) -> (
        Vec<crate::data_types::DataTypeType>,
        Vec<crate::data_types::DataTypeType>,
    ) {
        (Vec::new(), vec![DataTypeType::Mesh])
    } */
    fn node_metadata(&self) -> StaticNodeMetadata {
        StaticNodeMetadata { color: "#15803d" }
    }
}
impl SocketInterface for DistributePointsV1 {
    fn get_input_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::default()
    }

    fn get_output_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::new([TypeDescriptor {
            r#type: DataTypeKind::Collection,
            mutable: true,
        }])
    }
}
