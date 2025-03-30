use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::{
    asset::instance::AssetInstance,
    data_types::{DataTypeInstance, DataTypeType, Property, PropertyMetadata},
    mesh::OneiroiMesh,
    operations::{Operation, PropertyInterface, StaticNodeMetadata},
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
    fn try_set_property(&mut self, property: &str, value: DataTypeInstance) -> Result<(), ()> {
        match property {
            "size" => {
                self.size
                    .update(value.inner_vec3().expect("TODO error handling"));
                Ok(())
            }
            "origin" => {
                self.origin
                    .update(value.inner_vec3().expect("TODO error handling"));
                Ok(())
            }

            _ => {
                println!("called set_prop with {:?}", property);
                Err(())
            }
        }
    }
    fn try_get_property(&self, property: &str) -> Result<DataTypeInstance, ()> {
        match property {
            "size" => Ok(self.size.get_instance()),
            "origin" => Ok(self.origin.get_instance()),
            _ => {
                //println!("called get_prop with {:?}", property);
                Err(())
            }
        }
    }

    fn get_properties(&self) -> Vec<PropertyMetadata> {
        println!("Get property called on Box");

        //TODO this can be most likely optimized
        let default = Self::default();

        let mut info = self.size.get_property_meta();
        info.set_name("size");
        info.set_default(default.size.get_instance());
        let mut info2 = self.size.get_property_meta();
        info2.set_name("origin");
        info.set_default(default.origin.get_instance());
        vec![info, info2]
    }
}

impl Operation for BoxV1 {
    fn compute(&self, _: Vec<&DataTypeInstance>) -> Vec<DataTypeInstance> {
        let mut surface = OneiroiMesh::default();

        let size = self.size.get_value() * 0.5;
        let origin = *self.origin.get_value();

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

        surface.add_tri_with_points([ftr, ftl, fbr]);
        surface.add_tri_with_points([ftl, fbl, fbr]);
        surface.add_tri_with_points([ftr, fbr, btr]);
        surface.add_tri_with_points([btr, fbr, bbr]);
        surface.add_tri_with_points([btr, bbr, bbl]);
        surface.add_tri_with_points([bbl, btl, btr]);
        surface.add_tri_with_points([btl, bbl, ftl]);
        surface.add_tri_with_points([fbl, ftl, bbl]);
        surface.add_tri_with_points([btr, ftl, ftr]);
        surface.add_tri_with_points([btr, btl, ftl]);
        surface.add_tri_with_points([fbl, bbl, fbr]);
        surface.add_tri_with_points([bbl, bbr, fbr]);

        vec![DataTypeInstance::Mesh(Property::new(surface))]
    }

    /* fn get_sockets(
        &self,
    ) -> (
        Vec<crate::data_types::DataTypeType>,
        Vec<crate::data_types::DataTypeType>,
    ) {
        (Vec::new(), vec![DataTypeType::Mesh])
    } */
    fn static_metadata(&self) -> StaticNodeMetadata {
        StaticNodeMetadata { color: "#15803d" }
    }

    fn get_input_sockets(&self) -> Vec<DataTypeType> {
        vec![]
    }

    fn get_output_sockets(&self) -> Vec<DataTypeType> {
        vec![DataTypeType::Mesh]
    }
}
