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
        data_types::{DataType, DataTypeKind, Mesh, Selection, TypeDescriptor},
    },
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtrudeV1 {
    query: Property<Selection>,
    amount: Property<f32>,
    merge_adjacent_normals: Property<bool>,
    //inputs: Vec<SocketConnection>,
}
impl Default for ExtrudeV1 {
    fn default() -> Self {
        Self {
            query: Property::new(Selection::new("1")),
            amount: Property::new(1.0),
            merge_adjacent_normals: Property::new(true),
            //inputs: vec![SocketConnection::default()],
        }
    }
}

/* impl ExtrudeV1 {
    fn extrude_edge(&self, edge: Edge, context: &mut Mesh) {
        let points = context.points_in_edge(edge);
    }

    fn extrude_point(&self, point: PointHandle, context: &mut Mesh) {}
} */

impl Node for ExtrudeV1 {
    fn compute(
        &self,
        input_sockets: Option<&[Reference]>,
        context: &impl ContextProvider,
        //TODO explore this
        // write_back: [&mut DataTypeValue],
    ) -> Box<[Option<OwnedDataType>]> {
        let input = context.get_reference(input_sockets.unwrap()[0]);
        let input: &Mesh = input.dispatch_ref().unwrap();
        let new_surface = input.clone();
        //let selection: Vec<FaceHandle> = self.query.get_value(context).try_get().unwrap();
        //TODO
        return Box::new([Some(OwnedDataType::Mesh(Box::new(new_surface)))]);
        /* let mut point_map = HashMap::new();
        let mut point_set: HashSet<PointHandle> = HashSet::new();
        //let mut edge_set = HashSet::new();
        for face_handle in &selection {
            //TODO maybe dont need to clone
            let points_in_face = *input.get_points_in_face(face_handle);
            let points: HashSet<PointHandle> = HashSet::from_iter(points_in_face);
            let new_points: Vec<PointHandle> =
            //TODO maybe somehow remove this copied in a larger refactor
                points.difference/* relative_complement */(&point_set).copied().collect();
            for point in new_points {
                let new_handle = new_surface.duplicate_point(&point);
                point_map.insert(point, new_handle);
                point_set.insert(point);
                if *self.merge_adjacent_normals.get_value() {
                    //TODO average out normals
                    /* let normals_to_average: Vec<Vec3> = self
                    .get_faces_in_point(&new_handle)
                    .iter()
                    .map(|f| self.get_face_normal(*f))
                    .collect(); */
                    //TODO this is not accounting for different face normals
                    new_surface.move_point(
                        &new_handle,
                        input.get_face_normal(*face_handle) * self.amount.get_value(),
                    )
                }
            }
            /* let edges_in_face = self.edges_in_face[face_handle.idx()].clone();
            edge_set = edges_in_face.difference(edge_set);

                for edge in edge_set {
                    self.points_in_edge
                    } */

            //TODO this is a mess need to connect need to filter what to connect wiith edge set
            new_surface.add_tri([
                points_in_face[0],
                points_in_face[1],
                point_map[&points_in_face[0]],
            ]);
            new_surface.add_tri([
                points_in_face[1],
                point_map[&points_in_face[1]],
                point_map[&points_in_face[0]],
            ]);

            new_surface.add_tri([
                points_in_face[1],
                points_in_face[2],
                point_map[&points_in_face[1]],
            ]);
            new_surface.add_tri([
                points_in_face[2],
                point_map[&points_in_face[2]],
                point_map[&points_in_face[1]],
            ]);

            new_surface.add_tri([
                points_in_face[2],
                points_in_face[0],
                point_map[&points_in_face[2]],
            ]);
            new_surface.add_tri([
                points_in_face[0],
                point_map[&points_in_face[0]],
                point_map[&points_in_face[2]],
            ]);

            new_surface.rebind_tri(
                *face_handle,
                [
                    point_map[&points_in_face[0]],
                    point_map[&points_in_face[1]],
                    point_map[&points_in_face[2]],
                ],
            );
            /* self.move_tri(
            face_handle,
            self.get_face_normal(*face_handle) * extrude_amount,
            ); */
        }
        */

        //Dependant on average_adjacent_varying_normals
        /*  let mut shared_edges: HashSet<Edge> = HashSet::new();
        let mut outside_edges = HashSet::with_capacity(selection.len() * 3);

        let edges = selection.into_iter().flat_map(|f| input.edges_in_face(f));
        for edge in edges {
            if !outside_edges.insert(edge) {
                shared_edges.insert(edge);
            }
        }
        for outside_edge in outside_edges.difference(&shared_edges) {
            self.extrude_edge(*outside_edge, &mut new_surface);
        } */

        //let mut point_set = HashSet::new();
        /* for edge_handle in edge_set {
        let new_point_set = point_set.union(self.points_in_edge[edge_handle.0].clone());
        point_set = new_point_set;
        } */

        /* for point_handle in point_set {
        let idx = self.add_point(self.data_points_position[point_handle.0]);
        //new_points.push_back(idx);
        //self.add_edge([point, idx]);
        } */
        /* for face_handle in selection {
        let points = self.points_in_face[face_handle.0].clone();
        let mut new_points: Vector<PointHandle> = Vector::new();
        for point in points {
            let idx = self.add_point(self.get_point_position(&point));
            new_points.push_back(idx);
            self.add_edge([point, idx]);
            }

            self.rebind_tri(face_handle, [new_points[0], new_points[1], new_points[2]]);
            self.move_tri(
                &face_handle,
                self.get_face_normal(face_handle) * extrude_amount,
                );
                } */

        Box::new([Some(OwnedDataType::new(new_surface))])
    }

    /* fn get_sockets(
        &self,
    ) -> (
        Vec<crate::data_types::DataTypeType>,
        Vec<crate::data_types::DataTypeType>,
    ) {
        (vec![DataTypeType::Mesh], vec![DataTypeType::Mesh])
    } */
    fn node_metadata(&self) -> StaticNodeMetadata {
        StaticNodeMetadata { color: "#4338ca" }
    }
}
impl SocketInterface for ExtrudeV1 {
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

impl PropertyInterface for ExtrudeV1 {
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
            "query" => Ok(self.query.get_literal_value().to_data_type_ref()),
            "amount" => Ok(self.amount.get_literal_value().to_data_type_ref()),
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
            name: "amount".into(),
            r#type: default.amount.get_type(),
            default: default.amount.get_literal_value().to_data_type_value(),
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
