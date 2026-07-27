use std::collections::HashMap;

use glam::Mat4;

use crate::nurbs::CubicNurbsSegmentCache;

// Idea how to model the GPU cache for the Graphs without overhead. Should work with bindless iiuc.
// The accessors HashMap would need to be moved out and maybe codegened into the arguments itself because the graph is baked hence we should be able to use hard offsets.
pub struct HeterogeneousArena {
    matricies: Vec<Mat4>,
    cubic_nurbs: Vec<CubicNurbsSegmentCache>,
    accessors: HashMap<u32, StorageLocation>,
}

impl HeterogeneousArena {
    pub fn get<T>(&self, idx: u32) -> Option<T> {
        /* match self.accessors.get(&idx)? {
            StorageLocation::F64(f) => *f,
            StorageLocation::CubicNurbs(_, _) => todo!(),
            StorageLocation::Matrix(_) => ,
        } */
        None
    }

    pub fn set<T>(&mut self, idx: u32, value: T) {
        /* if let Some(StorageLocation::F64(offset)) = self.route_map.get(&idx) {
            self.f64_storage[*offset] = value;
        } */
    }
}

#[derive(Clone, Copy)]
pub enum StorageLocation {
    F64(f32),
    CubicNurbs(u32, u32),
    Matrix(usize),
}
