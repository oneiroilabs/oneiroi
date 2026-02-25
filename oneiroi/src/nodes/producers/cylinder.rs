use core::f32;
use std::f32::consts::TAU;

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
        data_types::{DataType, DataTypeKind, Mesh, TypeDescriptor},
        trait_types::MeshMut2D,
    },
};

//struct Test(Option<[NonZeroU32; 3]>);
//struct Test(Vec3);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CylinderV1 {
    radius: Property<f32>,
    height: Property<f32>,
    segments: Property<i64>,
    has_caps: Property<bool>,
}
impl Default for CylinderV1 {
    fn default() -> Self {
        Self {
            radius: Property::new(0.5),
            height: Property::new(2.0),
            segments: Property::new(6),
            has_caps: Property::new(false),
        }
    }
}

impl PropertyInterface for CylinderV1 {
    fn try_set_property(
        &mut self,
        property: &str,
        value: OwnedDataType,
    ) -> Result<(), SetPropertyError> {
        match property {
            "radius" => {
                self.radius
                    .set_value(value.dispatch().expect("TODO error handling"));
                Ok(())
            }
            "height" => {
                self.height
                    .set_value(value.dispatch().expect("TODO error handling"));
                Ok(())
            }
            "segments" => {
                self.segments
                    .set_value(value.dispatch().expect("TODO error handling"));
                Ok(())
            }
            "has_caps" => {
                self.has_caps
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
            "radius" => Ok(self.radius.get_literal_value().to_data_type_ref()),
            "height" => Ok(self.height.get_literal_value().to_data_type_ref()),
            "segments" => Ok(self.segments.get_literal_value().to_data_type_ref()),
            "has_caps" => Ok(self.has_caps.get_literal_value().to_data_type_ref()),

            _ => {
                //println!("called get_prop with {:?}", property);
                Err(PropertyNotFound)
            }
        }
    }

    fn get_properties(&self) -> Box<[PropertyMetadata]> {
        //TODO this can be most likely optimized
        let default = Self::default();

        let info = PropertyMetadata {
            name: "radius".into(),
            r#type: default.radius.get_type(),
            default: default.radius.get_literal_value().to_data_type_value(),
            configuration: None,
            //configuration: default.size.get_configuration().to_owned(),
            documentation: "".into(),
        };

        let info2 = PropertyMetadata {
            name: "height".into(),
            r#type: default.height.get_type(),
            default: default.height.get_literal_value().to_data_type_value(),
            configuration: None,
            //configuration: default.size.get_configuration().to_owned(),
            documentation: "".into(),
        };

        let info3 = PropertyMetadata {
            name: "segments".into(),
            r#type: default.segments.get_type(),
            default: default.segments.get_literal_value().to_data_type_value(),
            configuration: None,
            //configuration: default.size.get_configuration().to_owned(),
            documentation: "".into(),
        };

        let info4 = PropertyMetadata {
            name: "has_caps".into(),
            r#type: default.has_caps.get_type(),
            default: default.has_caps.get_literal_value().to_data_type_value(),
            configuration: None,
            //configuration: default.size.get_configuration().to_owned(),
            documentation: "".into(),
        };
        Box::new([info, info2, info3, info4])
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
            0 => Ok(self.radius.set_external(reference)),
            1 => Ok(self.height.set_external(reference)),
            2 => Ok(self.segments.set_external(reference)),
            3 => Ok(self.has_caps.set_external(reference)),
            _ => Err(SetPropertyError::WrongIndex),
        }
    }

