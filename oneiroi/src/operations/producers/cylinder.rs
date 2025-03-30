use core::f32;

use glam::Vec3;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    asset::instance::AssetInstance,
    data_types::{DataTypeInstance, DataTypeType, Property, PropertyMetadata},
    mesh::OneiroiMesh,
    operations::{Operation, PropertyInterface, StaticNodeMetadata},
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
    fn try_set_property(&mut self, property: &str, value: DataTypeInstance) -> Result<(), ()> {
        match property {
            "radius" => {
                self.radius
                    .update(value.inner_f32().expect("TODO error handling"));
                Ok(())
            }
            "height" => {
                self.height
                    .update(value.inner_f32().expect("TODO error handling"));
                Ok(())
            }
            "segments" => {
                self.segments
                    .update(value.inner_int().expect("TODO error handling"));
                Ok(())
            }
            "has_caps" => {
                self.has_caps
                    .update(value.inner_bool().expect("TODO error handling"));
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
            "radius" => Ok(self.radius.get_instance()),
            "height" => Ok(self.height.get_instance()),
            "segments" => Ok(self.segments.get_instance()),
            "has_caps" => Ok(self.has_caps.get_instance()),

            _ => {
                //println!("called get_prop with {:?}", property);
                Err(())
            }
        }
    }

    fn get_properties(&self) -> Vec<PropertyMetadata> {
        //TODO this can be most likely optimized
        let default = Self::default();

        let mut info = self.radius.get_property_meta();
        info.set_name("radius");
        info.set_default(default.radius.get_instance());

        let mut info2 = self.height.get_property_meta();
        info2.set_name("height");
        info2.set_default(default.height.get_instance());

        let mut info3 = self.segments.get_property_meta();
        info3.set_name("segments");
        info3.set_default(default.segments.get_instance());

        let mut info4 = self.has_caps.get_property_meta();
        info4.set_name("has_caps");
        info4.set_default(default.has_caps.get_instance());
        vec![info, info2, info3, info4]
    }
}

impl Operation for CylinderV1 {
    fn compute(&self, _: Vec<&DataTypeInstance>) -> Vec<DataTypeInstance> {
        let mut surface = OneiroiMesh::default();
        let segments = *self.segments.get_value();
        let height = *self.height.get_value(); // can also / 2 and add a origin point
        let radius = *self.radius.get_value();

        let angle = 360. / (segments as f32);

        let mut points_vec: Vec<Vec3> = Vec::with_capacity((2 * segments) as usize);

        //calculate the position for all points
        for segement in 0..segments {
            let angle = (angle * segement as f32) * (f32::consts::PI / 180.);

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
        surface.add_tri_with_points([
            point_handles[point_handles.len() - 2],
            point_handles[point_handles.len() - 1],
            point_handles[0],
        ]);
        surface.add_tri_with_points([
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
