use itertools::Itertools;

use crate::type_system::data_types::Vec3;

/* trait IPointHandle: PartialEq + Eq + Ord + Clone + Copy {}

trait IEdge: Clone + Copy + PartialEq + Eq {
    fn not_point(&self) -> Self::PointHandle;
}

trait IFaceHandle: Clone + Copy + PartialEq + Eq + PartialOrd + Ord {} */

pub trait MeshMut0D {
    type PointHandle: PartialEq + Eq + Ord + Clone + Copy;
    /// Gets the position of the supplied Point.
    fn position(&self, handle: Self::PointHandle) -> Vec3;
    /// Overwrites the position of the supplied points.
    fn set_position(&mut self, point: Self::PointHandle, position: Vec3);
    /// Adds a new Point to the Mesh.
    fn add_point(&mut self, position: Vec3) -> Self::PointHandle;
}

pub trait MeshMut1D: MeshMut0D {
    type Edge: Clone + Copy + PartialEq + Eq;

    /// Method to retrieve the opposite point of an Edge.
    /// Maybe factor this out into own trait or so but fine for now.
    fn not_point(edge: Self::Edge, handle: Self::PointHandle) -> Self::PointHandle;

    /// Duplicates the input Point onto the same Position.
    /// Also an Edge will be connected between the two points.
    /// Returns the newly created Point and the newly created Edge.
    fn extrude_connectivity(
        &mut self,
        handle: Self::PointHandle,
    ) -> (Self::PointHandle, Self::Edge) {
        let pos = self.position(handle);
        let new_point = self.add_point(pos);
        let new_edge = self.add_edge((handle, new_point));
        (new_point, new_edge)
    }

    /// # Safety
    /// Should be sorted already -> smaller index of point in the first position
    unsafe fn add_edge_unchecked(
        &mut self,
        points: (Self::PointHandle, Self::PointHandle),
    ) -> Self::Edge;

    #[track_caller]
    fn add_edge(&mut self, points: (Self::PointHandle, Self::PointHandle)) -> Self::Edge {
        debug_assert!(
            points.0 != points.1,
            "Points arent allowed to be the same when creating an edge"
        );
        //SAFETY: we sort the points before inserting
        if points.0 < points.1 {
            unsafe { self.add_edge_unchecked(points) }
        } else {
            unsafe { self.add_edge_unchecked((points.1, points.0)) }
        }
    }

    fn add_edge_strip(
        &mut self,
        points: impl IntoIterator<Item = Self::PointHandle>,
        closed: bool,
    ) -> Box<[Self::Edge]> {
        let points = points.into_iter();
        let len = points.try_len().unwrap();
        //println!("{len}");
        //The capacity of the edges is dependant if the EdgeStrip is closed or not.
        let mut new_edges = if closed {
            Box::new_uninit_slice(len)
        } else {
            Box::new_uninit_slice(len - 1)
        };

        let mut first = None;
        let mut last = None;
        for (index, (a, b)) in points.tuple_windows().enumerate() {
            //println!("{index}, ({a:?},{b:?})");
            if index == 0 {
                first = Some(a);
            }
            if index == len - 2 {
                last = Some(b);
            }
            new_edges[index].write(self.add_edge((a, b)));
        }
        if closed {
            new_edges[len - 1].write(self.add_edge((first.unwrap(), last.unwrap())));
        }

        //SAFETY: Everything was initialized beforehand.
        unsafe { new_edges.assume_init() }
    }
}

/// In the 2D World "Faces" which are always Trinagles in this case can be produced since they are planar.
pub trait MeshMut2D: MeshMut1D {
    type FaceHandle: Clone + Copy + PartialEq + Eq + PartialOrd + Ord;

    fn add_tri(&mut self, points: [Self::PointHandle; 3]) -> Self::FaceHandle;

    /// Points all get connected together
    /// TODO: Should correctly handle closed polys either through params or not
    fn extrude_edge_strip_connectivity(
        &mut self,
        edge_strip_points: impl IntoIterator<Item = Self::PointHandle>,
    ) -> Box<[Self::PointHandle]> /* (Box<[PointHandle]>, Box<[Edge]>) */ {
        let points = edge_strip_points.into_iter();
        let len = points.try_len().unwrap();

        // Preallocate for every Point and extrude every Point producing new Point and Edge.
        let mut extruded_points_info = Box::new_uninit_slice(len);
        for (index, point) in points.into_iter().enumerate() {
            let extrude = self.extrude_connectivity(point);
            extruded_points_info[index].write(extrude);
        }
        //SAFETY: Everything got initialized just before.
        let extruded_points_info = unsafe { extruded_points_info.assume_init() };

        // Reallocate the Points which also get returned by the function afterwards.
        let mut points = Box::new_uninit_slice(extruded_points_info.len());
        points[0].write(extruded_points_info[0].0);

        for index in 1..extruded_points_info.len() {
            let (last_extruded_point, last_extruded_edge) = extruded_points_info[index - 1];
            let (cur_extruded_point, cur_extruded_edge) = extruded_points_info[index];

            points[index].write(cur_extruded_point);

            let last_bot_point = Self::not_point(last_extruded_edge, last_extruded_point);
            let cur_bot_point = Self::not_point(cur_extruded_edge, cur_extruded_point);
            /* println!(
                "F1: {:?}, F2: {:?}",
                [last_gen.0, last_bot_point, cur_bot_point],
                [cur_bot_point, cur_gen.0, last_gen.0]
            ); */

            self.add_tri([last_bot_point, last_extruded_point, cur_bot_point]);
            self.add_tri([cur_extruded_point, cur_bot_point, last_extruded_point]);
        }

        //SAFETY: Everything got initialized above.
        unsafe { points.assume_init() }
    }

    fn bevel_edges(&mut self) {}
}
