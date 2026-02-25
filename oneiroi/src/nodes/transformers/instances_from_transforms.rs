use serde::{Deserialize, Serialize};

use crate::{
    nodes::{
        ContextProvider, Node, PropertyInterface, PropertyNotFound, SetPropertyError,
        SocketInterface, StaticNodeMetadata,
    },
    property::{Property, PropertyMetadata},
    type_system::{
        OwnedDataType, Reference, TypeRef,
        data_types::{Collection, DataType, DataTypeKind, Instance, Transform, TypeDescriptor},
    },
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstancesFromTransformsV1 {
    transform: Property<Transform>,
}

impl Default for InstancesFromTransformsV1 {
    fn default() -> Self {
        Self {
            transform: Property::new(Transform::IDENTITY),
        }
    }
}

impl PropertyInterface for InstancesFromTransformsV1 {
    fn try_set_property(
        &mut self,
        property: &str,
        value: OwnedDataType,
    ) -> Result<(), SetPropertyError> {
        match property {
            "transform" => {
                self.transform
                    .set_value(value.dispatch().expect("TODO error handling"));
                Ok(())
            }

            _ => {
                println!("called set_prop with {property:?}");

                Err(SetPropertyError::NotFound)
            }
        }
    }
    fn try_get_property(&self, property: &str) -> Result<TypeRef, PropertyNotFound> {
        match property {
            "transform" => Ok(self.transform.get_literal_value().to_data_type_ref()),

            _ => {
                //println!("called get_prop with {:?}", property);
                Err(PropertyNotFound)
            }
        }
    }

    fn get_properties(&self) -> Box<[PropertyMetadata]> {
        //TODO this can be most likely optimized
        let default = Self::default();

        let mut info = PropertyMetadata {
            name: "transform".into(),
            r#type: default.transform.get_type(),
            default: default.transform.get_literal_value().to_data_type_value(),
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
        /* let string = match property {
            "amount" => match &self.amount {
                Property::Script(oneiroi_script) => oneiroi_script.get_string(),
                Property::Value(v) => v.generate_script(),
            },

            _ => return Err(PropertyNotFound),
        };
        Ok(string) */
        Err(PropertyNotFound)
    } */

    /* fn try_get_property_metadata(&self, property: &str) -> Result<PropertyMetadata, ()> {
        todo!()
    } */
}

impl Node for InstancesFromTransformsV1 {
    fn compute(
        &self,
        input_sockets: Option<&[Reference]>,
        context: &impl ContextProvider,
        //TODO explore this
        // write_back: [&mut DataTypeValue],
    ) -> Box<[Option<OwnedDataType>]> {
        let inputs = input_sockets.unwrap();
        //TODO get points
        let points = context.get_reference(inputs[0]);
        let transforms: &Collection = points.dispatch_ref().unwrap();
        //let points = vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)];
        let mut instances = Collection::new(DataTypeKind::Instance);

        let local_tf = self.transform.get_value(context);

        for transform in transforms.iterate() {
            let transform: Transform = transform.clone().dispatch().unwrap();
            instances.push(OwnedDataType::Instance(Box::new(Instance::new(
                transform * *local_tf,
                inputs[1],
            ))));
            println!("Instance TF: {transform}");
        }
        Box::new([Some(OwnedDataType::new(instances))])
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
impl SocketInterface for InstancesFromTransformsV1 {
    fn get_input_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::new([
            TypeDescriptor {
                r#type: DataTypeKind::Collection,
                mutable: false,
            },
            TypeDescriptor {
                r#type: DataTypeKind::Mesh,
                mutable: false,
            },
        ])
    }

    fn get_output_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::new([TypeDescriptor {
            r#type: DataTypeKind::Collection,
            mutable: true,
        }])
    }
    /* fn get_input_sockets(&self) -> Vec<DataTypeType> {
        vec![DataTypeType::Collection, DataTypeType::Mesh]
    }

    fn get_output_sockets(&self) -> Vec<DataTypeType> {
        vec![DataTypeType::Collection]
    } */
}

/* impl TestingNodeExt for InstancesFromPointsV1 {
    fn compute_new(
        &self,
        inputs: &[crate::data_types::TypeDescriptor],
        context: &impl crate::operations::Handler,
    ) -> Box<[DataTypeValue]> {
        //context.get_unqiue_index(index);
        Box::new([DataTypeValue::Int(11)])
    }
} */
