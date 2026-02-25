use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use fixedbitset::FixedBitSet;
use glam::Vec2;
use glam::Vec3;
use itertools::Itertools;
use petgraph::Direction::{self, Incoming, Outgoing};
use petgraph::algo::toposort;
use petgraph::visit::{Bfs, EdgeRef, NodeFiltered, Topo};
use rustc_hash::FxBuildHasher;
use serde::{Deserialize, Serialize};

use super::AssetBase;
use super::EdgeIndex;
use super::NodeIndex;
use super::OneiroiGraph;
use super::instance::AssetInstance;

use crate::nodes::Node;
use crate::nodes::Nodes;
use crate::nodes::PropertyInterface;
use crate::nodes::SocketInterface;
use crate::nodes::SocketMetadata;
use crate::nodes::StaticNodeMetadata;
use crate::property::PropertyMetadata;
use crate::property::script::Script;
use crate::type_system::Reference;
use crate::type_system::data_types::TypeDescriptor;
use crate::type_system::{OwnedDataType, TypeRef};

use super::AssetTemplate;
use super::Connection;
use super::EmbeddedAsset;
use super::NodeMetadata;

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct AssetMetadata {
    //name: String,
    position: Vec2,
}

#[derive(Deserialize)]
struct EditableAssetDeserializeProxy {
    graph: OneiroiGraph,

    exposed_property_order: Vec<Reference>,
    embedded_assets: Vec<EmbeddedAsset>,
    property_scripts: HashMap<Reference, Script, FxBuildHasher>,
}

