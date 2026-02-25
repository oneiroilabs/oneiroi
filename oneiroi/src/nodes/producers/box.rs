use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::{
    nodes::{
        ContextProvider, Node, PropertyInterface, PropertyNotFound, SetPropertyError,
        SocketInterface, SocketMetadata, StaticNodeMetadata,
    },
    property::{Property, PropertyMetadata},
    type_system::{
        OwnedDataType, Reference, TypeRef,
        data_types::{DataType, DataTypeKind, Mesh, TypeDescriptor},
        trait_types::{MeshMut0D, MeshMut2D},
    },
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BoxV1 {
    size: Property<Vec3>,
    origin: Property<Vec3>,
    subdivisions: Property<Vec3>,
}
impl Default for BoxV1 {
    fn default() -> Self {
        Self {
            size: Property::new(Vec3::new(1.0, 1.0, 1.0)),
            origin: Property::new(Vec3::new(0.0, 0.0, 0.0)),
            subdivisions: Property::new(Vec3::new(1.0, 1.0, 1.0)),
        }
    }
}

impl PropertyInterface for BoxV1 {
    fn try_set_property(
        &mut self,
        property: &str,
        value: OwnedDataType,
    ) -> Result<(), SetPropertyError> {
        match property {
            "size" => {
                self.size
                    .set_value(value.dispatch().expect("TODO error handling"));
                Ok(())
            }
            "origin" => {
                self.origin
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
            "size" => Ok(self.size.get_literal_value().to_data_type_ref()),
            "origin" => Ok(self.origin.get_literal_value().to_data_type_ref()),
            _ => {
                //println!("called get_prop with {:?}", property);
                Err(PropertyNotFound)
            }
        }
    }

    fn get_properties(&self) -> Box<[PropertyMetadata]> {
        println!("Get property called on Box");

        let default = Self::default();

        let info = PropertyMetadata {
            name: "size".into(),
            r#type: default.size.get_type(),
            default: default.size.get_literal_value().to_data_type_value(),
            configuration: None,
            //configuration: default.size.get_configuration().to_owned(),
            documentation: "".into(),
        };

        let info2 = PropertyMetadata {
            name: "origin".into(),
            r#type: default.origin.get_type(),
            default: default.origin.get_literal_value().to_data_type_value(),
            configuration: None,
            //configuration: default.size.get_configuration().to_owned(),
            documentation: "".into(),
        };

        vec![info, info2].into_boxed_slice()
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
            0 => Ok(self.size.set_external(reference)),
            1 => Ok(self.origin.set_external(reference)),
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

impl Node for BoxV1 {
    fn compute(
        &self,
        input_sockets: Option<&[Reference]>,
        context: &impl ContextProvider,
        //TODO explore this
        // write_back: [&mut DataTypeValue],
    ) -> Box<[Option<OwnedDataType>]> {
        let mut surface = Mesh::default();

        let size = self.size.get_value(context) * 0.5;
        let origin = *self.origin.get_value(context);

        //TODO subdiv
        //let mut points = Vec::with_capacity(8);

        let ftl = surface.add_point(Vec3::new(size.x, size.y, size.z));
        let ftr = surface.add_point(Vec3::new(size.x, size.y, -size.z));
        let fbl = surface.add_point(Vec3::new(size.x, -size.y, size.z));
        let fbr = surface.add_point(Vec3::new(size.x, -size.y, -size.z));
        let btl = surface.add_point(Vec3::new(-size.x, size.y, size.z));
        let bbl = surface.add_point(Vec3::new(-size.x, -size.y, size.z));
        let bbr = surface.add_point(Vec3::new(-size.x, -size.y, -size.z));
        let btr = surface.add_point(Vec3::new(-size.x, size.y, -size.z));

        surface.add_tri([ftr, ftl, fbr]);
        surface.add_tri([ftl, fbl, fbr]);
        surface.add_tri([ftr, fbr, btr]);
        surface.add_tri([btr, fbr, bbr]);
        surface.add_tri([btr, bbr, bbl]);
        surface.add_tri([bbl, btl, btr]);
        surface.add_tri([btl, bbl, ftl]);
        surface.add_tri([fbl, ftl, bbl]);
        surface.add_tri([btr, ftl, ftr]);
        surface.add_tri([btr, btl, ftl]);
        surface.add_tri([fbl, bbl, fbr]);
        surface.add_tri([bbl, bbr, fbr]);

        //surface.set_all_edges_hard();

        Box::new([Some(OwnedDataType::Mesh(Box::new(surface)))])
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
impl SocketInterface for BoxV1 {
    fn get_input_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::default()
    }

    fn get_output_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::new([TypeDescriptor {
            r#type: DataTypeKind::Mesh,
            mutable: true,
        }])
    }
}
