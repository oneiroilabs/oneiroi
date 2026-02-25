use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::rc::Rc;

use godot::classes::Engine;
use godot::global::PropertyUsageFlags;
use godot::meta::{ClassName, PropertyHintInfo, PropertyInfo};
use godot::prelude::*;
use godot::{classes::Resource, obj::Gd};

use oneiroi::asset::editable::AssetEditorMethods;
use oneiroi::asset::{NodeIndex, NodeMetadata, Vec2};
use oneiroi::nodes::{SocketMetadata, StaticNodeMetadata};

use crate::core::asset::OneiroiAsset;
use crate::core::data_conversion::{OneiroiToGodot, TypeConvert};
use crate::editor::editor_server::OneiroiEditorServer;

#[derive(GodotClass)]
#[class(tool,no_init,internal, base=Resource)]
pub struct OneiroiNode {
    base: Base<Resource>,
    node_index: NodeIndex,
    asset: Gd<OneiroiAsset>,
    //node: Rc<(RefCell<NodeMetadata>, RefCell<Nodes>)>,
}

#[cfg(debug_assertions)]
impl Drop for OneiroiNode {
    fn drop(&mut self) {
        godot_print!("{:#?} is being destroyed!", self.node_index);
    }
}

#[godot_api]
impl IResource for OneiroiNode {
    fn get_property_list(&mut self) -> Vec<PropertyInfo> {
        let mut properties = Vec::<PropertyInfo>::new();

        for prop in self
            .asset
            .bind()
            .get_editable()
            .get_node_properties(self.node_index)
        {
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
        if let Ok(prop) = self
            .asset
            .bind()
            .get_editable()
            .try_get_node_property(self.node_index, &property.to_string())
        {
            /* godot_print!(
                "Called get on {:?}, {:?}",
                property,
                prop.get_variant_value()
            ); */
            return Some(prop.convert());
        }
        None
    }

    fn set_property(&mut self, property: StringName, value: Variant) -> bool {
        //godot_print!("Called Set Property");
        let Ok(_) = self.asset.bind_mut().get_edit_mut().try_set_node_property(
            self.node_index,
            &property.to_string(),
            value.convert(),
        ) else {
            return false;
        };
        //self.node.0.borrow_mut().set_dirty();

        true
    }
}

#[godot_api]
impl OneiroiNode {
    pub fn init_with_node(node_index: NodeIndex) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            node_index,
            asset: Engine::singleton()
                .get_singleton("OneiroiEditorServer")
                .unwrap()
                .cast::<OneiroiEditorServer>()
                .bind()
                .get_active_asset()
                .unwrap(),
        })
    }

    pub fn get_metadata(&self) -> NodeMetadata {
        //cloning because running into bind issues otherwise
        //doesnt matter to much anyway bcs its editor funcitonality
        self.asset
            .bind()
            .get_editable()
            .try_get_node_metadata(self.node_index)
            .unwrap()
    }

    //TODO maybe remove this
    pub fn static_metadata(&self) -> StaticNodeMetadata {
        //cloning because running into bind issues otherwise
        //doesnt matter to much anyway bcs its editor funcitonality
        self.asset
            .bind()
            .get_editable()
            .try_get_node_static_metadata(self.node_index)
            .unwrap()
    }

    pub fn get_index(&self) -> NodeIndex {
        self.node_index
    }

    #[func]
    fn node_index_integer(&self) -> u16 {
        self.node_index.index() as u16
    }

    pub fn set_position(&mut self, position: Vector2) {
        self.asset
            .bind_mut()
            .get_edit_mut()
            .try_set_node_position(self.node_index, Vec2::new(position.x, position.y));
    }

    pub fn get_sockets(&self) -> (Box<[SocketMetadata]>, Box<[SocketMetadata]>) {
        //TODO  error handling
        self.asset
            .bind()
            .get_editable()
            .try_get_node_sockets(self.node_index)
            .unwrap()
    }
}