impl From<EditableAssetDeserializeProxy> for EditableAsset {
    fn from(mut value: EditableAssetDeserializeProxy) -> Self {
        for asset in &mut value.embedded_assets {
            for node in asset.graph_nodes() {
                value.graph[node]
                    .1
                    .get_embedded_instance()
                    .set_template(asset.get_template());
            }
        }

        EditableAsset {
            //metadata: value.metadata,
            graph: value.graph,

            exposed_property_order: value.exposed_property_order,
            embedded_assets: value.embedded_assets,
            template: OnceLock::new(),
            property_scripts: value.property_scripts,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(from = "EditableAssetDeserializeProxy")]
pub struct EditableAsset {
    graph: OneiroiGraph,

    property_scripts: HashMap<Reference, Script, FxBuildHasher>,

    //TODO
    //These Strings are in order of the exposed property keys and get handled when modifying the graph
    exposed_property_order: Vec<Reference>,

    embedded_assets: Vec<EmbeddedAsset>,

    #[serde(skip)]
    template: OnceLock<Arc<AssetTemplate>>,
}

//Takes care of providing a default cube with output
#[cfg(feature = "editor")]
impl Default for EditableAsset {
    fn default() -> Self {
        let mut asset = Self {
            //metadata: Default::default(),
            graph: Default::default(),
            embedded_assets: Default::default(),
            template: Default::default(),
            exposed_property_order: Default::default(),
            property_scripts: Default::default(),
        };

        _ = asset.graph.add_node((NodeMetadata::empty(), Nodes::Expose));

        //add default cube
        let n1 = asset.add_node("Output");

        let n2 = asset.add_node("Box");
        let node = &mut asset.graph[n2];
        node.0.set_position(Vec2 { x: -160., y: 0. });

        _ = asset.try_add_node_connection(n2, 0, n1, 0);

        asset
    }
}

impl EditableAsset {
    //TODO this should theorethically go in the trivial helper trait
    #[cfg(feature = "editor")]
    pub fn add_node(&mut self, alias: &str) -> NodeIndex {
        let node = match alias {
            "EmbeddedAsset" => return self.add_subgraph(),
            _ => Nodes::from_alias(alias),
        };

        let mut meta = NodeMetadata::empty();
        meta.set_name(alias.into());
        self.graph.add_node((meta, node))
    }

    //TODO this should theorethically go in the trivial helper trait
    #[cfg(feature = "editor")]
    pub fn add_subgraph(&mut self) -> NodeIndex {
        let asset = EditableAsset::default();
        let instance = asset.get_instance();
        let mut meta = NodeMetadata::empty();
        meta.set_name("EmbeddedAsset".into());
        let node = self
            .graph
            .add_node((meta, Nodes::EmbeddedAsset(Box::new(instance))));
        self.embedded_assets.push(EmbeddedAsset::Internal(
            super::Asset::Editable(asset),
            vec![node],
        ));
        node
    }

    #[cfg(feature = "editor")]
    pub fn is_connection_allowed(
        &self,
        from: NodeIndex,
        from_socket: u8, //TODO maybe distinct type dont know yet how it plays out with parameter
        to: NodeIndex,
        to_socket: u8, //TODO maybe distinct type dont know yet how it plays out with parameter
    ) -> bool {
        use petgraph::Direction::Incoming;

        //TODO this validation is flawed
        self.graph.edges_directed(to, Incoming).any(|er| {
            if er.weight().is_socket() {
                er.weight().target() == to_socket
            } else {
                false
            }
        })
    }

    #[cfg(feature = "editor")]
    pub fn get_nodes(&self) -> Vec<NodeIndex> {
        //we skip the first node since we know its the exsocket node
        self.graph.node_indices().skip(1).collect()
    }

    /// Get all Node connections that exist in the Asset.
    /// e.g. to reconstruct them on load
    #[cfg(feature = "editor")]
    pub fn get_node_connections(&self) -> Vec<EdgeIndex> {
        self.graph
            .edge_indices()
            .filter(|id| self.graph.edge_weight(*id).unwrap().is_socket())
            .collect()
    }

    #[cfg(feature = "editor")]
    /// Not yet sure how this is going to work out
    pub fn get_connection(&self, index: EdgeIndex) -> (u8, u8) {
        (self.graph[index].source(), self.graph[index].target())
    }

    #[cfg(feature = "editor")]
    /// Not yet sure how this is going to work out
    pub fn get_edge_endpoints(&self, index: EdgeIndex) -> (NodeIndex, NodeIndex) {
        self.graph
            .edge_endpoints(index)
            .expect("Supplied a edge index that is out of bounds")
    }

    pub fn get_instance(&self) -> AssetInstance {
        AssetInstance::new(
            self.template
                .get_or_init(|| Arc::new(AssetTemplate::transform(self))),
        )
    }
}

#[derive(Debug)]
pub enum TrivialError {
    NodeNotFound(NodeIndex),
    PropertyNotFound(String),
    PropertyValidationError(),
}

// This helper trait is responsible for all _trivial_ operations that dont put Graph Validity at risk
//TOTO exact name TBD
pub trait AssetEditorMethods {
    fn get_node_properties(&self, node_index: NodeIndex) -> Box<[PropertyMetadata]>;
    fn try_get_node_property(
        &self,
        node_index: NodeIndex,
        property: &str,
    ) -> Result<TypeRef, TrivialError>;
    fn try_set_node_property(
        &mut self,
        node_index: NodeIndex,
        property: &str,
        value: OwnedDataType,
    ) -> Result<(), NodeError>;

    fn try_set_node_position(
        &mut self,
        node_index: NodeIndex,
        position: Vec2,
    ) -> Result<(), NodeError>;
    fn try_get_node_sockets(
        &self,
        node_index: NodeIndex,
    ) -> Result<(Box<[SocketMetadata]>, Box<[SocketMetadata]>), NodeError>;
    fn try_get_node_metadata(&self, node_index: NodeIndex) -> Result<NodeMetadata, NodeError>;
    fn try_get_node_static_metadata(
        &self,
        node_index: NodeIndex,
    ) -> Result<StaticNodeMetadata, NodeError>;

    fn get_embedded_asset_nodes(&self) -> Vec<NodeIndex>;
}

impl AssetEditorMethods for EditableAsset {
    fn get_node_properties(&self, node_index: NodeIndex) -> Box<[PropertyMetadata]> {
        self.graph[node_index].1.get_properties()
    }

    fn try_get_node_property(
        &self,
        node_index: NodeIndex,
        property: &str,
    ) -> Result<TypeRef, TrivialError> {
        self.graph[node_index]
            .1
            .try_get_property(property)
            .map_err(|err| TrivialError::NodeNotFound(node_index))
    }

    fn try_set_node_property(
        &mut self,
        node_index: NodeIndex,
        property: &str,
        value: OwnedDataType,
    ) -> Result<(), NodeError> {
        self.graph[node_index].1.try_set_property(property, value);
        Ok(())
    }

    fn try_set_node_position(
        &mut self,
        node_index: NodeIndex,
        position: Vec2,
    ) -> Result<(), NodeError> {
        //TODO error handling
        self.graph[node_index].0.set_position(position);
        Ok(())
    }

    //TODO this is bad
    fn try_get_node_sockets(
        &self,
        node_index: NodeIndex,
    ) -> Result<(Box<[SocketMetadata]>, Box<[SocketMetadata]>), NodeError> {
        let mut io = (vec![], vec![]);
        for type_descriptor in self.graph[node_index].1.get_input_sockets() {
            io.0.push(SocketMetadata {
                type_descriptor,
                name: None,
                color: Vec3::ZERO,
                documentation: None,
            });
        }
        for type_descriptor in self.graph[node_index].1.get_output_sockets() {
            io.1.push(SocketMetadata {
                type_descriptor,
                name: None,
                color: Vec3::ZERO,
                documentation: None,
            });
        }
        Ok((io.0.into_boxed_slice(), io.1.into_boxed_slice()))
    }

    fn try_get_node_metadata(&self, node_index: NodeIndex) -> Result<NodeMetadata, NodeError> {
        Ok(self.graph[node_index].0.clone())
    }

    fn try_get_node_static_metadata(
        &self,
        node_index: NodeIndex,
    ) -> Result<StaticNodeMetadata, NodeError> {
        Ok(self.graph[node_index].1.node_metadata())
    }

    fn get_embedded_asset_nodes(&self) -> Vec<NodeIndex> {
        let mut all_nodes = Vec::new();
        for asset in self.embedded_assets.iter() {
            match asset {
                EmbeddedAsset::Internal(_, items) => all_nodes.extend(items),
                EmbeddedAsset::External(_, _) => (),
            }
        }
        all_nodes
    }
}

#[derive(Debug)]
pub enum ScriptingError {
    TODO,
}

//TODO make a better name for that
#[derive(Debug)]
pub enum NodeError {
    NodeNotFound(NodeIndex),
    NodeInputMissing(),
    ExternalReferenceMissing(),
}

// This helper trait is responsible for all *non-trivial* operations that put Graph Validity at risk
//TODO exact name TBD
pub trait NonTrivialEditorAction {
    /// Adds a connection between 2 node sockets
    // This could also take in ContextUids idk yet
    fn try_add_node_connection(
        &mut self,
        from: NodeIndex,
        from_socket: u8, //TODO maybe distinct type dont know yet how it plays out with parameter
        to: NodeIndex,
        to_socket: u8, //TODO maybe distinct type dont know yet how it plays out with parameter
    ) -> Result<(), NodeError>;

    fn delete_node(&mut self, index: NodeIndex) -> Result<(), NodeError>;

    fn delete_connection(
        &mut self,
        from: NodeIndex,
        from_socket: u8, //TODO maybe distinct type dont know yet how it plays out with parameter
        to: NodeIndex,
        to_socket: u8,
    ) -> Result<(), NodeError>;
}

impl NonTrivialEditorAction for EditableAsset {
    fn try_add_node_connection(
        &mut self,
        from: NodeIndex,
        source: u8, //TODO maybe distinct type dont know yet how it plays out with parameter
        to: NodeIndex,
        target: u8, //TODO maybe distinct type dont know yet how it plays out with parameter
    ) -> Result<(), NodeError> {
        //TODO validation
        self.graph
            .add_edge(from, to, Connection::Socket { source, target });
        Ok(())
    }

    fn delete_node(&mut self, index: NodeIndex) -> Result<(), NodeError> {
        {
            //TODO error handling
            self.graph.remove_node(index);
            Ok(())
        }
    }

    fn delete_connection(
        &mut self,
        from: NodeIndex,
        from_socket: u8, //TODO maybe distinct type dont know yet how it plays out with parameter
        to: NodeIndex,
        to_socket: u8,
    ) -> Result<(), NodeError> {
        use petgraph::{Direction::Incoming, visit::EdgeRef};

        let connection = self.graph.edges_directed(to, Incoming).find(|er| {
            if er.weight().is_socket() {
                er.weight().target() == to_socket
            } else {
                false
            }
            //&& er.weight().socket_from() == from_socket
        });
        if let Some(edge) = connection {
            if self.graph.remove_edge(edge.id()).is_some() {
                return Ok(());
            }
        }
        Err(NodeError::NodeNotFound(to))
    }
}

pub trait ScriptingInterface {
    fn try_set_script(
        &mut self,
        node: NodeIndex,
        property: u8,
        script: String,
    ) -> Result<(), ScriptingError>;

    fn try_get_script(&self, node: NodeIndex, property: u8) -> Result<String, ScriptingError>;
}

impl ScriptingInterface for EditableAsset {
    fn try_set_script(
        &mut self,
        node: NodeIndex,
        property: u8,
        script: String,
    ) -> Result<(), ScriptingError> {
        println!("{script}");
        let resolver_type = self.graph[node]
            .1
            .try_get_property_metadata(property)
            .get_type();
        if let Ok(script) = Script::check(node, property, resolver_type, &script, self) {
            self.property_scripts.insert(
                Reference::Property {
                    node,
                    index: property,
                },
                script,
            );
            Ok(())
        } else {
            Err(ScriptingError::TODO)
        }
    }

    fn try_get_script(&self, node: NodeIndex, property: u8) -> Result<String, ScriptingError> {
        if let Some(script) = self.property_scripts.get(&Reference::Property {
            node,
            index: property,
        }) {
            Ok(script.get_content().to_string())
        } else {
            Err(ScriptingError::TODO)
        }
    }
}

impl AssetBase for EditableAsset {
    fn get_template(&self) -> Arc<AssetTemplate> {
        self.template
            .get_or_init(|| Arc::new(AssetTemplate::transform(self)))
            .clone()
    }

    fn is_node_input(&self) -> FixedBitSet {
        let nodes = self
            .graph
            .externals(petgraph::Direction::Incoming)
            .filter(|n| self.graph[*n].1.is_input_node())
            .collect::<Box<_>>();
        let mut set = FixedBitSet::with_capacity(self.graph.node_count());
        for node in nodes {
            set.insert(node.index());
        }
        set
    }
    fn is_node_output(&self) -> FixedBitSet {
        let nodes = self
            .graph
            .externals(petgraph::Direction::Outgoing)
            .filter(|n| self.graph[*n].1.is_output_node())
            .collect::<Box<_>>();
        let mut set = FixedBitSet::with_capacity(self.graph.node_count());
        for node in nodes {
            set.insert(node.index());
        }
        set
    }

    fn get_toposort(&self, filter_mask: &FixedBitSet) -> Vec<NodeIndex> {
        let filtered_graph =
            NodeFiltered::from_fn(&self.graph, |f| filter_mask.contains(f.index() as usize));
        toposort(&filtered_graph, None).unwrap()
    }

    fn get_node_map(
        &self,
        dynamic_nodes: &FixedBitSet,
    ) -> HashMap<NodeIndex, (Box<[Reference]>, Nodes), FxBuildHasher> {
        let mut map =
            HashMap::with_capacity_and_hasher(dynamic_nodes.count_ones(..), FxBuildHasher);

        for node in dynamic_nodes.ones() {
            let node = (node as u16).into();
            let dependencies = self.get_node_dependencies(node);
            map.insert(node, (dependencies, self.graph[node].1.clone()));
        }
        map
    }

    fn nodes_reachable_from_respective_input(&self) -> Box<[FixedBitSet]> {
        // Find all nodes which are of type InputSocket
        //TODO accept this as input param since we already have a way to retrieve the input nodes.
        let input_nodes: Box<[NodeIndex]> = self
            .graph
            .externals(petgraph::Direction::Incoming)
            .filter(|n| self.graph[*n].1.is_input_node())
            .collect::<Box<_>>();

        //TODO the order is important and should be sorted into correct one

        // For each input socket we allocate one slice index.
        let mut node_bitsets = Box::new_uninit_slice(input_nodes.len());

        let node_count = self.graph.node_count();
        // Traverse each graph of reachable nodes in order to mark them
        for (index, node) in input_nodes.into_iter().enumerate() {
            let mut bits = FixedBitSet::with_capacity(node_count);

            let mut bfs = Bfs::new(&self.graph, node);
            while let Some(nx) = bfs.next(&self.graph) {
                bits.insert(nx.index());
            }
            node_bitsets[index].write(bits);
        }

        // SAFETY: All indices have been initialized in the loop  above
        unsafe { node_bitsets.assume_init() }
    }

    fn get_node_dependencies(&self, index: NodeIndex) -> Box<[Reference]> {
        let connections = self.graph.edges_directed(index, Direction::Incoming);
        //Since the edges could have wrong sequence of connections sort them
        let connections = connections.sorted_by_key(|d| d.weight().target());
        let mut dependencies = Vec::new();
        for edge in connections {
            if edge.weight().is_socket() {
                let source = edge.source();
                let socket = edge.weight().source();
                if self.graph[edge.source()].1.is_input_node() {
                    dependencies.push(Reference::External { socket })
                } else {
                    dependencies.push(Reference::Standard {
                        socket,
                        node: source,
                    })
                }
            }
        }
        dependencies.into_boxed_slice()
    }

    fn get_scripts(&self) -> impl Iterator<Item = (&Reference, &Script)> {
        self.property_scripts.iter()
    }

    fn get_const_cache(
        &self,
        dynamic_nodes_without_outputs: &FixedBitSet,
    ) -> HashMap<NodeIndex, Box<[OwnedDataType]>, FxBuildHasher> {
        let mut constant_cache: HashMap<NodeIndex, Box<[OwnedDataType]>, FxBuildHasher> =
            HashMap::with_capacity_and_hasher(
                dynamic_nodes_without_outputs.count_zeroes(..),
                FxBuildHasher,
            );

        let const_graph = NodeFiltered::from_fn(&self.graph, |f| {
            !dynamic_nodes_without_outputs.contains(f.index()) && f != NodeIndex::new(0)
        });
        let mut graph_to_traverse = Topo::new(&const_graph);

        while let Some(node) = graph_to_traverse.next(&const_graph) {
            let dependencies = self.get_node_dependencies(node);

            let computed = self.graph[node]
                .1
                .compute(Some(&dependencies), &constant_cache);

            constant_cache.insert(node, computed.into_iter().flatten().collect());
        }
        constant_cache
    }

    fn get_outputs_and_info(
        &self,
        output_nodes: &FixedBitSet,
    ) -> (Box<[Reference]>, Box<[TypeDescriptor]>) {
        //TODO remove
        //#[cfg(debug_assertions)]
        //println!("{:#?}", Dot::new(&self.graph));

        let output_nodes = output_nodes.ones().map(NodeIndex::new).collect::<Box<_>>();

        let mut outputs = Box::new_uninit_slice(output_nodes.len());
        let mut output_info = Box::new_uninit_slice(output_nodes.len());

        for (index, node) in output_nodes.iter().enumerate() {
            for edge in self.graph.edges_directed(*node, Incoming) {
                outputs[index].write(Reference::Standard {
                    node: edge.source(),
                    socket: edge.weight().source(),
                });
                output_info[index].write(
                    self.graph[edge.source()].1.get_output_sockets()
                        [edge.weight().source() as usize]
                        .clone(),
                );
            }
        }
        unsafe { (outputs.assume_init(), output_info.assume_init()) }
    }

    fn get_input_info(&self, input_nodes: &FixedBitSet) -> Option<Box<[TypeDescriptor]>> {
        let input_nodes = input_nodes.ones().map(NodeIndex::new).collect::<Box<_>>();

        //let mut inputs = Box::new_uninit_slice(input_nodes.len());
        let mut input_info = Box::new_uninit_slice(input_nodes.len());
        //println!("{:?}", output_nodes);

        for (index, node) in input_nodes.iter().enumerate() {
            let edge = self.graph.edges_directed(*node, Outgoing).nth(0).unwrap();

            input_info[index].write(
                self.graph[edge.target()].1.get_input_sockets()[edge.weight().target() as usize]
                    .clone(),
            );
        }

        unsafe { Some(input_info.assume_init()) }
    }
}
