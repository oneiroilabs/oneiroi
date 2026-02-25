use std::{collections::HashSet, fmt::Debug};

use serde::{Deserialize, Serialize};

use crate::{
    nodes::{
        ContextProvider, Node, PropertyInterface, PropertyNotFound, SetPropertyError,
        SocketInterface, StaticNodeMetadata,
    },
    property::{Property, PropertyMetadata},
    type_system::{
        OwnedDataType, Reference, TypeRef,
        data_types::{DataType, DataTypeKind, Int, Mesh, Selection, TypeDescriptor},
        trait_types::MeshMut2D,
    },
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BevelV1 {
    query: Property<Selection>,
    segments: Property<Int>,
}
impl Default for BevelV1 {
    fn default() -> Self {
        Self {
            query: Property::new(Selection::new("1")),
            segments: Property::new(1),
        }
    }
}

impl Node for BevelV1 {
    fn compute(
        &self,
        input_sockets: Option<&[Reference]>,
        context: &impl ContextProvider,
        //TODO explore this
        // write_back: [&mut DataTypeValue],
    ) -> Box<[Option<OwnedDataType>]> {
        let input = context.get_reference(input_sockets.unwrap()[0]);
        let input: &Mesh = input.dispatch_ref().unwrap();
        let mut new_surface = input.clone();
        //let selection: Vec<FaceHandle> = self.query.get_value(context).try_get().unwrap();
        //TODO

        new_surface.bevel_edges();

        Box::new([Some(OwnedDataType::Mesh(Box::new(new_surface)))])
    }

    fn node_metadata(&self) -> StaticNodeMetadata {
        StaticNodeMetadata { color: "#4338ca" }
    }
}
impl SocketInterface for BevelV1 {
    fn get_input_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::new([TypeDescriptor {
            r#type: DataTypeKind::Mesh,
            mutable: true,
        }])
    }

    fn get_output_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::new([TypeDescriptor {
            r#type: DataTypeKind::Mesh,
            mutable: true,
        }])
    }
}

impl PropertyInterface for BevelV1 {
    fn try_set_property(
        &mut self,
        property: &str,
        value: OwnedDataType,
    ) -> Result<(), SetPropertyError> {
        match property {
            "query" => {
                self.query.set_value(value.dispatch().unwrap());
                Ok(())
            }
            "segments" => {
                self.segments
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
            "query" => Ok(self.query.get_literal_value().to_data_type_ref()),
            "segments" => Ok(self.segments.get_literal_value().to_data_type_ref()),
            //"origin" => Ok(self.origin.get_instance()),
            _ => {
                //println!("called get_prop with {:?}", property);
                Err(PropertyNotFound)
            }
        }
    }

    fn get_properties(&self) -> Box<[PropertyMetadata]> {
        let default = Self::default();

        let info = PropertyMetadata {
            name: "query".into(),
            r#type: default.query.get_type(),
            default: default.query.get_literal_value().to_data_type_value(),
            configuration: None,
            //configuration: default.size.get_configuration().to_owned(),
            documentation: "".into(),
        };
        let info2 = PropertyMetadata {
            name: "segments".into(),
            r#type: default.segments.get_type(),
            default: default.segments.get_literal_value().to_data_type_value(),
            configuration: None,
            //configuration: default.size.get_configuration().to_owned(),
            documentation: "".into(),
        };

        Box::new([info, info2])
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

    /* fn try_get_property_metadata(&self, property: &str) -> Result<PropertyMetadata, ()> {
        todo!()
    } */
}
