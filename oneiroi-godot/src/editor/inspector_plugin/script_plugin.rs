use crate::editor::asset_editor::node_editor::node_proxy::OneiroiNodeProxy;
use godot::classes::control::SizeFlags;
use godot::classes::{Button, EditorProperty, IEditorProperty, VBoxContainer};
use godot::prelude::*;

use super::script_editor::OneiroiScriptEditor;

#[derive(GodotClass)]
#[class(tool, init,internal, base=EditorProperty)]
pub struct OneiroiScriptPlugin {
    base: Base<EditorProperty>,

    #[init(val=VBoxContainer::new_alloc())]
    container: Gd<VBoxContainer>,

    #[init(val=Button::new_alloc())]
    button: Gd<Button>,
    //#[init(val=OneiroiScriptEditor::new_alloc())]
    #[var]
    code_edit: Option<Gd<OneiroiScriptEditor>>,
    #[var]
    resource: Option<Gd<OneiroiNodeProxy>>,
}

#[godot_api]
impl IEditorProperty for OneiroiScriptPlugin {
    fn enter_tree(&mut self) {
        //get the edited property
        let prop_name = self.base().get_edited_property();

        let resource = self
            .base_mut()
            .get_edited_object()
            .unwrap()
            .cast::<OneiroiNodeProxy>();
        //let node = resource.bind().get_node();

        //Safety: This local variable is necessry! Otherwise Godot frees the resource.
        self.resource = Some(resource.clone());

        self.code_edit = Some(OneiroiScriptEditor::init_with_node_id(
            prop_name.into(),
            //resource.bind().get_node(),
            resource.bind().get_node_id(),
        ));

        // Create container
        //let mut container = HBoxContainer::new_alloc();
        self.container.set_h_size_flags(SizeFlags::EXPAND_FILL);
        self.container.set_v_size_flags(SizeFlags::EXPAND_FILL);

        self.container.add_child(&self.button);
        self.container
            .add_child(self.code_edit.as_ref().expect("fff"));

        /* //TODO rework this
        self.code_edit
            .set_text("test" /* &property.bind().get_expression() */); //TODO */

        self.button.set_text("Edit Expression");
        let on_code_edit = self.base().callable("on_code_edit_toggle");
        self.button.connect("pressed", &on_code_edit);

        let on_focus_exited = self.base().callable("on_focus_exited");
        self.base_mut().connect("focus_exited", &on_focus_exited);
        self.code_edit
            .as_mut()
            .expect("ffff")
            .bind_mut()
            .base_mut()
            .connect("focus_exited", &on_focus_exited);

        //finish initialisation and send to inspector
        let container = self.container.clone();
        self.base_mut().add_child(&container);
        self.base_mut().add_focusable(&container);
        self.base_mut().set_bottom_editor(&container);
    }

    fn exit_tree(&mut self) {
        // Remove element from inspector when this plugin unmount:
        /* if let Some(editor) = self.code_edit.take() {
            self.base_mut().remove_child(&editor);
        } else {
            // Log error if button disappeared before
            godot_error!("Button wasn't found in exit_tree");
        } */
    }
}

#[godot_api]
impl OneiroiScriptPlugin {
    #[func]
    fn on_code_edit_toggle(&mut self) {
        //TODO add validation if expression is correct

        let state = self.code_edit.as_ref().unwrap().is_visible();
        if state {
            self.button.set_text("Edit Expression");
        } else {
            self.button.set_text("Confirm");
        }
        //self.code_edit_toggle.set_visible(!state);
        self.code_edit.as_mut().unwrap().set_visible(!state);
        //self.dividor.set_visible(state);
    }

    #[func]
    fn on_focus_exited(&mut self) {
        if self.code_edit.as_ref().unwrap().has_focus() {
            return;
        }
        self.button.set_text("Edit Expression");
        self.code_edit.as_mut().unwrap().set_visible(false);
        //TODO set to deafult state again if expression invalid at that point
    }
}
