use glam::{Vec3, Vec4, Vec4Swizzles};
use serde::{Deserialize, Serialize};

use crate::{
    nodes::{
        ContextProvider, Node, PropertyInterface, PropertyNotFound, SetPropertyError,
        SocketInterface, SocketMetadata, StaticNodeMetadata,
    },
    property::{Property, PropertyMetadata},
    type_system::{
        OwnedDataType, Reference, TypeRef,
        data_types::{CubicBezier, DataTypeKind, Mesh, Outline, TypeDescriptor},
        trait_types::{MeshMut0D, MeshMut2D, SequentialSample},
    },
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SweepV1 {
    //Cap_start maybe seal?
    //Cap_end
}
impl Default for SweepV1 {
    fn default() -> Self {
        Self {}
    }
}

impl PropertyInterface for SweepV1 {
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
            "number" => self.try_get_property_index(0),
            "distance" => self.try_get_property_index(1),
            "equidistant" => self.try_get_property_index(2),
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

        /* let mut info = self.number.get_property_meta();
        info.set_name("number");
        info.set_default(default.number.get_literal_value().to_data_type_value());
        //const WHAT: Box<[PropertyMetadata]> = Box::new([info]);

        let mut info2 = self.distance.get_property_meta();
        info2.set_name("distance");
        info2.set_default(default.distance.get_literal_value().to_data_type_value());

        let mut info3 = self.equidistant.get_property_meta();
        info3.set_name("equidistant");
        info3.set_default(default.equidistant.get_literal_value().to_data_type_value()); */

        /* vec![info, info2, info3].into_boxed_slice() */
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
        match index {
            /* 0 => Ok(self.number.get_literal_value().to_data_type_ref()),
            1 => Ok(self.distance.get_literal_value().to_data_type_ref()),
            2 => Ok(self.equidistant.get_literal_value().to_data_type_ref()), */
            _ => {
                //println!("called get_prop with {:?}", property);
                Err(PropertyNotFound)
            }
        }
    }

    fn try_get_property_metadata(&self, index: u8) -> PropertyMetadata {
        self.get_properties()[index as usize].clone()
    }

    fn set_property_external(
        &mut self,
        index: u8,
        reference: Reference,
    ) -> Result<(), SetPropertyError> {
        match index {
            /* 0 => {
                self.number.set_external(reference);
                Ok(())
            }
            1 => {
                self.distance.set_external(reference);
                Ok(())
            }
            2 => Ok(self.equidistant.set_external(reference)), */
            _ => Err(SetPropertyError::WrongIndex),
        }
    }

    /* fn try_get_property_script(&self, property: &str) -> Result<String, PropertyNotFound> {
        let string = match property {
            "number" => match &self.number {
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

impl Node for SweepV1 {
    fn compute(
        &self,
        input_sockets: Option<&[Reference]>,
        context: &impl ContextProvider,
    ) -> Box<[Option<OwnedDataType>]> {
        let curve: &CubicBezier = context
            .get_reference(input_sockets.unwrap()[0])
            .dispatch_ref()
            .unwrap();
        let polygon: &Outline = context
            .get_reference(input_sockets.unwrap()[1])
            .dispatch_ref()
            .unwrap();
        let mut new_mesh = Mesh::default();

        // Input Polygon point count
        // Get Curve Transforms for now equidistant later on smart sweep with curvature constraint
        let transforms = curve.sample_at_fixed_distance(0.2);
        let poly_points = polygon
            .iterate()
            .map(|i| Vec3::new(i.x, i.y, 0.0))
            .collect::<Box<_>>();
        // Register Initial Ring at 0 curve position
        let mut inserted_points =
            new_mesh.add_points(polygon.iterate().map(|i| Vec3::new(i.x, i.y, 0.0)));
        /* let mut current_edges =
        new_mesh.add_edge_strip(inserted_points.clone(), polygon.is_closed()); */

        let len = transforms.len();

        //For each transform extrude the prev ring to the new transform
        for (index, transform) in transforms.into_iter().enumerate() {
            for (index, point) in inserted_points.iter().copied().enumerate() {
                /* println!(
                    "{}, Point {point:?}",
                    transform.transform_point3(poly_points[index])
                ); */
                new_mesh.set_position(point, transform.transform_point3(poly_points[index]))
            }
            if index == len - 1 {
                break;
            }
            let new_points = new_mesh.extrude_edge_strip_connectivity(inserted_points);
            //current_edges = new_edges;
            inserted_points = new_points;

            //current_edges =
        }

        //println!("{new_mesh:?}");

        // Potentialle take into account hard/smooth vertices.

        //Somehow get last curve position and seal the sweep

        Box::new([Some(OwnedDataType::new(new_mesh))])
    }

    fn node_metadata(&self) -> StaticNodeMetadata {
        StaticNodeMetadata { color: "#15803d" }
    }
}
impl SocketInterface for SweepV1 {
    fn get_input_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::new([
            TypeDescriptor {
                r#type: DataTypeKind::CubicBezier,
                mutable: false,
            },
            TypeDescriptor {
                r#type: DataTypeKind::Outline,
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
}
