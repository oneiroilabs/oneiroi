use glam::{Mat4, Vec2, Vec3, Vec4};

use crate::{curve::nurbs::CubicNurbsSegmentCache, types::DataType};

/// This is an arena available to [DataType]s
pub struct Arena {
    vectors: Vec<Vec4>,
    cubic_nurbs: Vec<CubicNurbsSegmentCache>,
}

trait ArenaAccess<T: DataType> {
    fn get(&self, index: u32) -> Option<T>;
    fn set(&mut self, index: u32, value: T) -> u32;
}

impl ArenaAccess<Vec3> for Arena {
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

impl ArenaAccess<Vec2> for Arena {
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

impl Arena {
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
