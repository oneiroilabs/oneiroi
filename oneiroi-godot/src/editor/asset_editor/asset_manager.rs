use godot::{
    classes::{
        ITabContainer, TabContainer,
        control::{LayoutPreset, SizeFlags},
        tab_bar::CloseButtonDisplayPolicy,
    },
    prelude::*,
};

use crate::{core::asset::OneiroiAsset, editor::editor_server::OneiroiEditorServer};

use super::node_editor::OneiroiNodeEditor;

#[derive(GodotClass)]
#[class(tool,internal, base=TabContainer)]
pub struct OneiroiAssetManager {
    base: Base<TabContainer>,
    /* parent: OnReady<Gd<VBoxContainer>>, */
    active_tabs: Vec<Gd<OneiroiAsset>>,
}

#[godot_api]
impl OneiroiAssetManager {
    #[func]
    fn on_new_asset_registered(&mut self, object: Gd<OneiroiAsset>) {
        //TODO this has to be reworked
        let pro_graph = OneiroiNodeEditor::init_with_asset(object.clone());

        self.active_tabs.push(object);
        let idx = self.active_tabs.len();
        self.base_mut().add_child(&pro_graph);
        self.base_mut()
            .set_current_tab(i32::try_from(idx - 1).unwrap());
    }
}

#[godot_api]
impl ITabContainer for OneiroiAssetManager {
    fn init(base: Base<TabContainer>) -> Self {
        Self {
            base,
            active_tabs: Vec::new(),
        }
    }

    fn ready(&mut self) {
        self.base_mut().set_h_size_flags(SizeFlags::EXPAND_FILL);
        self.base_mut().set_anchors_preset(LayoutPreset::FULL_RECT);
        let mut tabbar = self.base().get_tab_bar().unwrap();
        tabbar.set_tab_close_display_policy(CloseButtonDisplayPolicy::SHOW_ACTIVE_ONLY);

        //TODO loop through singleton
        //connect to OneiroiEditorServer to instantiate new Tabs
        let suite_registered_handler = self.base().callable("on_new_asset_registered");
        godot::classes::Engine::singleton()
            .get_singleton("OneiroiEditorServer")
            .expect("Oneiroi Suite Server not available HUH?")
            .cast::<OneiroiEditorServer>()
            .connect("new_active_asset", &suite_registered_handler);

        /* let pro_graph = OneiroiGraph::new_alloc();

        self.base_mut().add_child(pro_graph);
        let pro_graph = OneiroiGraph::new_alloc();

        self.base_mut().add_child(pro_graph) */
    }
}
