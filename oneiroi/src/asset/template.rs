use std::{collections::HashMap, time::Instant};

use fixedbitset::FixedBitSet;

use rustc_hash::FxBuildHasher;

use crate::{
    nodes::{ContextProvider, Node, Nodes, PropertyInterface},
    property::PropertyMetadata,
    type_system::{
        OwnedDataType, Reference, TypeRef,
        data_types::{DataTypeKind, TypeDescriptor},
    },
};

use super::{AssetBase, NodeIndex};

/// Holds all shared state between instances of a particular Asset.
/// This allows multiple instances to be created much faster than without it.
#[derive(Debug)]
pub(crate) struct AssetTemplate {
    // All nodes which are dynamic have their Dependencies and configuraion baked here.
    dependency_node_map: HashMap<NodeIndex, (Box<[Reference]>, Nodes), FxBuildHasher>,
    // For each node index all other changing nodes are cached here.
    node_connectivity: Box<[FixedBitSet]>,

    // The Topological order of the graph to apply the node_connectivity to.
    topo_order: Vec<NodeIndex>,

    //All evaluations of constant nodes get cached here.
    constant_cache: HashMap<NodeIndex, Box<[OwnedDataType]>, FxBuildHasher>,

    exposed_properties: Box<[PropertyMetadata]>,

    // All relevant information for inputs and outputs required by each instance.
    outputs: Box<[Reference]>,
    output_infos: Box<[TypeDescriptor]>,
    input_infos: Option<Box<[TypeDescriptor]>>,
}

impl AssetTemplate {
    /// This is the way in which different AssetBases get transformed into a template.
    /// This method is called internally when a template gets requested by a AssetProcessor.
    pub(super) fn transform(base: &impl AssetBase) -> Self {
        #[cfg(debug_assertions)]
        let computation_instant = Instant::now();

        let dynamic_node_connectivity = base.nodes_reachable_from_respective_input();
        let mut is_node_dynamic = FixedBitSet::new();
        for reachable_nodes in &dynamic_node_connectivity {
            is_node_dynamic |= reachable_nodes;
        }
        let is_node_dynamic = is_node_dynamic;

        //TODO revive the property stuff
        /*  for node in exposed_nodes {
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
        } */

        for (prop_ref, script) in base.get_scripts() {}

        let mut exposed_properties = Box::new_uninit_slice(2);

        exposed_properties[0].write(PropertyMetadata {
            name: "distance".into(),
            r#type: DataTypeKind::Float,
            default: OwnedDataType::Float(0.5),
            configuration: None,
            documentation: "".into(),
        });

        exposed_properties[1].write(PropertyMetadata {
            name: "height".into(),
            r#type: DataTypeKind::Float,
            default: OwnedDataType::Float(2.),
            configuration: None,
            documentation: "".into(),
        });

        let exposed_properties = unsafe { exposed_properties.assume_init() };

        // Evaluation of nodes

        let mut output_nodes = base.is_node_output();

        let (outputs, output_infos) = base.get_outputs_and_info(&output_nodes);

        let dynamic_nodes_without_outputs = &is_node_dynamic | &output_nodes;

        let constant_cache = base.get_const_cache(&dynamic_nodes_without_outputs);
        // To allow for a bitwise and we need to flip the bits to get all not output nodes.
        output_nodes.toggle_range(..);
        let dynamic_output_filtered = &is_node_dynamic & &output_nodes;

        // Retrieve the dependency map of the dynamic nodes.
        let mut dependency_node_map = base.get_node_map(&dynamic_output_filtered);

        let mut input_nodes = base.is_node_input();
        let input_infos = base.get_input_info(&input_nodes);

        // Same reasoning as with the output nodes applies here.
        input_nodes.toggle_range(..);
        let dynamic_io_filtered = &dynamic_output_filtered & &input_nodes;

        let topo_order = base.get_toposort(&dynamic_io_filtered);

        //TODO this is a really bad hardcode
        _ = dependency_node_map
            .get_mut(&(5.into()))
            .unwrap()
            .1
            .set_property_external(1, Reference::ExternalProperty { index: 0 });

        #[cfg(debug_assertions)]
        println!(
            "Internal template computation took {} nanoseconds",
            computation_instant.elapsed().as_nanos()
        );
        AssetTemplate {
            constant_cache,

            dependency_node_map,
            node_connectivity: dynamic_node_connectivity,
            topo_order,

            exposed_properties,

            outputs,
            output_infos,
            input_infos,
        }
    }

    #[track_caller]
    pub(crate) fn get_reference(&self, reference: Reference) -> TypeRef {
        (&self.constant_cache[&reference.node()][reference.index() as usize]).into()
    }

    /// Contained Helper to retrieve all Nodes which need to recompute in topological order.
    #[inline(always)]
    pub(crate) fn query(
        &self,
        changed: &[bool],
        property_changes: FixedBitSet,
    ) -> Box<[NodeIndex]> {
        //Initial set comes from the nodes which need to be recomputed because of property changes.
        let mut set = property_changes;

        // If the corresponding inputs hace changed produce final bitset with every node set.
        for (index, value) in changed.iter().enumerate() {
            if *value {
                let current_set = &self.node_connectivity[index];
                set |= current_set;
            }
        }

        let changed_nodes = set
            .into_ones()
            .map(|id| Into::<NodeIndex>::into(id as u16))
            .collect::<Box<_>>();

        self.topo_order
            .iter()
            .filter(|topo_id| {
                changed_nodes
                    .iter()
                    .any(|changed_id| changed_id == *topo_id)
            })
            .copied()
            .collect()
    }

    /// Compute abstraction helper to map to the internal node compute function.
    #[inline(always)]
    pub(crate) fn compute(
        &self,
        node: NodeIndex,
        processor: &impl ContextProvider,
    ) -> Box<[Option<OwnedDataType>]> {
        let (inputs, node) = self.dependency_node_map.get(&node).unwrap();
        node.compute(Some(inputs), processor)
    }

    pub fn outputs(&self) -> &[Reference] {
        &self.outputs
    }

    pub fn get_output_sockets(&self) -> impl Iterator<Item = TypeDescriptor> {
        self.output_infos.iter().cloned()
    }

    pub fn get_input_sockets(&self) -> impl Iterator<Item = TypeDescriptor> {
        self.input_infos.as_ref().unwrap().iter().cloned()
    }

    pub fn get_properties(&self) -> &[PropertyMetadata] {
        &self.exposed_properties
    }
}

//This is kind of a hack to access the right intermediate stuff but idk kinda fine
impl ContextProvider for HashMap<NodeIndex, Box<[OwnedDataType]>, FxBuildHasher> {
    fn get_reference(&self, index: Reference) -> TypeRef {
        (&self.get(&index.node()).unwrap()[index.index() as usize]).into()
    }
}
