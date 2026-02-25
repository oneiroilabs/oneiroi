mod mesh;
mod sequential_sample;

pub use mesh::{MeshMut0D, MeshMut1D, MeshMut2D};
pub use sequential_sample::SequentialSample;

use crate::type_system::variants::TypeRef;

pub trait TraitType<'a> {
    const TRAIT_TYPE_KIND: TraitTypeKind;
    fn get_type_ref(value: TypeRef<'a>) -> Self;
}

#[derive(Debug, PartialEq, Eq)]
pub enum TraitTypeKind {
    SequentialSample,
}
