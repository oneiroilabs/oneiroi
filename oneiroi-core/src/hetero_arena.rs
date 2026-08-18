use std::collections::HashMap;

use glam::{Mat4, Vec2, Vec3, Vec4};

use crate::{curve::nurbs::CubicNurbsSegmentCache, data_type::DataType};

// Idea how to model the GPU cache for the Graphs without overhead. Should work with bindless iiuc.
// The accessors HashMap would need to be moved out and maybe codegened into the arguments itself because the graph is baked hence we should be able to use hard offsets.
pub struct HeterogeneousArena {
    vectors: Vec<Vec4>,
    cubic_nurbs: Vec<CubicNurbsSegmentCache>,
    //accessors: HashMap<u32, StorageLocation>,
}

trait ArenaAccess<T: DataType> {
    fn get(&self, index: u32) -> Option<T>;
    fn set(&mut self, index: u32, value: T) -> u32;
}

impl ArenaAccess<Vec3> for HeterogeneousArena {
    #[inline(always)]
    fn get(&self, index: u32) -> Option<Vec3> {
        self.vectors
            .get(index as usize)
            .map(|v| Vec3::new(v.x, v.y, v.z))
    }

    #[inline(always)]
    fn set(&mut self, index: u32, value: Vec3) -> u32 {
        self.vectors[index as usize] = Vec4::new(value.x, value.y, value.z, 0.);
        0
    }
}

impl ArenaAccess<Vec2> for HeterogeneousArena {
    #[inline(always)]
    fn get(&self, index: u32) -> Option<Vec2> {
        self.vectors
            .get(index as usize)
            .map(|v| Vec2::new(v.x, v.y))
    }

    #[inline(always)]
    fn set(&mut self, index: u32, value: Vec2) -> u32 {
        self.vectors[index as usize] = Vec4::new(value.x, value.y, 0., 0.);
        0
    }
}

impl HeterogeneousArena {
    #[inline(always)]
    pub fn get<T: DataType>(&self, idx: u32) -> Option<T>
    where
        Self: ArenaAccess<T>,
    {
        ArenaAccess::get(self, idx)
    }

    #[inline(always)]
    pub fn set<T: DataType>(&mut self, idx: u32, value: T) -> u32
    where
        Self: ArenaAccess<T>,
    {
        ArenaAccess::set(self, idx, value)
    }
}

/* pub trait ExecutionContext {
    //type Index;

    fn get_unchecked<T>(&self, index: u32) -> T;
    fn set_unchecked<T>(&self, index: u32, value: T);
} */
