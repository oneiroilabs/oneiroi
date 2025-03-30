use godot::{
    classes::{
        Button, Control, EditorPlugin, HSplitContainer, IControl, IEditorPlugin, VSplitContainer,
        control::{LayoutPreset, SizeFlags},
    },
    prelude::*,
};
use tree_attribute_display::GeometrySheetTree;

mod tree_attribute_display;

#[derive(GodotClass)]
#[class(tool,internal, init, base=Control)]
pub struct GeometrySheet {
    base: Base<Control>,
    #[init(val=GeometrySheetTree::new_alloc())]
    tree: Gd<GeometrySheetTree>,
}

#[godot_api]
impl IControl for GeometrySheet {
    fn ready(&mut self) {
        let tree = self.tree.clone();

        let mut split = HSplitContainer::new_alloc();
        split.set_anchors_and_offsets_preset(LayoutPreset::FULL_RECT);

        let selector = Button::new_alloc();

        split.add_child(&selector);
        split.add_child(&tree);

        self.base_mut().add_child(&split)
    }
}

#[derive(GodotClass)]
#[class(tool,internal, base=EditorPlugin)]
struct GeometrySheetContainer {
    base: Base<EditorPlugin>,
    main_control: Gd<GeometrySheet>,
}

#[godot_api]
impl IEditorPlugin for GeometrySheetContainer {
    fn init(base: Base<EditorPlugin>) -> Self {
        let main_control = GeometrySheet::new_alloc();

        Self { main_control, base }
    }

    fn enter_tree(&mut self) {
        let control_clone = self.main_control.clone();
        self.base_mut()
            .add_control_to_bottom_panel(&control_clone, "Geometry Sheet");
    }

    fn exit_tree(&mut self) {
        let cloned_ctrl = self.main_control.clone();
        self.base_mut()
            .remove_control_from_bottom_panel(&cloned_ctrl);
        self.main_control.clone().free();
    }
    fn make_visible(&mut self, is_visible: bool) {
        let cloned_ctrl = self.main_control.clone();
        if is_visible {
            self.base_mut().make_bottom_panel_item_visible(&cloned_ctrl);
            /* self.main_control.set_visible(true); */
        } else {
            /* self.base_mut().hide_bottom_panel(); */
            /* self.main_control.set_visible(false); */
        }
    }

    fn get_plugin_name(&self) -> GString {
        "OneiroiGeometrySheet".into()
    }

    //Should not popup automatically when resource is selected
    fn handles(&self, _: Gd<Object>) -> bool {
        false
    }
}
