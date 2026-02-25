use serde::{Deserialize, Serialize};

use crate::asset::NodeIndex;

#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reference {
    /// The Standard Reference gets constructed by default and holds a reference to
    /// the current AssetProcessor.
    Standard {
        node: NodeIndex,
        socket: u8,
    },
    /// An AssetProcessor is responsible to map any mutable input it receives to an external one.
    /// This preserves the original resource
    External {
        //node: NodeIndex,
        socket: u8,
    },
    /// An AssetProcessor is responsible to map Standard references in DataTypes to Internal ones.
    Internal {
        node: NodeIndex,
        index: u8,
    },

    ExternalProperty {
        index: u8,
    },
    Property {
        node: NodeIndex,
        index: u8,
    },

    ///Invalid State should not be reached other than after default construction
    #[default]
    Uninitialized,
}

impl Reference {
    pub(crate) fn node(&self) -> NodeIndex {
        match self {
            Reference::Standard { node, .. } => *node,
            Reference::Internal { node, .. } => *node,
            Reference::ExternalProperty { .. } => unreachable!(),
            Reference::External { .. } => unreachable!(),
            Reference::Uninitialized => unreachable!(),
            Reference::Property { node, .. } => *node,
        }
    }
    pub(crate) fn index(&self) -> u8 {
        match self {
            Reference::Standard { socket, .. } => *socket,
            Reference::Property { index, .. } => *index,
            Reference::Uninitialized => todo!(),
            Reference::External { .. } => todo!(),
            Reference::Internal { index, .. } => *index,
            Reference::ExternalProperty { index } => todo!(),
        }
    }
}
