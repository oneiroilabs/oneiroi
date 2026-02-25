use std::fmt::Debug;

use std::num::NonZeroU32;

use glam::Vec3;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::type_system::trait_types::{MeshMut0D, MeshMut1D, MeshMut2D};
use crate::type_system::data_types::{ArributeMetadata, DataType, DataTypeKind};
use crate::type_system::variants::{OwnedDataType, TypeRef};
use crate::type_system::reference::Reference;
use crate::{ImHashMap, ImHashSet, ImVec};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub struct PointHandle(NonZeroU32);
impl PointHandle {
    #[inline]
    pub(crate) fn idx(&self) -> usize {
        self.0.get() as usize - 1
    }
    #[inline]
    pub(crate) fn get(self) -> u32 {
        self.0.get() - 1
    }

    #[inline]
    pub(crate) fn new(raw_idx: usize) -> Self {
        PointHandle(NonZeroU32::new(raw_idx as u32).expect("Supplied 0 as Handle"))
    }
}

/// An Edge id is 8 byte wide and consists of two points.
/// To be a valid Edge the smaller PointHandle always needs to be at the front of the tuple.
/// This is done because Edges are in a Mesh are undirected hence a convention needs to be created.
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Edge(PointHandle, PointHandle);

impl Edge {
    pub fn new(points: (PointHandle, PointHandle)) -> Self {
        debug_assert!(
            points.0 != points.1,
            " Cannot construct Edge with same Points"
        );

        if points.0 < points.1 {
            //SAFETY: We just correctly sorted the Points.
            unsafe { Edge::from_points_unchecked(points) }
        } else {
            //SAFETY: We just correctly sorted the Points.
            unsafe { Edge::from_points_unchecked((points.1, points.0)) }
        }
    }

    /// Produces an edge in the Same order the Points were supplied.
    /// Expects that the smaller PointHandle is at the front.
    /// Doesn't perfom validity checks. must be done by the caller.
    #[inline]
    unsafe fn from_points_unchecked(points: (PointHandle, PointHandle)) -> Self {
        Edge(points.0, points.1)
    }

    /// Returns the Point handle which is not the input handle
    /// SAFETY: must supply valid point handle
    #[inline]
    pub(crate) fn not_point(&self, handle: PointHandle) -> PointHandle {
        debug_assert!(handle == self.0 || handle == self.1);
        if self.0 == handle { self.1 } else { self.0 }
    }

    /* pub fn idx(&self) -> usize {
        //self.0.get() //TODO
        //+4_294_967_294
        self.0.get() as usize - 1
    } */
}
trait FacesInEdge {
    fn insert_face(&mut self, face: FaceHandle);
}
impl FacesInEdge for [Option<FaceHandle>; 2] {
    fn insert_face(&mut self, face: FaceHandle) {
        if self[0].is_some() {
            self[1] = Some(face);
            debug_assert!(self[0] != self[1]);
            self.sort()
        } else {
            self[0] = Some(face);
        }
    }
}

/// Identifies a Triangle in the Mesh.
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub struct FaceHandle(NonZeroU32);

impl FaceHandle {
    #[inline]
    pub fn new(raw_idx: usize) -> Self {
        FaceHandle(NonZeroU32::new(raw_idx as u32).expect("Supplied 0 as Handle"))
    }
    #[inline]
    pub fn idx(&self) -> usize {
        self.0.get() as usize - 1
    }
}

/// TODO should be a builder which can request all the things on demand.
/// Provides the Mesh connectivity information as a Indexed Buffer / Tringable Table.
/// Since an owned version of the data is supplied the Vecs can be directly consumed.
pub struct IndexedMeshBuffers {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,

    pub indices: Vec<u32>,
}

trait FacesOfEdge {
    fn is_full(&self, handle: Edge) -> bool;
}

//impl FacesOfEdge for HashMap<Edge, [Option<FaceHandle>; 2]> {
impl FacesOfEdge for ImHashMap<Edge, [Option<FaceHandle>; 2]> {
    fn is_full(&self, handle: Edge) -> bool {
        //This unwrap could potentially be gracefully handled
        let faces = self.get(&handle).unwrap();
        faces[0].is_some() && faces[1].is_some()
    }
}

/// This custom Mesh format tries to achieve the following things:
/// - Always use Triangles and dont allow anything else under the hood.
/// - Always maintain a correct topolgy without exceptions.
/// - Use structural stahring to allow for efficient mutation.
///   Therefore it separates the connectivity from the data and uses a data-oriented approach.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Mesh {
    //Stores only connectivity information!
    //edges_of_point: Vector<SmallVec<EdgeHandle, 3>>,
    faces_of_point: ImVec<SmallVec<FaceHandle, 3>>,

    //This is a HashMap to satisfy the querying needs in light of smooth shading.
    edges: ImHashMap<Edge, [Option<FaceHandle>; 2]>,

    points_of_face: ImVec<[PointHandle; 3]>,
    edges_of_face: ImVec<[Edge; 3]>,

    //TODO allow for multible surfaces
    //This is most likely done with adding an attribute of NonZeroU8/u8 idk yet to each POINT
    //this way all the information that builds up on those aka Edges and Faces
    //have this information indirectly associated with them
    //When traversing this would also implicitly hold since different surfaces cannot share points
    //and in turn cannot overlap
    //this has the additional benefit that everything uses the same buffers with only a Vec<NonZeroU8> any overhead

    //Stores the Data associated with the Various Items
    data_points_position: ImVec<Vec3>,
    //data_faces_normal: ImVec<Vec3>,
    data_hard_edge: ImHashSet<Edge>,
    //data_faces_material: ImHashMap<FaceHandle, u8>,

    // TODO allow for multible surfaces and materials
    material: Option<Reference>,
}

