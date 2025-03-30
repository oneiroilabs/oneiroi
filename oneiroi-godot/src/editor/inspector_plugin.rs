use godot::classes::{EditorInspectorPlugin, EditorPlugin, IEditorInspectorPlugin, IEditorPlugin};
use godot::{global, prelude::*};
use script_plugin::OneiroiScriptPlugin;

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
        value_type: VariantType,
        name: GString,
        _hint_type: global::PropertyHint,
        _hit_string: GString,
        _flags: global::PropertyUsageFlags,
        _wide: bool,
    ) -> bool {
        _ = object.clone();

        //remove the build in resource editors
        if name == "resource_local_to_scene".into()
            || name == "resource_path".into()
            || name == "resource_name".into()
            || name == "script".into()
        {
            return false;
        }

        //TODO make the script editor available again
        if let Ok(vec) = object.as_ref().unwrap().get(name.arg()).try_to::<Vector3>() {
            //godot_print!("Property {:?} was {:?}", name, vec);
            self.base_mut()
                .add_property_editor_ex(&name, &OneiroiScriptPlugin::new_alloc())
                .add_to_end(true)
                .done();
        }
        if let Ok(vec) = object.as_ref().unwrap().get(name.arg()).try_to::<bool>() {
            //godot_print!("Property {:?} was {:?}", name, vec);
            self.base_mut()
                .add_property_editor_ex(&name, &OneiroiScriptPlugin::new_alloc())
                .add_to_end(true)
                .done();
        }
        if let Ok(vec) = object.as_ref().unwrap().get(name.arg()).try_to::<i64>() {
            //godot_print!("Property {:?} was {:?}", name, vec);
            self.base_mut()
                .add_property_editor_ex(&name, &OneiroiScriptPlugin::new_alloc())
                .add_to_end(true)
                .done();
        }
        if let Ok(vec) = object.as_ref().unwrap().get(name.arg()).try_to::<f32>() {
            //godot_print!("Property {:?} was {:?}", name, vec);
            self.base_mut()
                .add_property_editor_ex(&name, &OneiroiScriptPlugin::new_alloc())
                .add_to_end(true)
                .done();
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
        self.inspector = plugin.clone();
        self.base_mut().add_inspector_plugin(&plugin);
    }

    fn exit_tree(&mut self) {
        // Remove inspector plugin when editor plugin leaves scene tree.
        let plugin = self.inspector.clone();
        self.base_mut().remove_inspector_plugin(&plugin);
    }
}
