use std::os::windows::io::HandleOrInvalid;
use std::slice::Iter;
use std::{num::NonZeroU32, ops::Index};

use glam::Vec3;
use im::{HashMap, HashSet, Vector};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

#[cfg(test)]
mod tests;

struct Test([Option<FaceHandle>; 2]);

trait Handle {}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub struct PointHandle(NonZeroU32);
impl PointHandle {
    pub fn idx(&self) -> usize {
        self.0.get() as usize - 1
    }
    pub fn new(raw_idx: usize) -> Self {
        PointHandle(NonZeroU32::new(raw_idx as u32).expect("Supplied 0 as Handle"))
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct EdgeHandle(PointHandle, PointHandle);

impl EdgeHandle {
    pub fn new(points: (PointHandle, PointHandle)) -> Self {
        //TODO validation
        EdgeHandle(points.0, points.1)
    }

    /* pub fn idx(&self) -> usize {
        //self.0.get() //TODO
        //+4_294_967_294
        self.0.get() as usize - 1
    } */
}
trait FacesInEdge {
    fn edge_full(&self) -> bool;
    fn insert_face(&mut self, face: FaceHandle);
}
impl FacesInEdge for [Option<FaceHandle>; 2] {
    fn edge_full(&self) -> bool {
        self[0].is_some() && self[1].is_some()
    }

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

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub struct FaceHandle(NonZeroU32);

impl FaceHandle {
    pub fn new(raw_idx: usize) -> Self {
        FaceHandle(NonZeroU32::new(raw_idx as u32).expect("Supplied 0 as Handle"))
    }
    pub fn idx(&self) -> usize {
        self.0.get() as usize - 1
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct OneiroiMesh {
    //Stores only connectivity information!
    //edges_of_point: Vector<SmallVec<EdgeHandle, 3>>,
    faces_of_point: Vector<SmallVec<FaceHandle, 3>>,

    //This is a HashMap to satisfy the querying needs in light of smooth shading.
    edges: HashMap<EdgeHandle, [Option<FaceHandle>; 2]>,

    points_of_face: Vector<[PointHandle; 3]>,
    edges_of_face: Vector<[EdgeHandle; 3]>,

    //TODO allow for multible surfaces
    //This is most likely done with adding an attribute of NonZeroU8/u8 idk yet to each POINT
    //this way all the information that builds up on those aka Edges and Faces
    //have this information indirectly associated with them
    //When traversing this would also implicitly hold since different surfaces cannot share points
    //and in turn cannot overlap
    //this has the additional benefit that everything uses the same buffers with only a Vec<NonZeroU8> any overhead

    //quads: Vector<Vector<usize>>,

    //Stores the Data associated with the Various Items
    data_points_position: Vector<Vec3>,
    data_faces_normal: Vector<Vec3>,
    data_hard_edge: HashSet<EdgeHandle>,
}

impl OneiroiMesh {
    //Keeps the winding order intact
    pub fn add_tri_with_points(&mut self, mut points: [PointHandle; 3]) -> FaceHandle {
        debug_assert!(points[0] != points[1] && points[1] != points[2] && points[0] != points[2]);
        // keep order of input
        self.points_of_face.push_back(points);
        let new_face = FaceHandle::new(self.points_of_face.len());

        self.faces_of_point
            .get_mut(points[0].idx())
            .unwrap()
            .push(new_face);
        self.faces_of_point
            .get_mut(points[1].idx())
            .unwrap()
            .push(new_face);
        self.faces_of_point
            .get_mut(points[2].idx())
            .unwrap()
            .push(new_face);

        //Before edges get inserted get right edge order
        points.sort();

        //SAFETY: Just sorted the points so indices can be taken
        let edge1 = unsafe { self.add_edge_unchecked((points[0], points[1])) };
        let edge3 = unsafe { self.add_edge_unchecked((points[0], points[2])) };
        let edge2 = unsafe { self.add_edge_unchecked((points[1], points[2])) };
        self.edges_of_face.push_back([edge1, edge2, edge3]);

        self.edges.get_mut(&edge1).unwrap().insert_face(new_face);
        self.edges.get_mut(&edge2).unwrap().insert_face(new_face);
        self.edges.get_mut(&edge3).unwrap().insert_face(new_face);

        debug_assert!(
            self.edges_of_face.len() == self.points_of_face.len(),
            "The length of edges_in_face and points_in_face is not equal!"
        );

        //calculate normal
        self.calculate_normal(new_face);

        new_face
    }

    pub fn add_triangle_strip_with_points(
        &mut self,
        points: impl IntoIterator<Item = PointHandle>,
    ) -> Vec<FaceHandle> {
        let mut new_faces = vec![];

        //TODO use next_chunk once stabilized
        let mut flip = false;
        for (point1, point2, point3) in points.into_iter().tuple_windows() {
            let face: FaceHandle;
            if flip {
                face = self.add_tri_with_points([point2, point1, point3]);
                flip = false;
            } else {
                face = self.add_tri_with_points([point1, point2, point3]);
                flip = true;
            }
            new_faces.push(face);
        }
        new_faces
    }

    // Should be sorted already -> smaller index of point in least significant bits
    unsafe fn add_edge_unchecked(&mut self, points: (PointHandle, PointHandle)) -> EdgeHandle {
        let new_handle = EdgeHandle(points.0, points.1);
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

    pub fn add_point(&mut self, position: Vec3) -> PointHandle {
        //TODO handle searching if point slot is free

        //TODO fill out
        //self.edges_of_point.push_back(SmallVec::new());
        self.faces_of_point.push_back(SmallVec::new());

        /* debug_assert!(
            self.edges_of_point.len() == self.faces_of_point.len(),
            "The length of faces_in_point and edges_in_point is not equal!"
        ); */

        self.data_points_position.push_back(position);

        debug_assert!(
            self.data_points_position.len() == self.faces_of_point.len(),
            "The Index of data_points_position is not equal to points length!"
        );

        PointHandle::new(self.faces_of_point.len())
    }

    //TODO can return a boxed slice
    pub fn add_points(&mut self, positions: impl IntoIterator<Item = Vec3>) -> Vec<PointHandle> {
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
            .collect::<Vec<_>>()
    }

    pub fn get_points(&self) -> Vector<Vec3> {
        self.data_points_position.clone()
    }

    //TODO this is dirty and needs a rename
    pub fn get_faces(&self) -> Vector<[PointHandle; 3]> {
        //TODO only submit points i guess
        self.points_of_face.clone()
    }

    pub fn get_face_normal(&self, face: FaceHandle) -> Vec3 {
        self.data_faces_normal[face.idx()]
    }

    pub fn points_in_face(&self, face: FaceHandle) -> [PointHandle; 3] {
        self.points_of_face[face.idx()]
    }

    /* pub fn edges_in_point(&self, point: PointHandle) -> impl Iterator<Item = EdgeHandle> + use<> {
        //Clone should be fine here?
        self.edges_of_point[point.idx()].clone().into_iter()
    } */

    pub fn faces_in_point(&self, point: PointHandle) -> Iter<'_, FaceHandle> {
        self.faces_of_point[point.idx()].iter()
    }

    pub fn edges_in_face(&self, face: FaceHandle) -> [EdgeHandle; 3] {
        self.edges_of_face[face.idx()]
    }

    pub fn points_in_edge(&self, edge: EdgeHandle) -> [PointHandle; 2] {
        if self.edges.contains_key(&edge) {
            [edge.0, edge.1]
        } else {
            panic!()
        }
    }

    pub fn point_position(&self, point: PointHandle) -> Vec3 {
        self.data_points_position[point.idx()]
    }

    fn calculate_normal(&mut self, face: FaceHandle) {
        let points = self.points_in_face(face);

        let point_0 = self.point_position(points[1]);
        let point_1 = self.point_position(points[0]);
        let point_2 = self.point_position(points[2]);
        let vec1 = point_0 - point_2;
        let vec2 = point_0 - point_1;

        if self.data_faces_normal.len() + 1 == face.idx() {
            self.data_faces_normal
                .push_back(vec1.cross(vec2).normalize());
        } else {
            self.data_faces_normal
                .insert(face.idx(), vec1.cross(vec2).normalize());
        }
    }

    pub fn move_tri(&mut self, face: &FaceHandle, offset: Vec3) {
        //TODO If not moving along normal need to recalculate normal for all points

        let points = &self.points_of_face[face.idx()];
        //TODO factor out in move point
        for point in points {
            //TODO maybe factor out in set_point_position
            self.data_points_position[point.idx()] += offset;
        }
    }

    pub fn move_point(&mut self, point: &PointHandle, offset: Vec3) {
        self.data_points_position[point.idx()] += offset;
    }

    //TODO maybe make this more flexible to allow varieng points
    pub fn rebind_tri(&mut self, face: FaceHandle, points: [PointHandle; 3]) {
        //TODO broken method
        self.points_of_face[face.idx()] = points;
    }

    pub fn duplicate_point(&mut self, point_handle: &PointHandle) -> PointHandle {
        self.add_point(self.data_points_position[point_handle.idx()])
    }
}
