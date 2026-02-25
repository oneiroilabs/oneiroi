use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use editable::EditableAsset;
use fixedbitset::FixedBitSet;
pub use glam::Vec2;
use instance::AssetInstance;
use petgraph::{Directed, prelude::StableGraph};
use runtime::RuntimeAsset;
use rustc_hash::FxBuildHasher;
use serde::{Deserialize, Serialize};
use template::AssetTemplate;

use petgraph::graph::EdgeIndex as InternalEdgeIndex;
use petgraph::graph::NodeIndex as InternalNodeIndex;

//reexsocketing for integrations
pub type NodeIndex = InternalNodeIndex<u16>;
pub type EdgeIndex = InternalEdgeIndex<u16>;

use crate::nodes::Nodes;
use crate::property::script::Script;
use crate::type_system::OwnedDataType;
use crate::type_system::Reference;
use crate::type_system::data_types::TypeDescriptor;

pub mod editable;
pub mod instance;
pub mod runtime;
pub mod server;
pub mod template;
#[cfg(test)]
pub mod test;

// The Graph that holds the connection state of the Asset
pub(crate) type OneiroiGraph = StableGraph<(NodeMetadata, Nodes), Connection, Directed, u16>;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub(crate) enum Connection {
    //The standard connection between nodes storing the input and output sockets.
    Socket { source: u8, target: u8 },
    // Connects two properties of different nodes
    Property { source: u8, target: u8 },
}

impl Connection {
    #[inline]
    pub(crate) fn is_socket(self) -> bool {
        match self {
            Connection::Socket { .. } => true,
            Connection::Property { .. } => false,
        }
    }

    #[inline]
    pub(crate) fn is_property(self) -> bool {
        match self {
            Connection::Socket { .. } => false,
            Connection::Property { .. } => true,
        }
    }

    #[inline]
    pub(crate) fn target(self) -> u8 {
        match self {
            Connection::Socket { target, .. } => target,
            Connection::Property { target, .. } => target,
        }
    }