impl Mesh {
    pub(crate) fn add_triangle_strip_with_points(
        &mut self,
        points: impl IntoIterator<Item = PointHandle>,
    ) -> Vec<FaceHandle> {
        let mut new_faces = vec![];

        //TODO use next_chunk once stabilized
        let mut flip = false;
        for (point1, point2, point3) in points.into_iter().tuple_windows() {
            let face: FaceHandle;
            if flip {
                face = self.add_tri([point2, point1, point3]);
                flip = false;
            } else {
                face = self.add_tri([point1, point2, point3]);
                flip = true;
            }
            new_faces.push(face);
        }
        new_faces
    }

    /// This method batch insertes new Points into the Mesh.
    /// Since it is not using unoccupied indices but instead appends
    /// the new points to the end, it can return a Boxed slice.
    pub(crate) fn add_points(
        &mut self,
        positions: impl IntoIterator<Item = Vec3>,
    ) -> Box<[PointHandle]> {
        let mut length: usize = 0;

        for position in positions {
            self.data_points_position.push_back(position);
            //self.edges_of_point.push_back(SmallVec::new());
            self.faces_of_point.push_back(SmallVec::new());
            length += 1;
        }

        /* debug_assert!(
            self.edges_of_point.len() == self.faces_of_point.len(),
            "The length of faces_in_point and edges_in_point is not equal!"
        ); */

        debug_assert!(
            self.data_points_position.len() == self.faces_of_point.len(),
            "The Index of data_points_position is not equal to points length!"
        );

        let new_len = self.faces_of_point.len() + 1;

        (new_len - length..new_len)
            .map(PointHandle::new)
            .collect::<Box<_>>()
    }

    pub(crate) fn points_in_face(&self, face: FaceHandle) -> [PointHandle; 3] {
        self.points_of_face[face.idx()]
    }

    pub(crate) fn edges_in_face(&self, face: FaceHandle) -> [Edge; 3] {
        self.edges_of_face[face.idx()]
    }

    pub(crate) fn points_in_edge(&self, edge: Edge) -> [PointHandle; 2] {
        if self.edges.contains_key(&edge) {
            [edge.0, edge.1]
        } else {
            panic!()
        }
    }

    /// Calculates the face normal to use later.
    /// The returned normal is not normalized.
    fn calc_face_normal(&self, face: FaceHandle) -> Vec3 {
        let points = self.points_in_face(face);

        let point_0 = self.position(points[1]);
        let point_1 = self.position(points[0]);
        let point_2 = self.position(points[2]);

        let vec1 = point_0 - point_2;
        let vec2 = point_0 - point_1;

        vec1.cross(vec2)
    }

    /// Calculates the Point Normals of the Mesh which can be used to render it.
    fn calculate_point_normals(&self) -> Vec<Vec3> {
        // First calculate the unnormalized Face normals.
        // Preallocate for every Face
        let mut face_normals = Box::new_uninit_slice(self.points_of_face.len());
        for face in 1..=self.points_of_face.len() {
            face_normals[face - 1].write(self.calc_face_normal(FaceHandle::new(face)));
        }
        let face_normals = unsafe { face_normals.assume_init() };

        // Preallocate for every Point.
        let mut normals = Vec::with_capacity(self.faces_of_point.len());
        for point in &self.faces_of_point {
            // Summate the normals of the adjacent Faces of Point.
            let mut normal = Vec3::ZERO;
            for face in point {
                normal += face_normals[face.idx()];
            }
            normals.push(normal.normalize_or_zero());
        }

        normals
    }

    pub(crate) fn set_edge_hard(&mut self, edge: Edge) {
        self.data_hard_edge.insert(edge);
    }

    fn has_normal_contention(&self, edge: Edge) -> bool {
        self.edges.is_full(edge) //TODO
    }

