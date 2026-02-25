use std::{collections::HashMap, sync::Arc, time::Instant};

use fixedbitset::FixedBitSet;

use rustc_hash::FxBuildHasher;
use serde::{Deserialize, Serialize};

use crate::{
    asset::template::AssetTemplate,
    nodes::{
        ContextProvider, Node, PropertyInterface, PropertyNotFound, SetPropertyError,
        SocketInterface, StaticNodeMetadata,
    },
    property::{PropertyInstance, PropertyMetadata},
    type_system::{OwnedDataType, Reference, TypeRef, data_types::TypeDescriptor},
};

struct ContextBridge<'a> {
    parent: &'a dyn ContextProvider,
    this: &'a dyn ContextProvider,
    node_cache: HashMap<Reference, OwnedDataType, FxBuildHasher>,
    parent_references: Box<[Reference]>,
}

impl ContextProvider for ContextBridge<'_> {
    fn get_reference(&self, index: Reference) -> TypeRef {
        match index {
            Reference::Standard { .. } => {
                if let Some(value) = self.node_cache.get(&index) {
                    TypeRef::from(value)
                } else {
                    self.this.get_reference(index)
                }
            }
            Reference::ExternalProperty { .. } => self.this.get_reference(index),
            Reference::External { socket } => self
                .parent
                .get_reference(self.parent_references[socket as usize]),
            Reference::Internal { .. } => todo!(),
            Reference::Uninitialized => todo!(),
            Reference::Property { .. } => todo!(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetInstance {
    // All the exposed properties from the Asset.
    properties: Box<[PropertyInstance]>,

    //This is theoretically not optional but rather be injected on deserialization
    #[serde(skip)]
    template: Option<Arc<AssetTemplate>>,
}

impl Clone for AssetInstance {
    fn clone(&self) -> Self {
        Self {
            properties: self.properties.clone(),
            //graph: self.graph.clone(),
            // dynamic_nodes: self.dynamic_nodes.clone(),
            //output_nodes: self.output_nodes.clone(),
            template: self.template.clone(),
            //generation: self.generation,
            //node_cache: self.node_cache.clone(),
            //dirty_nodes: self.dirty_nodes.clone(),
            //mapped_input_sockets: self.mapped_input_sockets.clone(),
            //transformed_input_sockets: self.transformed_input_sockets.clone(),
        }
    }
}

impl AssetInstance {
    pub(super) fn new(template: &Arc<AssetTemplate>) -> Self {
        //take ownership
        let template = template.clone();

        /* let mut transformed_input_sockets =
            Box::new_uninit_slice(template.get_input_sockets().count());
        for item in transformed_input_sockets.iter_mut() {
            item.write(None);
        }
        let transformed_input_sockets =
            RefCell::new(unsafe { transformed_input_sockets.assume_init() }); */

        //let dynamic_graph = template.get_dynamic_graph();

        // let mut dirty_nodes = HashMap::new();
        //TODO Initialize the hashmap with the right capacity to avoid reallocations
        //let mut node_cache =
        //HashMap::with_capacity_and_hasher(dynamic_graph.node_count(), FxBuildHasher);
        //let node_cache = RefCell::new(node_cache);

        let exposed_properties = template.get_properties();
        let mut properties = Box::new_uninit_slice(exposed_properties.len());

        for (index, prop) in exposed_properties.iter().enumerate() {
            properties[index].write(PropertyInstance::new(prop));

            //let starting_node = template.get_property_index_name(prop.name());

            //dirty_nodes.insert(starting_node.0, true);
        }

        let properties = unsafe { properties.assume_init() };

        Self {
            properties,
            //output_sockets: output_sockets.iter().map(|s| SocketInstance {}).collect(),
            //asset_index: 0,
            //graph: dynamic_graph,
            //node_cache: Default::default(),
            // output_nodes: Vec::new(),
            //: template.clone().get_generation(),
            template: Some(template),
            //dirty_nodes,
            // mapped_input_sockets: Default::default(),
            //transformed_input_sockets,
        }
    }

    pub(super) fn set_template(&mut self, template: Arc<AssetTemplate>) {
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
    fn try_set_property(
        &mut self,
        property: &str,
        value: OwnedDataType,
    ) -> Result<(), SetPropertyError> {
        /* if let Some(property) = self.properties.get_mut(property_name) {
            //TODO remove this most likely
            debug_assert!(
                property.get_type() == value.get_type(),
                "Trying to set property with other type than expected"
            );

            *property = value.clone().get_instance();
            //println!("{:?}", value);
        } else {
            //println!("Property {property_name} not found");
            return Err(SetPropertyError::PropertyNotFound);
        } */
        /* let starting_node = self
            .template
            .as_ref()
            .unwrap()
            .get_property_index_name(property_name);

        self.dirty_nodes.insert(starting_node.0, true); */

        //let mhm = self.graph[starting_node.0].try_set_property(&starting_node.1, value.clone());
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

        println!("Property: {property}, to: {value:?}");
        if let Some(index) = self
            .template
            .as_ref()
            .unwrap()
            .get_properties()
            .iter()
            .position(|i| i.name() == property)
        {
            println!("IT FOUND THE POSITION OF THE PROPERTY {property}");
            self.try_set_property_index(index as u8, value)
        } else {
            Err(SetPropertyError::NotFound)
        }
    }

    fn try_get_property(&self, property: &str) -> Result<TypeRef, PropertyNotFound> {
        if let Some(index) = self
            .template
            .as_ref()
            .unwrap()
            .get_properties()
            .iter()
            .position(|i| i.name() == property)
        {
            self.try_get_property_index(index as u8)
        } else {
            Err(PropertyNotFound)
        }
        //
    }

    fn get_properties(&self) -> Box<[PropertyMetadata]> {
        self.template.as_ref().unwrap().get_properties().into()
    }

    fn try_set_property_index(
        &mut self,
        index: u8,
        value: OwnedDataType,
    ) -> Result<(), SetPropertyError> {
        //TODO proper error handling
        self.properties
            .get_mut(index as usize)
            .unwrap()
            .set_value(value);
        Ok(())
    }

    fn try_get_property_index(&self, index: u8) -> Result<TypeRef, PropertyNotFound> {
        Ok((&self.properties[index as usize]).into())
    }

    fn set_property_external(
        &mut self,
        index: u8,
        reference: Reference,
    ) -> Result<(), SetPropertyError> {
        todo!()
    }

    /* fn try_get_property_script(&self, property: &str) -> Result<String, PropertyNotFound> {
        todo!()
    } */

    /* fn try_get_property_metadata(&self, property: &str) -> Result<PropertyMetadata, ()> {
        todo!()
    } */
}

impl Node for AssetInstance {
    fn compute(
        &self,
        input_sockets: Option<&[Reference]>,
        context: &impl ContextProvider,
        //TODO explore this
        //write_back: Box<[&mut DataTypeValue]>,
    ) -> Box<[Option<OwnedDataType>]> {
        let computation_instant = Instant::now();

        //Properly borrow recurring uses only one time instead of each call.
        let template = self.template.as_deref().unwrap();
        // let mut node_cache = self.node_cache.borrow_mut();

        //TODO Update all properties that have changed since the last computation
        //template.

        let mut context = ContextBridge {
            parent: context,
            this: self,
            node_cache: Default::default(),
            parent_references: input_sockets.unwrap().into(),
        };

        //Map the input References received from the outside to local References
        //It needs to be cached to be accessed by nodes calling get_reference()
        //This should maybe be done in an initialization step, maybe not
        //Wait a minute we already receive the right thing to query i think hmmge.
        //Maybe this is not even necessary
        /* let input_sockets = input_sockets.unwrap();
        *self.mapped_input_sockets.borrow_mut() = input_sockets.into();
        {
            let mut sockets_to_map = self.transformed_input_sockets.borrow_mut();
            for (index, reference) in input_sockets.iter().enumerate() {
                //TODO cache the references
                sockets_to_map[index] = Some(context.get_reference(*reference).clone());
            }
        } */

        //TODO receive which references have effectively changed
        //TODO replace with changed slice
        let baked_input: Box<[bool]> = Box::new([true]);
        let input: &[bool] = &baked_input;

        let nodes_to_compute = template.query(input, FixedBitSet::new());
        for node in nodes_to_compute {
            let computed = template.compute(node, &context);
            for (index, value) in computed.into_iter().enumerate() {
                context.node_cache.insert(
                    Reference::Standard {
                        node,
                        socket: index as u8,
                    },
                    value.unwrap(),
                );
            }
        }

        // References got changed
        // Retrieve dependencies from the changed references
        //compute each cluster from changes

        //Maybe retrieve the clusters

        /* let dirty_nodes_entrypoints = self
        .dirty_nodes
        .iter()
        .filter(|(_, v)| **v)
        .map(|v| v.0)
        .copied(); */
        //TODO actually filter
        // it ignores the edges that have a incoming edge so we need to remove that incoming edge
        // but only if no other dependant node also has a update leading to a wrong result
        /* let filtered_graph = EdgeFiltered::from_fn(&self.graph, |edge| true);
        let mut traversal_ordered = Topo::with_initials(&filtered_graph, dirty_nodes_entrypoints);
        while let Some(node) = traversal_ordered.next(&filtered_graph) {
            //println!("HUH  {:?}", node);
            let connections = self.graph.edges_directed(node, Direction::Incoming);
            let mut dependencies = Vec::new();
            for edge in connections {
                let source = edge.source();
                let socket = edge.weight().0;

                dependencies.push(Reference::Standard {
                    socket,
                    node: source,
                })
            }
            let computed = self.graph[node].compute(Some(&dependencies), self);
            for socket in 0..computed.len() {
                self.node_cache.borrow_mut().insert(
                    Reference::Standard {
                        node,
                        socket: socket as u8,
                    },
                    Some(computed[socket].clone()),
                );
            }
        } */

        println!(
            "Internal instance computation took {} nanoseconds",
            computation_instant.elapsed().as_nanos()
        );

        /*  println!("{:#?}", context.node_cache);
        println!("{:#?}", template.outputs().collect::<Vec<_>>()); */

        template
            .outputs()
            .iter()
            .map(|out| {
                if let Some(value) = context.node_cache.remove(out) {
                    Some(value)
                } else {
                    Some(template.get_reference(*out).into())
                }
            })
            .collect::<Box<_>>()
    }

    fn node_metadata(&self) -> StaticNodeMetadata {
        StaticNodeMetadata { color: "#b45309" }
    }
}
impl SocketInterface for AssetInstance {
    fn get_input_sockets(&self) -> Box<[TypeDescriptor]> {
        self.template
            .as_ref()
            .unwrap()
            .get_input_sockets()
            .collect()
    }

    fn get_output_sockets(&self) -> Box<[TypeDescriptor]> {
        self.template
            .as_ref()
            .unwrap()
            .get_output_sockets()
            .collect()
    }
}

impl ContextProvider for AssetInstance {
    fn get_reference(&self, index: Reference) -> TypeRef {
        match index {
            Reference::Standard { .. } => self.template.as_ref().unwrap().get_reference(index),
            Reference::External { .. } => unreachable!(),
            Reference::Internal { .. } => unreachable!(),
            Reference::ExternalProperty { index } => (&self.properties[index as usize]).into(),
            Reference::Uninitialized => unreachable!(),
            Reference::Property { node, index } => unreachable!(),
        }

        /* if let Some(value) = self.node_cache.borrow().get(&index) {
            value.clone()
        } else {
            self.template.as_ref().unwrap().get_reference(index)
        } */
    }
}
