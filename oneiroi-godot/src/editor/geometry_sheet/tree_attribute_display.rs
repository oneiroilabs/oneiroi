use godot::{
    classes::{
        GraphEdit, GraphNode, IGraphEdit, IPopup, ITree, InputEvent, InputEventMouseButton, Popup,
        PopupMenu, Tree, TreeItem,
        control::{LayoutPreset, SizeFlags},
        graph_edit::GridPattern,
        tree::SelectMode,
        window::{Flags, WindowInitialPosition},
    },
    global::MouseButton,
    prelude::*,
};

use crate::editor::editor_server::OneiroiEditorServer;

#[derive(GodotClass)]
#[class(tool,init,internal, base=Tree)]
pub struct GeometrySheetTree {
    base: Base<Tree>,

    #[var]
    root: Option<Gd<TreeItem>>,
}

#[godot_api]
impl ITree for GeometrySheetTree {
    fn ready(&mut self) {
        //TODO maybe make this in other tab
        let on_node_selected = self.base().callable("on_node_selected");
        godot::classes::Engine::singleton()
            .get_singleton(&StringName::from("OneiroiEditorServer"))
            .expect("Oneiroi Suite Server not available HUH?")
            .cast::<OneiroiEditorServer>()
            .connect("node_selected", &on_node_selected);

        self.base_mut()
            .set_anchors_and_offsets_preset(LayoutPreset::FULL_RECT);

        self.base_mut().set_hide_root(true);
        self.base_mut().set_select_mode(SelectMode::ROW);
        self.base_mut().set_column_titles_visible(true);

        self.clear();
    }
}

#[godot_api]
impl GeometrySheetTree {
    fn clear(&mut self) {
        if let Some(root) = self.root.clone() {
            root.free();
        }

        let root = self
            .base_mut()
            .create_item()
            .expect("Couldnt generate new Tree Item");
        self.root = Some(root)
    }

    #[func]
    fn on_node_selected(&mut self, resource: Gd<Resource>) {
        //maybe get custom props here to inspect them
        /*  let data = resource
            .get("base_props")
            .try_to::<Gd<OneiroiProps>>()
            .expect("has no Oneiroi props")
            .bind()
            .get_cache();
        self.clear();
        self.base_mut().set_columns(4);
        self.base_mut().set_column_title(0, "Index");
        //self.base_mut().set_column_expand(0, false);
        //self.base_mut().set_column_clip_content(0, false);
        //self.base_mut().set_column_custom_minimum_width(0, 0);
        self.base_mut().set_column_title(1, "x");
        //self.base_mut().set_column_expand(1, false);
        //self.base_mut().set_column_clip_content(1, false);
        //self.base_mut().set_column_custom_minimum_width(1, 0);
        self.base_mut().set_column_title(2, "y");
        //self.base_mut().set_column_expand(2, false);
        //self.base_mut().set_column_clip_content(2, false);
        //self.base_mut().set_column_custom_minimum_width(2, 0);
        self.base_mut().set_column_title(3, "z"); */
        //self.base_mut().set_column_expand(3, false);
        //self.base_mut().set_column_clip_content(3, false);
        //self.base_mut().set_column_custom_minimum_width(3, 0);
        /* let position = data.bind().get_points();
        for (idx, vec) in position.to_vec().iter().enumerate() {
            let mut item = self
                .root
                .clone()
                .expect("Root has to be there")
                .create_child()
                .expect("Child couldnt be created");

            item.set_text(0, idx.to_string().into());
            item.set_text(1, format!("{:.3}", vec.x).into());
            item.set_tooltip_text(1, vec.x.to_string().into());
            item.set_text(2, format!("{:.3}", vec.y).into());
            item.set_tooltip_text(2, vec.y.to_string().into());
            item.set_text(3, format!("{:.3}", vec.z).into());
            item.set_tooltip_text(3, vec.z.to_string().into());
        } */
    }
}
