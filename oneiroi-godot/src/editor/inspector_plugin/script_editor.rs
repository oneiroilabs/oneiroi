use godot::{
    classes::{
        CodeEdit, ICodeEdit,
        control::{LayoutPreset, SizeFlags},
    },
    prelude::*,
};
use oneiroi::asset::{NodeIndex, editable::ScriptingInterface};

use crate::{core::asset::OneiroiAsset, editor::editor_server::OneiroiEditorServer};

#[derive(GodotClass)]
#[class(tool,no_init,internal, base=CodeEdit)]
pub struct OneiroiScriptEditor {
    base: Base<CodeEdit>,
    property_name: String,
    node_index: NodeIndex,
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

        let asset = self.get_asset();
        let script = asset
            .bind()
            .get_editable()
            .try_get_script(self.node_index, 0)
            .unwrap();

        self.base_mut().set_text(&script);

        let text_changed = self.base().callable("on_text_changed");
        self.base_mut().connect("text_changed", &text_changed);
    }
}

#[godot_api]
impl OneiroiScriptEditor {
    pub fn init_with_node_id(property_name: String, node_index: NodeIndex) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            node_index,
            property_name,
        })
    }

    // Retrieves the asset from the Singleton
    fn get_asset(&mut self) -> Gd<OneiroiAsset> {
        godot::classes::Engine::singleton()
            .get_singleton("OneiroiEditorServer")
            .unwrap()
            .cast::<OneiroiEditorServer>()
            .bind()
            .get_active_asset()
            .unwrap()
    }

    #[func]
    fn on_text_changed(&mut self) {
        let mut asset = self.get_asset();
        let mut asset_ref = asset.bind_mut();

        asset_ref.get_edit_mut().try_set_script(
            self.node_index,
            0,
            String::from(self.base().get_text()),
        );
    }
}
