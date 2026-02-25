use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::type_system::{
    data_types::{DataType, DataTypeKind},
    variants::{TypeRef, OwnedDataType},
};

//TODO maybe make this always hold Vec3s
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Outline {
    // Points are inserted in counter clockwise order
    points: Vec<Vec2>,
    closed: bool,
}

impl Outline {
    pub fn new() -> Self {
        Self {
            points: vec![
                Vec2::new(0.5, -0.5),
                Vec2::new(0.5, 0.5),
                Vec2::new(-0.5, 0.5),
                Vec2::new(-0.5, -0.5),
            ],
            closed: true,
        }
    }

    pub fn with_points(points: Vec<Vec2>) -> Self {
        Self {
            points,
            closed: true,
        }
    }

    pub(crate) fn iterate(&self) -> impl Iterator<Item = Vec2> {
        self.points.iter().cloned()
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.closed
    }
}

impl DataType for Outline {
    const DATA_TYPE_TYPE: super::DataTypeKind = DataTypeKind::Outline;

    fn intrinsic_attributes() -> Option<Box<[super::ArributeMetadata]>> {
        todo!()
    }

    type ConfigurationOptions = ();
    fn get_type_ref(value: TypeRef) -> &Self {
        match value {
            TypeRef::Outline(value) => value,
            _ => unreachable!(),
        }
    }

    fn get_type(value: OwnedDataType) -> Self {
        match value {
            OwnedDataType::Outline(value) => *value,
            _ => unreachable!(),
        }
    }

    fn to_data_type_value(&self) -> OwnedDataType {
        OwnedDataType::Outline(Box::new(self.clone()))
    }

    fn to_data_type_ref(&self) -> TypeRef {
        TypeRef::Outline(self)
    }
}
