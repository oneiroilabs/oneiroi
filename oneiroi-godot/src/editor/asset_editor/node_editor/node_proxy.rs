use std::cell::RefCell;
use std::rc::Rc;

use godot::global::PropertyUsageFlags;
use godot::meta::{ClassName, PropertyHintInfo, PropertyInfo};
use godot::prelude::*;
use godot::{classes::Resource, obj::Gd};

use oneiroi::asset::{NodeIndex, NodeMetadata, Vec2};
use oneiroi::data_types::DataTypeType;
use oneiroi::operations::{Nodes, Operation, PropertyInterface, StaticNodeMetadata};

use crate::core::data_conversion::{
    DataTypeConversion, GodotDataTypeToOneiroiDataType, OneiroiToGodot,
};

#[derive(GodotClass)]
#[class(tool,no_init,internal, base=Resource)]
pub struct OneiroiNodeProxy {
    base: Base<Resource>,
    node_index: NodeIndex,
    node: Rc<(RefCell<NodeMetadata>, RefCell<Nodes>)>,
}

#[cfg(debug_assertions)]
impl Drop for OneiroiNodeProxy {
    fn drop(&mut self) {
        godot_print!("{:#?} is being destroyed!", self.node);
    }
}

#[godot_api]
impl IResource for OneiroiNodeProxy {
    fn get_property_list(&mut self) -> Vec<PropertyInfo> {
        let mut properties = Vec::<PropertyInfo>::new();

        for prop in self.node.1.borrow().get_properties() {
            properties.push(PropertyInfo {
                variant_type: prop.get_type().variant_type(),
                class_name: ClassName::none(),
                property_name: prop.name().into(),
                hint_info: PropertyHintInfo::none(),
                usage: PropertyUsageFlags::EDITOR,
            });
        }
        properties
    }

    /* fn property_get_revert(&self, property: StringName) -> Option<Variant> {
        for prop in self.props.iter_shared() {
            if prop.at("prop_name").to::<StringName>() == property {
                godot_print!("{} {}", property, prop.at("prop_default"));
                return Some(prop.at("prop_default"));
            }
        }
        None
    } */

    fn get_property(&self, property: StringName) -> Option<Variant> {
        if let Ok(prop) = self.node.1.borrow().try_get_property(&property.to_string()) {
            /* godot_print!(
                "Called get on {:?}, {:?}",
                property,
                prop.get_variant_value()
            ); */
            return Some(prop.to_godot());
        }
        None
    }

    fn set_property(&mut self, property: StringName, value: Variant) -> bool {
        //godot_print!("Called Set Property");
        let Ok(_) = self
            .node
            .1
            .borrow_mut()
            //MAYBE TODO here none could be replaced by edited object?
            .try_set_property(&property.to_string(), value.to_oneiroi())
        else {
            return false;
        };
        self.node.0.borrow_mut().set_dirty();

        true
    }
}

#[godot_api]
impl OneiroiNodeProxy {
    pub fn init_with_node(
        node: Rc<(RefCell<NodeMetadata>, RefCell<Nodes>)>,
        node_index: NodeIndex,
    ) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            node,
            node_index,
        })
    }

    /* pub fn init_with_node(node_index: NodeIndex) -> Gd<Self> {
        Gd::from_init_fn(|base| Self { base, node_index })
    } */

    pub fn get_info(&self) -> NodeMetadata {
        //cloning because running into bind issues otherwise
        //doesnt matter to much anyway bcs its editor funcitonality
        self.node.0.borrow().clone()
    }

    pub fn static_metadata(&self) -> StaticNodeMetadata {
        //cloning because running into bind issues otherwise
        //doesnt matter to much anyway bcs its editor funcitonality
        self.node.1.borrow().static_metadata()
    }

    pub fn get_node_id(&self) -> NodeIndex {
        self.node_index
    }

    pub fn set_position(&self, position: Vector2) {
        self.node
            .as_ref()
            .0
            .borrow_mut()
            .set_position(Vec2::new(position.x, position.y))
    }

    pub fn get_sockets(&self) -> (Vec<DataTypeType>, Vec<DataTypeType>) {
        self.node.1.borrow().get_sockets()
    }

    //TODO remove most likely
    /* pub fn get_node(&self) -> Rc<(RefCell<NodeMetadata>, RefCell<Nodes>)> {
        self.node.clone()
    } */
}