    /* fn try_get_property_script(&self, property: &str) -> Result<String, PropertyNotFound> {
        let string = match property {
            "radius" => match &self.radius {
                Property::Script(oneiroi_script) => oneiroi_script.get_string(),
                Property::Value(v) => v.generate_script(),
            },
            "height" => match &self.height {
                Property::Script(oneiroi_script) => oneiroi_script.get_string(),
                Property::Value(v) => v.generate_script(),
            },
            "segments" => match &self.segments {
                Property::Script(oneiroi_script) => oneiroi_script.get_string(),
                Property::Value(v) => v.generate_script(),
            },
            "has_caps" => match &self.has_caps {
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

impl Node for CylinderV1 {
    fn compute(
        &self,
        input_sockets: Option<&[Reference]>,
        context: &impl ContextProvider,
        //TODO explore this
        // write_back: [&mut DataTypeValue],
    ) -> Box<[Option<OwnedDataType>]> {
        let mut surface = Mesh::default();
        let segments = *self.segments.get_value(context);
        let height = *self.height.get_value(context); // can also / 2 and add a origin point
        let radius = *self.radius.get_value(context);

        let angle = TAU / segments as f32;

        let mut points_vec: Vec<Vec3> = Vec::with_capacity((2 * segments) as usize);

        //calculate the position for all points
        for segement in 0..segments {
            let angle = angle * segement as f32;

            let x = radius * angle.cos();
            let z = -radius * angle.sin();
            points_vec.push(Vec3::new(x, height, z));
            points_vec.push(Vec3::new(x, 0., z));
        }
        debug_assert!(points_vec.len() == 2 * segments as usize);
        //conceptually the points are layed out like this
        // 0  2  4
        //
        // 1  3  ...
        let point_handles = surface.add_points(points_vec);
        debug_assert!(point_handles.len() == 2 * segments as usize);
        surface.add_tri([
            point_handles[point_handles.len() - 2],
            point_handles[point_handles.len() - 1],
            point_handles[0],
        ]);
        surface.add_tri([
            point_handles[0],
            point_handles[point_handles.len() - 1],
            point_handles[1],
        ]);
        surface.add_triangle_strip_with_points(point_handles);
        /* let vertical_and_diagonal_edges =
            surface.add_edge_strip(point_handles.iter().copied().chain([point_handles[0]]));
        let top_edges = surface.add_edge_strip(
            point_handles
                .iter()
                .copied()
                .chain([point_handles[0]])
                .step_by(2), //.chain([point_handles[0]]),
        );
        let bottom_edges = surface.add_edge_strip(
            point_handles
                .iter()
                .copied()
                .skip(1)
                .step_by(2) //.chain([point_handles[0]]),
                .chain([point_handles[1]]),
        ); */

        /* debug_assert!(
            bottom_edges.len() == top_edges.len(),
            "Cylinder needs equal amount of top and bottom edges"
        ); */

        /* let mut vert_tri_face_edges =
        Vec::with_capacity(vertical_and_diagonal_edges.len() * 2 + bottom_edges.len() * 2); */

        /* for id in 0..bottom_edges.len() {
            let vert_id = id * 2;
            let top_other = &vertical_and_diagonal_edges[vert_id..=vert_id + 1];
            let top = top_edges[id];
            vert_tri_face_edges.push(top);
            vert_tri_face_edges.extend_from_slice(top_other);
            //ensure counter clockwise
            let bottom_other = vertical_and_diagonal_edges[vert_id + 1];
            let bottom = bottom_edges[id];
            if vert_id + 2 == vertical_and_diagonal_edges.len() {
                vert_tri_face_edges.extend([bottom_other, bottom, vert_tri_face_edges[0]]);
                continue;
            }
            let bottom_other1 = vertical_and_diagonal_edges[vert_id + 2];
            vert_tri_face_edges.extend([bottom_other, bottom, bottom_other1]);
        } */

        //surface.add_faces_as_tri_edge_fill(vert_tri_face_edges);

        //Builds up the caps
        /* if *self.has_caps.get_value() {
            let forwards = point_handles
                .iter()
                .step_by(2)
                .skip(1)
                .take(((segments - 1) / 2) as usize)
                .copied();
            println!("Forward: {:?}", forwards.clone().collect::<Vec<_>>());
            let backwards = point_handles
                .iter()
                .rev()
                .skip(1)
                .step_by(2)
                .take(((segments) / 2) as usize - 1)
                .copied();

            println!("Backward: {:?}", backwards.clone().collect::<Vec<_>>());
            let top_cap_edges = surface.add_edge_strip(forwards.interleave(backwards));

            let forwards_faces = top_cap_edges.chunks_exact(2);
            for (index, edge) in forwards_faces.enumerate() {
                let face_edges = [edge[0], edge[1], top_edges[index + 1]];
                println!("{face_edges:?}");
                surface.add_tri(face_edges);
            }
            let backwards_faces = top_cap_edges[1..].chunks_exact(2);

            let top_edges_rev = top_edges.iter().rev().copied().collect::<Vec<_>>();
            for (index, edge) in backwards_faces.enumerate() {
                let face_edges = [top_edges_rev[index + 1], edge[0], edge[1]];
                println!("{face_edges:?}");
                surface.add_tri(face_edges);
            }
        } */
        Box::new([Some(OwnedDataType::new(surface))])
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
impl SocketInterface for CylinderV1 {
    fn get_input_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::default()
    }

    fn get_output_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::new([TypeDescriptor {
            r#type: DataTypeKind::Mesh,
            mutable: false,
        }])
    }
}
