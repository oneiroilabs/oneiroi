use std::{collections::HashMap, num::NonZeroU32, time::Instant};

use nom::Mode;
use petgraph::{
    Directed,
    Direction::{self, Incoming, Outgoing},
    Graph,
    dot::Dot,
    matrix_graph::node_index,
    prelude::StableGraph,
    visit::{
        Bfs, Data, EdgeRef, IntoEdgeReferences, IntoNodeReferences, NodeFiltered, NodeRef, Topo,
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    data_types::{DataTypeInstance, DataTypeType, Property, PropertyMetadata},
    operations::{Nodes, Operation, PropertyInterface},
};

use super::{Asset, Dependency, NodeIndex, NodeMetadata};
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct InstanceNode {
    //operation: Nodes,
    //maybe also get rid of it all together
    computation_time: Option<NonZeroU32>,
}

impl Into<InstanceNode> for NodeMetadata {
    fn into(self) -> InstanceNode {
        InstanceNode {
            //operation: self.operation,
            computation_time: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct InstanceDependency;

#[derive(Debug)]
pub(super) struct AssetTemplate {
    //The graph where all the nodes that have to be copied to instance
    dynamic_graph: StableGraph<(InstanceNode, Nodes), InstanceDependency, Directed, u16>,
    //The graph where all static nodes live
    shared_graph: Graph<(InstanceNode, Nodes), InstanceDependency, Directed, u16>,

    //const_cache: Maybe make this a node type or something
    constant_cache: HashMap<NodeIndex, Vec<DataTypeInstance>>,

    exposed_property_meta: Vec<PropertyMetadata>,
    exposed_property_index_name: HashMap<String, (NodeIndex, String)>,
    //exposed_property_instances: Vec<DataTypeInstance>,
    //Idk about this yet
    //input_sockets: Vec<PropertyMetadata>,
    outputs: Vec<(NodeIndex, u8, DataTypeType)>,
    inputs: Vec<(NodeIndex, u8, DataTypeType)>,

    //TODO type for all
    //dynamic_nodes: Vec<NodeIndex>,
    //maybe static entries
    //static_blocks: HashMap<NodeIndex, u8>,
    generation: u16,
}

impl AssetTemplate {
    pub(super) fn generate(asset: &Asset) -> Self {
        #[cfg(debug_assertions)]
        let computation_instant = Instant::now();
        let graph = &asset.graph;
        #[cfg(debug_assertions)]
        println!("{:#?}", Dot::new(graph));

        //General Safety: Each Graph gets constructed with the Expose node at Index 0 which is unique among the graph.

        enum NodeInfo {
            Const,
            Static,
            Dynamic,
            Orphan,
        }

        let mut node_tracker: HashMap<NodeIndex, NodeInfo> =
            HashMap::with_capacity(graph.node_count());

        for index in graph.node_indices() {
            node_tracker.insert(index, NodeInfo::Const);
        }

        //find exposed nodes
        let exposed_nodes = graph.neighbors_directed(NodeIndex::from(0), Incoming);
        //Mark exposed nodes as dynamic
        exposed_nodes.clone().for_each(|n| {
            node_tracker.insert(n, NodeInfo::Dynamic);
        });

        //TODO this should be calculating if the exposed properties have references incoming to them and in turn the source nodes must also be marked as dynamic since they require recomputation
        let node_edges = graph
            .edge_references()
            .filter(|e| e.weight().is_node())
            .map(|e| (e.source(), e.target(), e.weight()));

        let graph_to_traverse =
            StableGraph::<(), Dependency, Directed, u16>::from_edges(node_edges);

        for node in exposed_nodes {
            let exposed_connection_properties = graph
                .edges_directed(node, Outgoing)
                .filter(|e| e.weight().is_exposed())
                .map(|e| e.weight().get_target_property())
                .collect::<Vec<_>>();

            let mut outgoing_node_edges = graph_to_traverse
                .edges_directed(node, Outgoing)
                .filter(|e| e.weight().is_exposed());
            for connection in exposed_connection_properties {
                if let Some(edge) =
                    outgoing_node_edges.find(|e| e.weight().get_target_property() == connection)
                {
                    node_tracker.insert(edge.target(), NodeInfo::Dynamic);
                }
            }
        }

        // get all the nodes which provide dynamic things
        // only the case for properties
        let dyn_nodes: Vec<NodeIndex> = node_tracker
            .iter()
            .filter(|(_, v)| matches!(v, NodeInfo::Dynamic))
            .map(|(k, _)| *k)
            .collect::<Vec<_>>();

        //TODO filter graph to only include node connections
        for node in &dyn_nodes {
            let mut bfs = Bfs::new(graph, *node);
            while let Some(node) = bfs.next(graph) {
                if let Some(tracked) = node_tracker.get(&node) {
                    if matches!(tracked, NodeInfo::Dynamic) {
                        continue;
                    }
                }
                node_tracker.insert(node, NodeInfo::Static);
            }
        }

        //const evaluation

        //TODO bake propertiy refs
        let mut constant_cache: HashMap<NodeIndex, Vec<DataTypeInstance>> = HashMap::new();

        let const_graph = NodeFiltered::from_fn(graph, |f| {
            node_tracker.contains_key(&f)
                && !graph[f].1.borrow().is_output_node()
                && f != NodeIndex::from(0)
        });
        let mut graph_to_traverse = Topo::new(&const_graph);

        while let Some(node) = graph_to_traverse.next(&const_graph) {
            //println!("HUH  {:?}", node);
            let connections = graph.edges_directed(node, Incoming);
            let mut dependencies = Vec::new();
            for edge in connections {
                //TODO make this fill all sockets
                //can access edge.source and .weight
                let computed_ref = &constant_cache[&edge.source()][0];
                dependencies.push(computed_ref)
            }
            constant_cache.insert(node, graph[node].1.borrow().compute(dependencies));
        }
        //let sorted = toposort(graph, None).expect("This is an error");

        /* let constant_leaves = graph.externals(petgraph::Direction::Incoming);
        let const_inputs = constant_leaves
            .filter(|n| !matches!(*graph[*n].1.borrow(), Nodes::SocketInput(_)))
            .collect::<Vec<_>>();
        for node in const_inputs {
            if let Some(neighbor) = graph
                .neighbors_directed(node, Direction::Outgoing)
                .find(|n| graph[*n].1.borrow().is_output_node())
            {
                let edge = graph
                    .edges_connecting(node, neighbor)
                    .nth(0)
                    .unwrap()
                    .weight();
                //TODO
                constant_cache.insert(neighbor, graph[node].1.borrow().compute(vec![]));
            }
        } */

        let dynamic_graph: StableGraph<(InstanceNode, Nodes), InstanceDependency, Directed, u16> =
            graph.filter_map(
                |idx, n| {
                    if dyn_nodes.iter().any(|dyn_index| *dyn_index == idx) {
                        Some((
                            InstanceNode {
                                computation_time: None,
                            },
                            n.1.borrow().clone(),
                        ))
                    } else {
                        None
                    }
                },
                |idx, e| Some(InstanceDependency {}),
            );

        let exposed_connections = graph.edges_directed(NodeIndex::from(0), Incoming); //.map(|e| e.weight());

        let mut exposed_property_meta = Vec::new();
        let mut exposed_property_index_name = HashMap::new();
        for connection in exposed_connections {
            let mut meta = graph[connection.source()]
                .1
                .borrow()
                .try_get_property(connection.weight().get_target_property())
                .unwrap()
                .get_property_meta();
            //TODO this should in theory already be set
            meta.set_name(connection.weight().get_exposed_name());
            //meta.set_default(DataTypeInstance::Float(Property::new(2.0)));
            exposed_property_meta.push(meta);
            exposed_property_index_name.insert(
                connection.weight().get_exposed_name().to_string(),
                (
                    connection.source(),
                    connection.weight().get_target_property().to_string(),
                ),
            );
        }

        /* let mut hash_map = HashMap::new();
        for node in new_graph.node_indices() {
            hash_map.insert(node, RefCell::new(None::<OneiroiMesh>));
            } */

        //TODO this needs to be computed from the nodes
        //let mut sockets = Vec::new();
        /* sockets.push(SocketInfo {
        color: Vec3::ZERO,
        socket_type: PropertyType::Mesh,
        }); */

        let mut outputs = vec![];

        let output_nodes = graph
            .externals(Outgoing)
            .filter(|n| graph[*n].1.borrow().is_output_node())
            .collect::<Vec<_>>();

        //println!("{:?}", output_nodes);

        for node in output_nodes {
            for edge in graph
                .edges_directed(node, Incoming)
                .filter(|edge| edge.weight().is_connection())
            {
                outputs.push((
                    edge.source(),
                    edge.weight().port_from(),
                    graph[edge.source()].1.borrow().get_output_sockets()
                        [edge.weight().port_from() as usize],
                ));
            }
        }

        //println!(" WHAT {:?}", outputs);

        #[cfg(debug_assertions)]
        println!(
            "Internal template computation took {} nanoseconds",
            computation_instant.elapsed().as_nanos()
        );
        AssetTemplate {
            dynamic_graph,

            shared_graph: Graph::default(), //TODO soon

            constant_cache,
            //temp_vec: sorted,
            //computed_cache: hash_map,
            exposed_property_meta,
            exposed_property_index_name,

            inputs: Vec::new(),
            outputs,
            //dynamic_nodes: Vec::new(),
            //static_blocks: HashMap::new(),
            generation: asset.generation,
        }
    }

    #[cfg(not(feature = "only_runtime"))]
    pub fn is_recomputation_needed(&self, generation: u16) -> bool {
        self.generation != generation
    }

    #[cfg(not(feature = "only_runtime"))]
    pub fn get_generation(&self) -> u16 {
        self.generation
    }

    /* pub fn get_outputs(&self) -> Vec<DataTypeInstance> {
        self.constant_cache[&(NodeIndex::from(1u16), 0)].clone()
    } */

    pub fn outputs(&self) -> impl Iterator<Item = (NodeIndex, u8, DataTypeType)> {
        self.outputs.iter().copied()
    }

    pub fn get_constant(&self, id: (NodeIndex, u8)) -> DataTypeInstance {
        self.constant_cache.get(&id.0).unwrap()[id.1 as usize].clone()
    }

    pub fn get_properties(&self) -> Vec<PropertyMetadata> {
        self.exposed_property_meta.clone()
    }

    //TODO optimize
    pub fn get_property_index_name(&self, property_name: &str) -> (NodeIndex, String) {
        self.exposed_property_index_name
            .get(property_name)
            .unwrap()
            .clone()
    }

    pub fn get_dynamic_graph(
        &self,
    ) -> StableGraph<(InstanceNode, Nodes), InstanceDependency, Directed, u16> {
        self.dynamic_graph.clone()
    }
}
