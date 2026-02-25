use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::{
    nodes::{
        ContextProvider, Node, PropertyInterface, PropertyNotFound, SetPropertyError,
        SocketInterface, StaticNodeMetadata,
    },
    property::PropertyMetadata,
    type_system::{
        OwnedDataType, Reference, TypeRef,
        data_types::{DataTypeKind, Mesh, TypeDescriptor},
    },
};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SetMaterialV1 {
    //TODO eventually probably specify the surface?
}

impl Node for SetMaterialV1 {
    fn compute(
        &self,
        input_sockets: Option<&[Reference]>,
        context: &impl ContextProvider,
        //TODO explore this
        // write_back: [&mut DataTypeValue],
    ) -> Box<[Option<OwnedDataType>]> {
        let input_mesh = context.get_reference(input_sockets.unwrap()[0]);
        let input_mesh: &Mesh = input_mesh.dispatch_ref().unwrap();
        let mut output_mesh = input_mesh.clone();

        let material = input_sockets.unwrap()[1];
        output_mesh.set_material(material);

        Box::new([Some(OwnedDataType::new(output_mesh))])
    }

    fn node_metadata(&self) -> StaticNodeMetadata {
        StaticNodeMetadata { color: "#4338ca" }
    }
}
impl SocketInterface for SetMaterialV1 {
    fn get_input_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::new([
            TypeDescriptor {
                r#type: DataTypeKind::Mesh,
                mutable: true,
            },
            TypeDescriptor {
                r#type: DataTypeKind::Material,
                mutable: true,
            },
        ])
    }

    fn get_output_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::new([TypeDescriptor {
            r#type: DataTypeKind::Mesh,
            mutable: true,
        }])
    }
}

impl PropertyInterface for SetMaterialV1 {
    fn try_set_property(
        &mut self,
        property: &str,
        value: OwnedDataType,
    ) -> Result<(), SetPropertyError> {
        match property {
            _ => {
                println!("called set_prop with {:?}", property);
                Err(SetPropertyError::NotFound)
            }
        }
    }

    fn try_get_property(&self, property: &str) -> Result<TypeRef, PropertyNotFound> {
        match property {
            _ => {
                //println!("called get_prop with {:?}", property);
                Err(PropertyNotFound)
            }
        }
    }

    fn get_properties(&self) -> Box<[PropertyMetadata]> {
        /* let mut info2 = self.size.get_property_meta();
        info2.set_name("origin");
        info.set_default(default.merge_adjacent_normals.get_instance()); */
        vec![].into_boxed_slice()
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
}
