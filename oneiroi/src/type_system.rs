use crate::type_system::{data_types::DataTypeKind, trait_types::TraitTypeKind};

pub mod data_types;
mod reference;
pub mod trait_types;
mod variants;

pub use reference::Reference;
pub use variants::OwnedDataType;
pub use variants::TypeRef;

enum TypeKind {
    Data(DataTypeKind),
    Trait(TraitTypeKind),
}
