use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, OnceLock, RwLock},
};

pub use glam::Vec2;
use instance::AssetInstance;
use itertools::Itertools;
use petgraph::{Directed, prelude::StableGraph};
use serde::{Deserialize, Serialize};
use template::AssetTemplate;

use petgraph::graph::EdgeIndex as InternalEdgeIndex;
use petgraph::graph::NodeIndex as InternalNodeIndex;

//reexporting for integrations
pub type NodeIndex = InternalNodeIndex<u16>;
pub type EdgeIndex = InternalEdgeIndex<u16>;

use crate::operations::{
    Nodes, control_flow::socket_output::SocketOutputV1, producers::r#box::BoxV1,
};

#[cfg(test)]
pub mod test;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Dependency {
    Connection([u8; 2]), //The standard connection between nodes
    This,                // For Multiprops like vec3
    Same,                // Reference to same Node
    Node {
        target_property: String,
        source_property: String,
    }, // Refernece to other Node
    Expose {
        name: String,
        target_property: String,
    }, // Marker to Expose in Designer Land
    Runtime {
        name: String,
        target_property: String,
    }, // Marker to Expose in User Land
}

/* #[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dependency {
    dep_type: DependencyType,
    //not sure about this one yet
    //prop_type: PropertyType,
    //Between which Sockets numbers the dependency is
    dep_sockets: Option<[u8; 2]>,
} */

impl Dependency {
    #[cfg(not(feature = "only_runtime"))]
    pub fn port_from(&self) -> u8 {
        match self {
            Dependency::Connection(ports) => ports[0],
            _ => panic!(),
        }
    }

    #[cfg(not(feature = "only_runtime"))]
    pub fn port_to(&self) -> u8 {
        match self {
            Dependency::Connection(ports) => ports[1],
            _ => panic!(),
        }
    }

    pub(super) fn is_exposed(&self) -> bool {
        match self {
            Dependency::Connection(_) => false,
            Dependency::This => false,
            Dependency::Same => false,
            Dependency::Node {
                target_property,
                source_property,
            } => false,
            Dependency::Expose {
                name,
                target_property: property,
            } => true,
            Dependency::Runtime {
                name,
                target_property: property,
            } => true,
        }
    }

    pub(super) fn get_exposed_name(&self) -> &str {
        match self {
            Dependency::Connection(_) => panic!(),
            Dependency::This => panic!(),
            Dependency::Same => panic!(),
            Dependency::Node {
                target_property,
                source_property,
            } => panic!(),
            Dependency::Expose {
                name,
                target_property,
            } => name,
            Dependency::Runtime {
                name,
                target_property,
            } => name,
        }
    }

    pub(super) fn is_node(&self) -> bool {
        matches!(
            self,
            Dependency::Node {
                target_property: _,
                source_property: _
            }
        )
    }

    pub(super) fn is_connection(&self) -> bool {
        matches!(self, Dependency::Connection(_))
    }

