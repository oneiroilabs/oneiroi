use std::fmt::Debug;

use crate::type_system::{
    trait_types::{TraitType, TraitTypeKind},
    variants::TypeRef,
};

pub trait SequentialSample: Debug {}

impl<'a> TraitType<'a> for &'a dyn SequentialSample {
    const TRAIT_TYPE_KIND: TraitTypeKind = TraitTypeKind::SequentialSample;

    fn get_type_ref(value: TypeRef<'a>) -> Self {
        match value {
            TypeRef::SequentialSample(value) => value,
            _ => unreachable!(),
        }
    }
}