    /// Returns the Buffers which can be used to render the Mesh.
    /// TODO only return positions and indices most likely everything else on demand.
    pub fn get_index_mesh_buffers(&self) -> IndexedMeshBuffers {
        for hard_edge in self.data_hard_edge.iter() {
            if self.has_normal_contention(*hard_edge) {}
        }

        let positions: Vec<Vec3> = self
            .data_points_position
            .iter()
            .copied()
            .collect::<Vec<_>>();

        let normals: Vec<Vec3> = self.calculate_point_normals();

        let indices = self
            .points_of_face
            .clone()
            .into_iter()
            .flatten()
            .map(|h| h.get())
            .collect::<Vec<_>>();

        println!(
            "Pos: {positions:?}, Nor: {normals:?}, Ind: {indices:?}, Lengths: {},{},{}",
            positions.len(),
            normals.len(),
            indices.len()
        );

        IndexedMeshBuffers {
            positions,
            normals,
            indices,
        }
    }

    /// SAFETY: Expects a valid Material to be passed in.
    /// MUST be verified by the caller.
    pub(crate) fn set_material(&mut self, material: Reference) {
        //println!("Why are we not setting the materiall?");
        self.material = Some(material);
    }

    pub fn get_material_ref(&self) -> Option<Reference> {
        self.material
    }
}

impl DataType for Mesh {
    const DATA_TYPE_TYPE: DataTypeKind = DataTypeKind::Mesh;

    fn get_type_ref(value: TypeRef) -> &Self {
        match value {
            TypeRef::Mesh(val) => val,
            _ => unreachable!(),
        }
    }

    type ConfigurationOptions = ();

    fn get_type(value: OwnedDataType) -> Self {
        match value {
            OwnedDataType::Mesh(val) => *val,
            _ => unreachable!(),
        }
    }

    fn intrinsic_attributes() -> Option<Box<[ArributeMetadata]>> {
        None
    }

    fn to_data_type_value(&self) -> OwnedDataType {
        OwnedDataType::Mesh(Box::new(self.clone()))
    }

    fn to_data_type_ref(&self) -> TypeRef {
        TypeRef::Mesh(self)
    }
}

impl MeshMut0D for Mesh {
    type PointHandle = PointHandle;

    fn position(&self, handle: PointHandle) -> Vec3 {
        self.data_points_position[handle.idx()]
    }

    fn set_position(&mut self, point: PointHandle, position: Vec3) {
        //println!("Set Called with: {point:?}");
        self.data_points_position[point.idx()] = position;
        /* for face in self.faces_of_point[point.idx()].clone() {
            self.unnormalized_normal(face);
        } */
    }

    fn add_point(&mut self, position: Vec3) -> PointHandle {
        //TODO handle searching if point slot is free

        //TODO fill out
        //self.edges_of_point.push_back(SmallVec::new());
        self.faces_of_point.push_back(SmallVec::new());

        self.data_points_position.push_back(position);

        debug_assert!(
            self.data_points_position.len() == self.faces_of_point.len(),
            "The Index of data_points_position is not equal to points length!"
        );

        PointHandle::new(self.faces_of_point.len())
    }
}

impl MeshMut1D for Mesh {
    type Edge = Edge;

    // Should be sorted already -> smaller index of point in least significant bits
    unsafe fn add_edge_unchecked(&mut self, points: (PointHandle, PointHandle)) -> Edge {
        let new_handle = Edge(points.0, points.1);
        if self.edges.insert(new_handle, [None; 2]).is_some() {
            //self.faces_of_edge.push_back([None; 2]);
        }

        //TODO self.edges_in_point

        /* debug_assert!(
            self.points_of_edge.len() == self.faces_of_edge.len(),
            "The length of points_in_edge and faces_in_edge is not equal!"
        ); */
        new_handle
    }

    #[inline]
    fn not_point(edge: Self::Edge, handle: Self::PointHandle) -> Self::PointHandle {
        edge.not_point(handle)
    }
}

impl MeshMut2D for Mesh {
    type FaceHandle = FaceHandle;

    //Keeps the winding order intact
    fn add_tri(&mut self, mut points: [PointHandle; 3]) -> FaceHandle {
        debug_assert!(
            points[0] != points[1] && points[1] != points[2] && points[0] != points[2],
            "To add a triangle each point must be unique."
        );
        // keep order of input
        self.points_of_face.push_back(points);
        let new_face = FaceHandle::new(self.points_of_face.len());

        self.faces_of_point[points[0].idx()].push(new_face);
        self.faces_of_point[points[1].idx()].push(new_face);
        self.faces_of_point[points[2].idx()].push(new_face);

        //Before edges get inserted get right edge order
        points.sort();

        //SAFETY: Just sorted the points so indices can be taken
        let edge1 = unsafe { self.add_edge_unchecked((points[0], points[1])) };
        let edge2 = unsafe { self.add_edge_unchecked((points[1], points[2])) };
        let edge3 = unsafe { self.add_edge_unchecked((points[0], points[2])) };
        self.edges_of_face.push_back([edge1, edge2, edge3]);

        self.edges[&edge1].insert_face(new_face);
        self.edges[&edge2].insert_face(new_face);
        self.edges[&edge3].insert_face(new_face);

        debug_assert!(
            self.edges_of_face.len() == self.points_of_face.len(),
            "The length of edges_in_face and points_in_face must be equal!"
        );

        new_face
    }
}
