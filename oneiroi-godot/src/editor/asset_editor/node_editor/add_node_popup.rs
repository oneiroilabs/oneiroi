use godot::{
    classes::{
        window::{Flags, WindowInitialPosition},
        IPopup, Popup,
    },
    prelude::*,
};

use super::add_node_tree::AddNodeTree;

#[derive(GodotClass)]
#[class(tool,internal, base=Popup)]
pub struct AddNodePopup {
    base: Base<Popup>,
    pub available_nodes_tree: Gd<AddNodeTree>,
}

#[godot_api]
impl IPopup for AddNodePopup {
    fn init(base: Base<Popup>) -> Self {
        Self {
            base,
            available_nodes_tree: AddNodeTree::new_alloc(),
        }
    }

    fn ready(&mut self) {
        self.base_mut().set_title("Add new Node");
        self.base_mut().set_flag(Flags::BORDERLESS, false);
        self.base_mut().set_size(Vector2i { x: 500, y: 500 });
        //new_node_select.set_flag(Flags::ALWAYS_ON_TOP, true);
        self.base_mut()
            .set_initial_position(WindowInitialPosition::CENTER_MAIN_WINDOW_SCREEN);
        //new_node_select.set_flag(Flags::EXTEND_TO_TITLE, true);
        self.base_mut().set_visible(false);
        let available_nodes_tree = self.available_nodes_tree.clone();
        self.base_mut().add_child(&available_nodes_tree);
    }
}

#[godot_api]
impl AddNodePopup {
    #[func]
    pub fn make_visible(&mut self, position: Vector2) {
        self.available_nodes_tree.bind_mut().invoke_position = position;
        self.base_mut().set_visible(true);
    }
}
