use godot::{
    classes::{ITree, Tree, control::LayoutPreset},
    prelude::*,
};

#[derive(GodotClass)]
#[class(tool,init,internal, base=Tree)]
pub struct AddNodeTree {
    base: Base<Tree>,
    //#[init(val= OneiroiClassDb::new_alloc())]
    //classes: Gd<OneiroiClassDb>,
    pub invoke_position: Vector2, //active_graph_edit: Gd<GraphEdit>,
}

#[godot_api]
impl ITree for AddNodeTree {
    fn ready(&mut self) {
        self.base_mut()
            .set_anchors_and_offsets_preset(LayoutPreset::FULL_RECT);

        /* let mut ps_classdb = Engine::singleton()
            .get_singleton("OneiroiClassDb")
            .expect("Can find OneiroiClassDb")
            .try_cast::<OneiroiClassDb>()
            .expect("Is not ProcedutalClassDb");
        ps_classdb.call("update_class_list", &[]); */

        //Iterate through ClassDb and Build tree
        let mut root = self
            .base_mut()
            .create_item()
            .expect("Couldnt generate new Tree Item");
        root.set_text(0, "Root");

        /* for classes in ps_classdb.bind().get_nodes().iter_shared() {
            let mut item = self
                .base_mut()
                .create_item()
                .expect("Couldnt generate new Tree Item");
            item.set_text(
                0,
                &classes
                    .get("alias")
                    .expect("Class has no alias")
                    .try_to::<GString>()
                    .expect("alias is no GString"),
            );
        } */

        let mut item = self
            .base_mut()
            .create_item()
            .expect("Couldnt generate new Tree Item");
        item.set_text(0, "Box");
        let mut item = self
            .base_mut()
            .create_item()
            .expect("Couldnt generate new Tree Item");
        item.set_text(0, "Cylinder");
        let mut item = self
            .base_mut()
            .create_item()
            .expect("Couldnt generate new Tree Item");
        item.set_text(0, "Extrude");
        let mut item = self
            .base_mut()
            .create_item()
            .expect("Couldnt generate new Tree Item");
        item.set_text(0, "Output");
        let mut item = self
            .base_mut()
            .create_item()
            .expect("Couldnt generate new Tree Item");
        item.set_text(0, "SubGraph");

        //subscribe to item activated signal and connect it to on_item_activated
        let item_activated_handler = self.base().callable("on_item_activated");
        self.base_mut()
            .connect("item_activated", &item_activated_handler);
    }
}

#[godot_api]
impl AddNodeTree {
    #[func]
    fn on_item_activated(&mut self) {
        let selected_item = self
            .base()
            .get_selected()
            .expect("Signal Handler got ui_accept so has to be there");
        let text = selected_item.get_text(0);

        let position = self.invoke_position.to_variant();
        self.base_mut()
            .emit_signal("node_selected", &[text.to_variant(), position]);
    }
    #[signal]
    fn node_selected(name: GString, position: Vector2);
}
