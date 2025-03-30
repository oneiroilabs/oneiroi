use core::panic;
use std::{
    any::Any,
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Instant,
};

use petgraph::{
    Directed,
    Direction::Outgoing,
    Graph,
    prelude::StableGraph,
    visit::{EdgeFiltered, Topo},
};
use serde::{Deserialize, Serialize};

use crate::{
    asset::template::AssetTemplate,
    data_types::{DataTypeInstance, DataTypeType, PropertyMetadata},
    operations::{Operation, PropertyInterface, StaticNodeMetadata, producers::r#box::BoxV1},
};

use super::{
    Dependency, NodeIndex, Nodes,
    template::{InstanceDependency, InstanceNode},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetInstance {
    #[serde(skip)]
    graph: StableGraph<(InstanceNode, Nodes), InstanceDependency, Directed, u16>,

    properties: HashMap<String, DataTypeInstance>,
    //dynamic_nodes: Vec<(NodeIndex<u16>, Nodes)>,
    //TODO can be swapped with bit array
    dirty_nodes: HashMap<NodeIndex, bool>,

    //TODO dont know yet if this is necessary could be used for diffing inputs
    //outputs: Vec<NodeIndex<DataTypeInstance>>,
    //Tracks the internal SubAsset
    //asset_index: usize,

    //This is theoretically not optional but rather be injected on deserialization
    #[serde(skip)]
    template: Option<Arc<RwLock<AssetTemplate>>>,
    #[serde(skip)]
    generation: u16,

    #[serde(skip)]
    graph_node_cache: HashMap<NodeIndex, RefCell<Option<Vec<DataTypeInstance>>>>,
}

impl Clone for AssetInstance {
    fn clone(&self) -> Self {
        Self {
            properties: self.properties.clone(),
            graph: self.graph.clone(),
            // dynamic_nodes: self.dynamic_nodes.clone(),
            //output_nodes: self.output_nodes.clone(),
            template: self.template.clone(),
            generation: self.generation,
            graph_node_cache: self.graph_node_cache.clone(),
            dirty_nodes: self.dirty_nodes.clone(),
        }
    }
}

impl AssetInstance {
    pub(super) fn new(template: &Arc<RwLock<AssetTemplate>>) -> Self {
        //take ownership
        let template = template.clone();
        let dynamic_graph = template
            .try_read()
            .expect("Cant get read access to template")
            .get_dynamic_graph();
        let mut computed_cache = HashMap::with_capacity(dynamic_graph.node_count());
        for index in dynamic_graph.node_indices() {
            computed_cache.insert(index, RefCell::new(None));
        }

        let mut properties = HashMap::new();
        let mut dirty_nodes = HashMap::new();
        for prop in template.try_read().unwrap().get_properties() {
            properties.insert(prop.name().to_string(), prop.get_default().clone());

            let starting_node = template
                .read()
                .unwrap()
                .get_property_index_name(prop.name());

            dirty_nodes.insert(starting_node.0, true);
        }

        Self {
            properties,
            //output_sockets: output_sockets.iter().map(|s| SocketInstance {}).collect(),
            //asset_index: 0,
            graph: dynamic_graph,
            graph_node_cache: computed_cache,
            // output_nodes: Vec::new(),
            generation: template.clone().try_read().unwrap().get_generation(),
            template: Some(template),
            dirty_nodes,
        }
    }

    //#[cfg(not(feature = "only_runtime"))]
    /// It is the responsibility of the caller to compute after this returns true. Not doing so causes UB
    pub fn is_dirty(&mut self) -> bool {
        #[cfg(not(feature = "only_runtime"))]
        if let Ok(reader) = self
            .template
            .as_ref()
            .expect("has no template associated")
            .try_read()
        {
            if reader.is_recomputation_needed(self.generation) {
                self.generation = reader.get_generation();
                return true;
            }
        } else {
            eprintln!("Cant get a reader for the template. This is a bug");
        }
        false

        //TODO if local vars have changed recompute
        //println!("TODO recompute the graph from input changes")
    }
    /*  pub fn get_outputs(&self) -> Vec<PropertyInstance> {
        //TODO

        self.computed_cache
    } */

    pub(super) fn set_template(&mut self, template: Arc<RwLock<AssetTemplate>>) {
        self.template = Some(template)
    }

    //fn update_properties(&mut self, property_name: &str, value: &DataTypeInstance) {
    /* let starting_node = self
        .template
        .as_ref()
        .unwrap()
        .read()
        .unwrap()
        .get_property_node(property_name);
    self.graph[starting_node]
        .1
        .try_set_property(property_name, value.clone()); */
    /* let nodes = self
    .graph
    .edges_directed(starting_node, Outgoing)
    .filter(|e| {
        matches!(
            e.weight(),
            Dependency::Node {
                target_property,
                source_property
            }
        )
    }); */
    //}
}

impl PropertyInterface for AssetInstance {
    fn try_set_property(&mut self, property_name: &str, value: DataTypeInstance) -> Result<(), ()> {
        if let Some(property) = self.properties.get_mut(property_name) {
            debug_assert!(
                property.get_type() == value.get_type(),
                "Trying to set property with other type than expected"
            );
            *property = value.clone();
            println!("{:?}", value);
        } else {
            //println!("Property {property_name} not found");
            return Err(());
        }
        let starting_node = self
            .template
            .as_ref()
            .unwrap()
            .read()
            .unwrap()
            .get_property_index_name(property_name);

        self.dirty_nodes.insert(starting_node.0, true);

        let mhm = self.graph[starting_node.0]
            .1
            .try_set_property(&starting_node.1, value.clone());
        /*
        let nodes = self
                    .graph
                    .edges_directed(starting_node, Outgoing)
                    .filter(|e| {
                        matches!(
                            e.weight(),
                            Dependency::Node {
                                target_property,
                                source_property
                            }
                        )
                    }); */
        Ok(())
    }

    fn try_get_property(&self, property: &str) -> Result<DataTypeInstance, ()> {
        if let Some(property) = self.properties.get(property) {
            Ok(property.clone())
        } else {
            //println!("Property {property} not found");
            Err(())
        }
    }

    fn get_properties(&self) -> Vec<PropertyMetadata> {
        if let Ok(reader) = self.template.as_ref().unwrap().try_read() {
            reader.get_properties()
        } else {
            panic!("Couldnt obtain a reader from the Template! This is a bug")
        }
    }
}

impl Operation for AssetInstance {
    fn compute(&self, _input_sockets: Vec<&DataTypeInstance>) -> Vec<DataTypeInstance> {
        let computation_instant = Instant::now();

        let dirty_nodes_entrypoints = self
            .dirty_nodes
            .iter()
            .filter(|(_, v)| **v)
            .map(|v| v.0)
            .copied();
        //TODO actually filter
        // it ignores the edges that have a incoming edge so we need to remove that incoming edge
        // but only if no other dependant node also has a update leading to a wrong result
        let filtered_graph = EdgeFiltered::from_fn(&self.graph, |edge| true);
        let mut traversal_ordered = Topo::with_initials(&filtered_graph, dirty_nodes_entrypoints);
        while let Some(node) = traversal_ordered.next(&filtered_graph) {
            //println!("HUH  {:?}", node);
            *self.graph_node_cache[&node].borrow_mut() = Some(self.graph[node].1.compute(vec![]));
        }

        //TODO clear self.dirty_nodes

        println!(
            "Internal instance computation took {} nanoseconds",
            computation_instant.elapsed().as_nanos()
        );

        self.template
            .as_ref()
            .unwrap()
            .read()
            .unwrap()
            .outputs()
            .map(|out| {
                if let Some(cache) = self.graph_node_cache.get(&out.0) {
                    let cache = cache.borrow().as_ref().unwrap()[out.1 as usize].clone();
                    cache
                } else {
                    self.template
                        .as_ref()
                        .unwrap()
                        .read()
                        .unwrap()
                        .get_constant((out.0, out.1))
                }
            })
            .collect::<Vec<_>>()
    }

    fn static_metadata(&self) -> StaticNodeMetadata {
        StaticNodeMetadata { color: "#b45309" }
    }

    fn get_input_sockets(&self) -> Vec<DataTypeType> {
        //TODO
        vec![]
    }

    fn get_output_sockets(&self) -> Vec<DataTypeType> {
        self.template
            .as_ref()
            .unwrap()
            .read()
            .unwrap()
            .outputs()
            .map(|s| s.2)
            .collect()
    }
}
