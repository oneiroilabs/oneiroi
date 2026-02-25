use serde::{Deserialize, Serialize};

use crate::{
    nodes::{
        ContextProvider, Node, PropertyInterface, PropertyNotFound, SetPropertyError,
        SocketInterface, StaticNodeMetadata,
    },
    property::PropertyMetadata,
    type_system::{
        OwnedDataType, Reference, TypeRef,
        data_types::{DataTypeKind, Outline, TypeDescriptor},
    },
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PolygonV1 {}
impl Default for PolygonV1 {
    fn default() -> Self {
        Self {}
    }
}

impl PropertyInterface for PolygonV1 {
    fn try_set_property(
        &mut self,
        property: &str,
        value: OwnedDataType,
    ) -> Result<(), SetPropertyError> {
        match property {
            /* "size" => {
                self.size
                    .set_value(value.dispatch().expect("TODO error handling"));
                Ok(())
            }
            "origin" => {
                self.origin
                    .set_value(value.dispatch().expect("TODO error handling"));
                Ok(())
            } */
            _ => {
                println!("called set_prop with {:?}", property);
                Err(SetPropertyError::NotFound)
            }
        }
    }
    fn try_get_property(&self, property: &str) -> Result<TypeRef, PropertyNotFound> {
        match property {
            /* "size" => Ok(self.size.get_literal_value().to_data_type_ref()),
            "origin" => Ok(self.origin.get_literal_value().to_data_type_ref()), */
            _ => {
                //println!("called get_prop with {:?}", property);
                Err(PropertyNotFound)
            }
        }
    }

    fn get_properties(&self) -> Box<[PropertyMetadata]> {
        /* println!("Get property called on Box");

        //TODO this can be most likely optimized
        let default = Self::default();

        let mut info = self.size.get_property_meta();
        info.set_name("size");
        info.set_default(
            default
                .size
                .get_literal_value()
                .to_data_type_value()
                .clone(),
        );
        let mut info2 = self.size.get_property_meta();
        info2.set_name("origin");
        info.set_default(
            default
                .origin
                .get_literal_value()
                .to_data_type_value()
                .clone(),
        ); */
        //vec![info, info2].into_boxed_slice()
        Box::new([])
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
        match index {
            /*  0 => Ok(self.size.set_external(reference)),
            1 => Ok(self.origin.set_external(reference)), */
            _ => Err(SetPropertyError::WrongIndex),
        }
    }

    /* fn try_get_property_script(&self, property: &str) -> Result<String, PropertyNotFound> {
        let string = match property {
            "size" => match &self.size {
                Property::Script(oneiroi_script) => oneiroi_script.get_string(),
                Property::Value(v) => v.generate_script(),
            },
            "origin" => match &self.origin {
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

impl Node for PolygonV1 {
    fn compute(
        &self,
        input_sockets: Option<&[Reference]>,
        context: &impl ContextProvider,
        //TODO explore this
        // write_back: [&mut DataTypeValue],
    ) -> Box<[Option<OwnedDataType>]> {
        let mut poly = Outline::new();

        Box::new([Some(OwnedDataType::new(poly))])
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
impl SocketInterface for PolygonV1 {
    fn get_input_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::default()
    }

    fn get_output_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::new([TypeDescriptor {
            r#type: DataTypeKind::Outline,
            mutable: true,
        }])
    }
}
