use godot::classes::{EditorInspectorPlugin, EditorPlugin, IEditorInspectorPlugin, IEditorPlugin};
use godot::{global, prelude::*};
use script_plugin::OneiroiScriptPlugin;

use crate::editor::asset_editor::node_editor::node_proxy::OneiroiNode;

mod script_editor;
mod script_plugin;

#[derive(GodotClass)]
#[class(tool, init,internal, base=EditorInspectorPlugin)]
struct OneiroiNodeInspectorPlugin {
    base: Base<EditorInspectorPlugin>,
}

#[godot_api]
impl IEditorInspectorPlugin for OneiroiNodeInspectorPlugin {
    fn parse_property(
        &mut self,
        object: Option<Gd<Object>>, // object that is being inspected
        _value_type: VariantType,
        name: GString,
        _hint_type: global::PropertyHint,
        _hit_string: GString,
        _flags: global::PropertyUsageFlags,
        _wide: bool,
    ) -> bool {
        //remove the build in resource editors
        if name == "resource_local_to_scene".into()
            || name == "resource_path".into()
            || name == "resource_name".into()
            || name == "script".into()
        {
            return false;
        }

        let object = object.unwrap().cast::<OneiroiNode>();
        let id = object.bind().get_index();

        match object.get(name.arg()).get_type() {
            VariantType::VECTOR3 => self
                .base_mut()
                .add_property_editor_ex(&name, &OneiroiScriptPlugin::init_with_node_id(id))
                .add_to_end(true)
                .done(),
            VariantType::BOOL => self
                .base_mut()
                .add_property_editor_ex(&name, &OneiroiScriptPlugin::init_with_node_id(id))
                .add_to_end(true)
                .done(),
            VariantType::INT => self
                .base_mut()
                .add_property_editor_ex(&name, &OneiroiScriptPlugin::init_with_node_id(id))
                .add_to_end(true)
                .done(),
            VariantType::FLOAT => self
                .base_mut()
                .add_property_editor_ex(&name, &OneiroiScriptPlugin::init_with_node_id(id))
                .add_to_end(true)
                .done(),
            _ => (),
        }
        false
    }

    fn can_handle(&self, object: Option<Gd<Object>>) -> bool {
        if object.as_ref().unwrap().is_class("OneiroiNodeProxy") {
            return true;
        }

        false
    }
}

#[derive(GodotClass)]
#[class(tool, init,internal, base=EditorPlugin)]
struct InspectorEditorPlugin {
    base: Base<EditorPlugin>,
    inspector: Gd<OneiroiNodeInspectorPlugin>,
}

#[godot_api]
impl IEditorPlugin for InspectorEditorPlugin {
    fn enter_tree(&mut self) {
        // Create our inspector plugin and save it.
        let plugin = OneiroiNodeInspectorPlugin::new_gd();
        self.base_mut().add_inspector_plugin(&plugin);
        self.inspector = plugin;
    }

    fn exit_tree(&mut self) {
        // Remove inspector plugin when editor plugin leaves scene tree.
        let plugin = self.inspector.clone();
        self.base_mut().remove_inspector_plugin(&plugin);
    }
}
