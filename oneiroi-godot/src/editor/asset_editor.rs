use asset_manager::OneiroiAssetManager;
use asset_preview::OneiroiAssetPreview;
use godot::classes::control::{LayoutPreset, SizeFlags};
use godot::classes::{
    EditorInterface, EditorPlugin, HSplitContainer, IEditorPlugin, SubViewportContainer,
};
use godot::prelude::*;

use crate::core::asset::OneiroiAsset;

use super::editor_server::OneiroiEditorServer;

pub mod asset_manager;
pub mod asset_preview;
pub mod node_editor;

#[derive(GodotClass)]
#[class(tool,internal, base=EditorPlugin)]
struct OneiroiAssetEditor {
    base: Base<EditorPlugin>,
    main_control: Gd<HSplitContainer>,
}

#[godot_api]
impl IEditorPlugin for OneiroiAssetEditor {
    fn init(base: Base<EditorPlugin>) -> Self {
        Self {
            main_control: build_main_screen_ui(),
            base,
        }
    }

    fn enter_tree(&mut self) {
        self.main_control.set_visible(false);
        EditorInterface::singleton()
            .get_editor_main_screen()
            .expect("The Editor Main Screen isnt there wtf?")
            .add_child(&self.main_control);
    }

    fn exit_tree(&mut self) {
        // Perform typical plugin operations here.
        self.main_control.clone().free();
    }

    fn has_main_screen(&self) -> bool {
        true
    }

    fn get_plugin_name(&self) -> GString {
        "OneiroiEditor".into()
    }
    fn make_visible(&mut self, is_visible: bool) {
        if is_visible {
            self.main_control.set_visible(true);
        } else {
            self.main_control.set_visible(false);
        }
    }

    fn handles(&self, object: Gd<Object>) -> bool {
        if object.is_class("OneiroiAsset") {
            let obj = object.cast::<OneiroiAsset>();
            if obj.get_path().is_empty() {
                return false;
            }
            let mut server = godot::classes::Engine::singleton()
                .get_singleton("OneiroiEditorServer")
                .expect("Oneiroi Suite Server not available HUH?")
                .cast::<OneiroiEditorServer>();
            server.bind_mut().register_object_to_edit(obj);

            return true;
        }
        false
    }
}

fn build_main_screen_ui() -> Gd<HSplitContainer> {
    let mut top_split = HSplitContainer::new_alloc();
    top_split.set_anchors_preset(LayoutPreset::FULL_RECT);
    top_split.set_h_size_flags(SizeFlags::EXPAND_FILL);
    top_split.set_v_size_flags(SizeFlags::EXPAND_FILL);

    let active_tabs = OneiroiAssetManager::new_alloc();

    let mut vpc = SubViewportContainer::new_alloc();
    vpc.set_h_size_flags(SizeFlags::EXPAND_FILL);
    vpc.set_stretch(true);

    let vp = OneiroiAssetPreview::new_alloc();

    vpc.add_child(&vp);

    top_split.add_child(&vpc);
    top_split.add_child(&active_tabs);
    top_split
}