    pub(super) fn get_target_property(&self) -> &str {
        match self {
            Dependency::Connection(_) => panic!(),
            Dependency::This => panic!(),
            Dependency::Same => panic!(),
            Dependency::Node {
                target_property,
                source_property,
            } => target_property,
            Dependency::Expose {
                name,
                target_property,
            } => target_property,
            Dependency::Runtime {
                name,
                target_property,
            } => target_property,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeMetadata {
    name: String,
    postition: Vec2,
    #[serde(skip)]
    dirty: bool,
}
//TODO mark these methods as editor only most likely
impl NodeMetadata {
    pub fn new(/* name: String, */ position: Vec2) -> NodeMetadata {
        NodeMetadata {
            name: "".into(),
            postition: position,
            dirty: false,
        }
    }

    pub fn empty() -> NodeMetadata {
        NodeMetadata {
            name: "".into(),
            postition: Vec2::new(0.0, 0.0),
            dirty: false,
        }
    }

    pub fn set_dirty(&mut self) {
        self.dirty = true;
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

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EmbeddedAsset {
    Internal(Asset, Vec<NodeIndex>),
    //TODO add support for external assets
    External(AssetReference, Vec<NodeIndex>),
}
impl EmbeddedAsset {
    fn graph_nodes(&self) -> Vec<NodeIndex> {
        match self {
            EmbeddedAsset::Internal(_, items) => items.clone(),
            EmbeddedAsset::External(_, items) => items.clone(),
        }
    }

    fn get_template(&self) -> Arc<RwLock<AssetTemplate>> {
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

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AssetMetadata {
    //name: String,
    position: Vec2,
}

pub(crate) type OneiroiGraph =
    StableGraph<Rc<(RefCell<NodeMetadata>, RefCell<Nodes>)>, Dependency, Directed, u16>;

#[derive(Deserialize)]
struct AssetDeserializeProxy {
    metadata: AssetMetadata,
    graph: OneiroiGraph,
    //TODO
    //These Strings are in order of the exposed property keys and get handled when modifying the graph
    exposed_property_order: Vec<String>,
    sub_assets: Vec<EmbeddedAsset>,
}

impl AssetDeserializeProxy {
    fn init_properties(&self) -> OneiroiGraph {
        let mut graph = OneiroiGraph::with_capacity(self.graph.node_count(), 10);

        //TODO
        //for index in self.graph.node_indices() {}

        graph
    }
}

impl From<AssetDeserializeProxy> for Asset {
    fn from(mut value: AssetDeserializeProxy) -> Self {
        for asset in &mut value.sub_assets {
            for node in asset.graph_nodes() {
                value.graph[node]
                    .1
                    .borrow_mut()
                    .get_embedded_instance()
                    .set_template(asset.get_template());
            }
        }

        let property_graph = value.init_properties();

        Asset {
            metadata: value.metadata,
            graph: value.graph,
            property_graph,
            exposed_property_order: value.exposed_property_order,
            sub_assets: value.sub_assets,
            template: OnceLock::new(),
            generation: 0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(from = "AssetDeserializeProxy")]
pub struct Asset {
    metadata: AssetMetadata,

    graph: OneiroiGraph,
    #[serde(skip)]
    property_graph: OneiroiGraph,

    //TODO
    //These Strings are in order of the exposed property keys and get handled when modifying the graph
    exposed_property_order: Vec<String>,

    sub_assets: Vec<EmbeddedAsset>,

    #[serde(skip)]
    template: OnceLock<Arc<RwLock<AssetTemplate>>>,
    #[serde(skip)]
    generation: u16,
}

//Takes care of providing a default cube with output
impl Default for Asset {
    fn default() -> Self {
        let mut asset = Self {
            metadata: Default::default(),
            graph: Default::default(),
            property_graph: Default::default(),
            sub_assets: Default::default(),
            template: Default::default(),
            exposed_property_order: Vec::new(), //TODO
            generation: 0,
        };

        _ = asset.graph.add_node(Rc::new((
            RefCell::new(NodeMetadata::empty()),
            RefCell::new(Nodes::Expose),
        )));

        //add default cube
        let n1 = asset.add_node("Output");
        let n2 = asset.add_node("Box");

        _ = asset.add_node_connection(n2, 0, n1, 0);

        asset
    }
}

impl Asset {
    #[cfg(not(feature = "only_runtime"))]
    pub fn add_node(&mut self, alias: &str) -> NodeIndex {
        let node = match alias {
            "EmbeddedAsset" => return self.add_subgraph(),
            _ => Nodes::from_alias(alias),
        };
        self.graph.add_node(Rc::new((
            RefCell::new(NodeMetadata::empty()),
            RefCell::new(node),
        )))
    }

    #[cfg(not(feature = "only_runtime"))]
    pub fn add_subgraph(&mut self) -> NodeIndex {
        let asset = Asset::default();
        let instance = asset.get_instance();
        let mut meta = NodeMetadata::empty();
        meta.set_name("EmbeddedAsset".into());
        let node = self.graph.add_node(Rc::new((
            RefCell::new(meta),
            RefCell::new(Nodes::EmbeddedAsset(Box::new(instance))),
        )));
        self.sub_assets
            .push(EmbeddedAsset::Internal(asset, vec![node]));
        node
    }

    #[cfg(not(feature = "only_runtime"))]
    /// Adds a connection between 2 node sockets
    pub fn add_node_connection(
        &mut self,
        from: NodeIndex,
        from_port: u8, //TODO maybe distinct type dont know yet how it plays out with parameter
        to: NodeIndex,
        to_port: u8, //TODO maybe distinct type dont know yet how it plays out with parameter
    ) -> Result<(), ()> {
        //TODO validation
        self.graph
            .add_edge(from, to, Dependency::Connection([from_port, to_port]));
        Ok(())
    }
    #[cfg(not(feature = "only_runtime"))]
    pub fn allow_connection(
        &self,
        from: NodeIndex,
        from_port: u8, //TODO maybe distinct type dont know yet how it plays out with parameter
        to: NodeIndex,
        to_port: u8, //TODO maybe distinct type dont know yet how it plays out with parameter
    ) -> bool {
        use petgraph::Direction::Incoming;

        self.graph.edges_directed(to, Incoming).any(|er| {
            er.weight().is_connection() && er.weight().port_to() == to_port
            //&& er.weight().port_from() == from_port
        })
    }

    #[cfg(not(feature = "only_runtime"))]
    /// Allows to configure each Dependency option
    pub fn try_add_dependency(
        &mut self,
        from: NodeIndex,
        to: NodeIndex,
        dependency: Dependency,
    ) -> Result<(), ()> {
        //TODO validation
        self.graph.add_edge(from, to, dependency);
        Ok(())
    }

    #[cfg(not(feature = "only_runtime"))]
    pub fn get_nodes(&self) -> Vec<NodeIndex> {
        //we skip the first node since we know its the export node
        self.graph.node_indices().skip(1).collect()
    }

    #[cfg(not(feature = "only_runtime"))]
    /// Not yet sure how this is going to work out
    pub fn get_node(&self, index: NodeIndex) -> Rc<(RefCell<NodeMetadata>, RefCell<Nodes>)> {
        self.graph[index].clone()
    }

    /* #[cfg(not(feature = "only_runtime"))]
    /// Not yet sure how this is going to work out
    pub fn get_connections(&self) -> Vec<EdgeIndex<u16>> {
        //TODO filter out other dependencies and only get actual connections
        self.graph.edge_indices().collect()
    } */

    #[cfg(not(feature = "only_runtime"))]
    /// Not yet sure how this is going to work out
    pub fn get_node_connections(&self) -> Vec<EdgeIndex> {
        //TODO filter out other dependencies and only get actual connections
        self.graph
            .edge_indices()
            .filter(|e| {
                matches!(
                    self.graph.edge_weight(*e).unwrap(),
                    Dependency::Connection(_)
                )
            })
            .collect()
    }

    #[cfg(not(feature = "only_runtime"))]
    /// Not yet sure how this is going to work out
    pub fn get_connection(&self, index: EdgeIndex) -> Dependency {
        self.graph[index].clone()
    }

    #[cfg(not(feature = "only_runtime"))]
    pub fn delete_node(
        &mut self,
        index: NodeIndex,
    ) -> Result<Rc<(RefCell<NodeMetadata>, RefCell<Nodes>)>, ()> {
        self.graph.remove_node(index).ok_or(())
    }

    #[cfg(not(feature = "only_runtime"))]
    pub fn delete_connection(
        &mut self,
        from: NodeIndex,
        from_port: u8, //TODO maybe distinct type dont know yet how it plays out with parameter
        to: NodeIndex,
        to_port: u8,
    ) -> Result<(), ()> {
        use petgraph::{Direction::Incoming, visit::EdgeRef};

        let connection = self.graph.edges_directed(to, Incoming).find(|er| {
            er.weight().is_connection() && er.weight().port_to() == to_port
            //&& er.weight().port_from() == from_port
        });
        if let Some(edge) = connection {
            if self.graph.remove_edge(edge.id()).is_some() {
                return Ok(());
            }
        }
        Err(())
    }

    #[cfg(not(feature = "only_runtime"))]
    /// Not yet sure how this is going to work out
    pub fn get_edge_endpoints(&self, index: EdgeIndex) -> (NodeIndex, NodeIndex) {
        self.graph
            .edge_endpoints(index)
            .expect("Supplied a edge index that is out of bounds")
    }

    /* #[cfg(not(feature = "only_runtime"))]
    pub fn get_node(&self, index: NodeIndex<u16>) -> &(OneiroiNode, Nodes) {
        &self.graph[index]
    } */

    pub fn get_instance(&self) -> AssetInstance {
        AssetInstance::new(
            self.template
                .get_or_init(|| Arc::new(RwLock::new(AssetTemplate::generate(self)))),
        )
    }

    fn get_template(&self) -> Arc<RwLock<AssetTemplate>> {
        self.template
            .get_or_init(|| Arc::new(RwLock::new(AssetTemplate::generate(self))))
            .clone()
    }

    #[cfg(not(feature = "only_runtime"))]
    fn has_changed(&self) -> bool {
        self.graph
            .node_weights()
            .find(|n| n.0.borrow().is_dirty())
            .is_some()
    }

    #[cfg(not(feature = "only_runtime"))]
    pub fn recompute(&mut self) {
        if self.has_changed() {
            if let Ok(mut writer) = self
                .template
                .get()
                .expect("Cell should be initialized")
                .write()
            {
                *writer = AssetTemplate::generate(self);

                for node in self.graph.node_weights() {
                    node.0.borrow_mut().dirty = false;
                }
                self.generation += 1;
                println!("Graph Updated")
            }
        }
    }

    /* pub fn compute_instance(&self, instance: &mut AssetInstance) {
        instance.compute(input_sockets)
    } */

    //HUH WHAT IS THIS
    /* pub fn get_inputs(&self) -> AssetInstance {
        self.cached_export
            //TODO maybe not only pass the graph but whole Asset
            .get_or_init(|| AssetTemplate::init(&self.graph))
            .get_template()
    } */

    /* fn dependencies<'a>(
        &self,
        node: &NodeIndex<u16>,
        instance: &'a mut OneiroiDataInstance<'a>,
    ) {
        let neighbors = self.graph.neighbors_directed(*node, Incoming);
        //let mut ret: Vec<OneiroiDataInstance> = Vec::new();
        //let mut ret: OneiroiDataInstance = OneiroiDataInstance::Unit;
        for neighbor in neighbors {
            //ret =
            /* &* */
            *instance = OneiroiDataInstance::<'a>::Mesh(
                self.computed_node_mesh.borrow()[&neighbor].borrow(),
            );
        }
        /* if ret.is_empty() {
            ret.push(OneiroiDataInstance::Unit);
        } */
        //ret
        //OneiroiDataInstance::Unit
    } */

    /* pub fn render() -> OneiroiMesh {
        OneiroiBoxV1::default().compute()
    } */
}

pub mod instance;
pub mod template;

/* mod runtime {
    use petgraph::{Directed, Graph};

    #[derive(Debug)]
    struct RuntimeDependency {}

    #[derive(Debug)]
    struct RuntimeGraph<T> {
        graph: Graph<T, RuntimeDependency, Directed, u16>,
    }
} */