    #[inline]
    pub(crate) fn source(self) -> u8 {
        match self {
            Connection::Socket { source, .. } => source,
            Connection::Property { source, .. } => source,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeMetadata {
    name: String,
    postition: Vec2,
}
//TODO mark these methods as editor only most likely
impl NodeMetadata {
    pub fn new(/* name: String, */ position: Vec2) -> NodeMetadata {
        NodeMetadata {
            name: "".into(),
            postition: position,
        }
    }

    pub fn empty() -> NodeMetadata {
        NodeMetadata {
            name: "".into(),
            postition: Vec2::new(0.0, 0.0),
        }
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name
    }

    pub fn get_position(&self) -> Vec2 {
        self.postition
    }

    pub fn set_position(&mut self, position: Vec2) {
        self.postition = position
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EmbeddedAsset {
    Internal(Asset, Vec<NodeIndex>),
    //TODO add supsocket for external assets
    External(AssetReference, Vec<NodeIndex>),
}
impl EmbeddedAsset {
    fn graph_nodes(&self) -> Vec<NodeIndex> {
        match self {
            EmbeddedAsset::Internal(_, items) => items.clone(),
            EmbeddedAsset::External(_, items) => items.clone(),
        }
    }

    fn get_template(&self) -> Arc<AssetTemplate> {
        match self {
            EmbeddedAsset::Internal(asset, _) => asset.get_template(),
            EmbeddedAsset::External(asset_reference, _) => todo!(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetReference {
    path: String,
    uid: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Asset {
    Editable(EditableAsset),
    Runtime(RuntimeAsset),
}

impl Default for Asset {
    fn default() -> Self {
        Asset::Editable(EditableAsset::default())
    }
}

impl Asset {
    pub fn get_instance(&self) -> AssetInstance {
        match self {
            Asset::Editable(edit_asset) => edit_asset.get_instance(),
            Asset::Runtime(runtime_asset) => todo!(),
        }
    }

    pub fn get_edit_mut(&mut self) -> &mut EditableAsset {
        match self {
            Asset::Editable(edit_asset) => edit_asset,
            Asset::Runtime(_) => unimplemented!(),
        }
    }

    pub fn get_editable(&self) -> &EditableAsset {
        match self {
            Asset::Editable(edit_asset) => edit_asset,
            Asset::Runtime(_) => unimplemented!(),
        }
    }
}

/// A AssetBase defines the common methods required to produce a Template which is later
/// used as a common base for instances of given Asset.
pub(crate) trait AssetBase {
    /// Get template is encouraged to be cached by the implementor since this
    /// method can be called multible times, is expensive and must be deterministic.
    /// TODO maybe this method doesnt belong in this trait
    fn get_template(&self) -> Arc<AssetTemplate>;

    /// The SocketInput Node registeres a propagating input handled by the Processor.
    /// Since it is a intrsinsic Node providing custom functionality it can be Queried specially.
    /// Every Node reachable from such a Input is automatically not const anymore and atleast static.
    fn nodes_reachable_from_respective_input(&self) -> Box<[FixedBitSet]>;

    fn get_node_dependencies(&self, index: NodeIndex) -> Box<[Reference]>;

    fn is_node_input(&self) -> FixedBitSet;
    fn is_node_output(&self) -> FixedBitSet;

    fn get_node_map(
        &self,
        dynamic_nodes: &FixedBitSet,
    ) -> HashMap<NodeIndex, (Box<[Reference]>, Nodes), FxBuildHasher>;

    fn get_const_cache(
        &self,
        dynamic_nodes_without_outputs: &FixedBitSet,
    ) -> HashMap<NodeIndex, Box<[OwnedDataType]>, FxBuildHasher>;

    fn get_outputs_and_info(
        &self,
        output_nodes: &FixedBitSet,
    ) -> (Box<[Reference]>, Box<[TypeDescriptor]>);
    fn get_input_info(&self, input_nodes: &FixedBitSet) -> Option<Box<[TypeDescriptor]>>;

    fn get_toposort(&self, filter_mask: &FixedBitSet) -> Vec<NodeIndex>;

    fn get_scripts(&self) -> impl Iterator<Item = (&Reference, &Script)>;
}

impl AssetBase for Asset {
    fn get_template(&self) -> Arc<AssetTemplate> {
        match self {
            Asset::Editable(edit_asset) => edit_asset.get_template(),
            Asset::Runtime(runtime_asset) => todo!(),
        }
    }

    fn nodes_reachable_from_respective_input(&self) -> Box<[FixedBitSet]> {
        todo!()
    }

    /* fn get_node_graph(&self) -> &OneiroiGraph {
        match self {
            Asset::Edit(edit_asset) => edit_asset.get_node_graph(),
            Asset::Runtime(runtime_asset) => todo!(),
        }
    } */

    fn get_node_dependencies(&self, index: NodeIndex) -> Box<[Reference]> {
        match self {
            Asset::Editable(edit_asset) => edit_asset.get_node_dependencies(index),
            Asset::Runtime(runtime_asset) => todo!(),
        }
    }

    fn get_scripts(&self) -> impl Iterator<Item = (&Reference, &Script)> {
        match self {
            Asset::Editable(edit_asset) => edit_asset.get_scripts(),
            Asset::Runtime(runtime_asset) => todo!(),
        }
    }

    fn get_node_map(
        &self,
        dynamic_nodes: &FixedBitSet,
    ) -> HashMap<NodeIndex, (Box<[Reference]>, Nodes), FxBuildHasher> {
        todo!()
    }

    fn get_toposort(&self, nodes_to_process: &FixedBitSet) -> Vec<NodeIndex> {
        todo!()
    }

    fn is_node_input(&self) -> FixedBitSet {
        todo!()
    }

    fn is_node_output(&self) -> FixedBitSet {
        todo!()
    }

    fn get_const_cache(
        &self,
        dynamic_nodes_without_outputs: &FixedBitSet,
    ) -> HashMap<NodeIndex, Box<[OwnedDataType]>, FxBuildHasher> {
        todo!()
    }
    fn get_outputs_and_info(
        &self,
        output_nodes: &FixedBitSet,
    ) -> (Box<[Reference]>, Box<[TypeDescriptor]>) {
        todo!()
    }

    fn get_input_info(&self, input_nodes: &FixedBitSet) -> Option<Box<[TypeDescriptor]>> {
        todo!()
    }
}
