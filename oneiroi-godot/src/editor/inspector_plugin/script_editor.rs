use std::{cell::RefCell, rc::Rc};

use godot::{
    classes::{
        CodeEdit, ICodeEdit,
        control::{LayoutPreset, SizeFlags},
    },
    prelude::*,
};
use oneiroi::{
    asset::{Dependency, NodeIndex, NodeMetadata},
    data_types::{DataTypeType, Vec3},
    operations::{Nodes, Operation, PropertyInterface},
    script::OneiroiScript,
};

use crate::{core::asset::OneiroiAsset, editor::editor_server::OneiroiEditorServer};

//use crate::editor::Oneiroi_class_db_server::OneiroiClassDb;

#[derive(GodotClass)]
#[class(tool,no_init,internal, base=CodeEdit)]
pub struct OneiroiScriptEditor {
    base: Base<CodeEdit>,
    property_name: String,
    node_id: NodeIndex,
    //TODO this can most likely be removed
    //node: Rc<(RefCell<NodeMetadata>, RefCell<Nodes>)>,
}

#[godot_api]
impl ICodeEdit for OneiroiScriptEditor {
    fn ready(&mut self) {
        self.base_mut()
            .set_anchors_and_offsets_preset(LayoutPreset::FULL_RECT);
        self.base_mut().set_h_size_flags(SizeFlags::EXPAND_FILL);
        self.base_mut().set_visible(false);
        self.base_mut()
            .set_custom_minimum_size(Vector2::new(1.0, 75.0));

        /* let property = self
        .node
        .1
        .borrow()
        .try_get_property(self.property_name.clone()); */
        let property = self
            .get_asset()
            .bind_mut()
            .get_inner_mut()
            .get_node(self.node_id)
            .1
            .borrow()
            .try_get_property(&self.property_name)
            .unwrap();
        self.base_mut().set_text(&property.get_expression());

        let text_changed = self.base().callable("on_text_changed");
        self.base_mut().connect("text_changed", &text_changed);

        /* let text_set = self.base().callable("on_text_set");
        self.base_mut().connect("text_changed", &text_set); */
    }
}

#[godot_api]
impl OneiroiScriptEditor {
    /* pub fn init_with_node(
        property_name: String,
        node: Rc<(RefCell<NodeMetadata>, RefCell<Nodes>)>,
    ) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            node,
            property_name,
        })
    } */

    pub fn init_with_node_id(
        property_name: String,
        //node: Rc<(RefCell<NodeMetadata>, RefCell<Nodes>)>,
        node_id: NodeIndex,
    ) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            node_id,
            // node,
            property_name,
        })
    }

    fn get_asset(&mut self) -> Gd<OneiroiAsset> {
        godot::classes::Engine::singleton()
            .get_singleton("OneiroiEditorServer")
            .expect("Oneiroi Suite Server not available HUH?")
            .cast::<OneiroiEditorServer>()
            .bind()
            .get_active_asset()
            .unwrap()
    }

    #[func]
    fn on_text_changed(&mut self) {
        godot_print!("text changed");

        /* let mut asset = godot::classes::Engine::singleton()
            .get_singleton("OneiroiEditorServer")
            .expect("Oneiroi Suite Server not available HUH?")
            .cast::<OneiroiEditorServer>()
            .bind()
            .get_active_asset()
            .as_ref()
            .expect("fff")
            .clone();
        let mut asset_ref = asset.bind_mut();
        let bound_asset = asset_ref.get_inner_mut(); */
        let mut asset = self.get_asset();
        let mut asset_ref = asset.bind_mut();
        let bound_asset = asset_ref.get_inner_mut();

        /* let prop = self
        .node
        .1
        .borrow()
        .try_get_property(self.property_name.clone())
        .unwrap(); */
        let prop = bound_asset
            .get_node(self.node_id)
            .1
            .borrow()
            .try_get_property(&self.property_name)
            .unwrap();

        /* self.base().get_text().into() */
        //bound_asset.
        match prop.get_type() {
            DataTypeType::Omni => todo!(),
            DataTypeType::Mesh => todo!(),
            DataTypeType::Collection => todo!(),
            DataTypeType::Instance => todo!(),
            DataTypeType::Curve => todo!(),
            DataTypeType::Collider => todo!(),
            DataTypeType::Bool => todo!(),
            DataTypeType::Selection => todo!(),
            DataTypeType::Vec3 => {
                if let Ok(parsed) = OneiroiScript::<Vec3>::try_parse(
                    "Vec3(1,@expose('wow',3),1)",
                    bound_asset,
                    self.node_id,
                ) {
                    godot_print!("{:?}", parsed);
                    if bound_asset
                        .try_add_dependency(
                            self.node_id,
                            NodeIndex::from(0),
                            Dependency::Expose {
                                name: "wow".to_owned(),
                                target_property: self.property_name.clone(),
                            },
                        )
                        .is_ok()
                    {
                        godot_print!("YAY")
                    } else {
                        godot_error!("TOOD handle this")
                    }
                } else {
                    godot_error!("Das ist nicht gut TODO handle error greacefully")
                }
            }
            DataTypeType::Int => todo!(),
            DataTypeType::Float => todo!(),
            DataTypeType::Curve2d => todo!(),
        }

        //let result = Script::try_parse("Vec3(1,@expose('wow',3),1)".to_string(), bound_asset);
        //godot_print!("{:?}", result)

        //godot_print!("{:?}", self.base().get_text());
    }

    /* #[func]
    fn on_text_set(&mut self) {
        godot_print!("text set");
    } */
}
