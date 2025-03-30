use std::collections::HashMap;

use add_node_popup::AddNodePopup;
use godot::{
    classes::{
        EditorInterface, GraphEdit, IGraphEdit, InputEvent, InputEventKey, ResourceSaver,
        control::{LayoutPreset, SizeFlags},
        graph_edit::GridPattern,
    },
    global::Key,
    prelude::*,
};
use graph_node::OneiroiGraphNode;
use node_proxy::OneiroiNodeProxy;
use oneiroi::{
    asset::{NodeIndex, NodeMetadata},
    data_types::DataTypeType,
    operations::Nodes,
};

use crate::core::asset::OneiroiAsset;

mod add_node_popup;
mod add_node_tree;
mod graph_node;
pub mod node_proxy;

#[derive(GodotClass)]
#[class(tool,internal,no_init, base=GraphEdit)]
pub struct OneiroiNodeEditor {
    base: Base<GraphEdit>,
    node_adder: Gd<AddNodePopup>,
    asset: Gd<OneiroiAsset>,
}

#[godot_api]
impl IGraphEdit for OneiroiNodeEditor {
    fn ready(&mut self) {
        //setup base pbehaviour
        self.base_mut().set_h_size_flags(SizeFlags::EXPAND_FILL);
        self.base_mut().set_anchors_preset(LayoutPreset::FULL_RECT);
        self.base_mut().set_grid_pattern(GridPattern::DOTS);
        self.base_mut().set_right_disconnects(true);

        //enables popup to add node
        let adder = self.node_adder.clone();
        self.base_mut().add_child(&adder);

        let name: GString = self.asset.bind().base().get_path();
        self.base_mut().set_name(&name.get_file().split(".")[0]);

        //get the underlying nodes
        let nodes = self.asset.bind().get_inner().get_nodes();

        //reconstruct saved resource nodes to display nodes
        for node_index in nodes {
            //initialize new node with saved resource
            let graph_node = self.asset.bind().get_inner().get_node(node_index).clone();
            //.into_resource();
            let mut node = OneiroiGraphNode::init_with_node(graph_node, node_index);

            //set the name of the node to the index in the graph
            node.bind_mut()
                .base_mut()
                .set_name(&node_index.index().to_string());

            /*  let on_node_render_toggled = self.base().callable("on_node_render_toggled");
            node.bind_mut()
                .base_mut()
                .connect("render_toggled", &on_node_render_toggled); */

            //add it to the Graph
            self.base_mut().add_child(&node);
        }

        //reconstruct connections to display
        let connections = self.asset.bind().get_inner().get_node_connections();
        for index in connections {
            let nodes = self.asset.bind().get_inner().get_edge_endpoints(index);
            let connection = self.asset.bind().get_inner().get_connection(index);

            //add the conenction in the graph
            self.base_mut().connect_node(
                &nodes.0.index().to_string(),
                connection.port_from() as i32,
                &nodes.1.index().to_string(),
                connection.port_to() as i32,
            );
        }

        //add valid connections to the graph
        self.base_mut()
            .add_valid_connection_type(DataTypeType::Mesh as i32, DataTypeType::Omni as i32);

        //connect the signal from NodeTree
        let node_instantiated_handler = self.base().callable("on_node_instantiated");
        self.node_adder
            .bind_mut()
            .available_nodes_tree
            //TODO maybe rename signal
            .connect("node_selected", &node_instantiated_handler);

        //connect the connection request singal for connection handling
        let on_connection_request = self.base().callable("on_connection_request");
        self.base_mut()
            .connect("connection_request", &on_connection_request);
        //connect the popup_request signal
        let on_popup_request = self.base().callable("on_popup_request");
        self.base_mut().connect("popup_request", &on_popup_request);

        //conenct the node_selected /deselected signals
        let on_node_selected = self.base().callable("on_node_selected");
        self.base_mut().connect("node_selected", &on_node_selected);
        let on_node_deselected = self.base().callable("on_node_deselected");
        self.base_mut()
            .connect("node_deselected", &on_node_deselected);

        let on_nodes_delete_request = self.base().callable("on_nodes_delete_request");
        self.base_mut()
            .connect("delete_nodes_request", &on_nodes_delete_request);

        let on_disconnection_request = self.base().callable("on_disconnection_request");
        self.base_mut()
            .connect("disconnection_request", &on_disconnection_request);
    }

    fn gui_input(&mut self, event: Gd<InputEvent>) {
        //CTRL S resource save
        if let Ok(key) = event.try_cast::<InputEventKey>() {
            if key.is_ctrl_pressed() && key.get_keycode() == Key::S {
                ResourceSaver::singleton().save(&self.asset);
                godot_print!("Resource Saved!");
            }
        }
    }

    fn is_node_hover_valid(
        &mut self,
        from_node: StringName,
        from_port: i32,
        to_node: StringName,
        to_port: i32,
    ) -> bool {
        let fni: u16 = from_node
            .to_string()
            .parse()
            .expect("Parsing to index was not successfull");
        let fni = NodeIndex::from(fni);

        let tni: u16 = to_node
            .to_string()
            .parse()
            .expect("Parsing to index was not successfull");
        let tni = NodeIndex::from(tni);
        !self
            .asset
            .bind()
            .get_inner()
            .allow_connection(fni, from_port as u8, tni, to_port as u8)
    }

    fn physics_process(&mut self, _: f64) {
        self.asset.bind_mut().get_inner_mut().recompute();
    }
}

#[godot_api]
impl OneiroiNodeEditor {
    #[func]
    fn on_node_instantiated(&mut self, alias: GString, position: Vector2) {
        self.node_adder.set_visible(false);

        let index = self
            .asset
            .bind_mut()
            .get_inner_mut()
            .add_node(&alias.to_string());

        let node = self.asset.bind().get_inner().get_node(index).clone();
        node.0.borrow_mut().set_position(oneiroi::asset::Vec2 {
            x: position.x,
            y: position.y,
        });

        //add and register in graph
        /* let mut node: Gd<OneiroiGraphNode> = */
        let mut node = OneiroiGraphNode::init_with_node(node, index);

        //set the name of the node to the index in the graph
        node.bind_mut()
            .base_mut()
            .set_name(&index.index().to_string());

        self.base_mut().add_child(&node);

        // connect the render signal
        /* let on_node_render_toggled = self.base().callable("on_node_render_toggled");
        node.bind_mut()
            .base_mut()
            .connect("render_toggled", &on_node_render_toggled);
        self.base_mut().set_selected(&node);
        self.base_mut().add_child(&node); */
    }

    #[func]
    pub fn init_with_asset(asset: Gd<OneiroiAsset>) -> Gd<OneiroiNodeEditor> {
        Gd::from_init_fn(|base| Self {
            base,
            asset,
            node_adder: AddNodePopup::new_alloc(),
        })
    }

    #[func]
    fn on_connection_request(
        &mut self,
        from_node: StringName,
        from_port: i32,
        to_node: StringName,
        to_port: i32,
    ) {
        let fni: u16 = from_node
            .to_string()
            .parse()
            .expect("Parsing to index was not successfull");
        let fni = NodeIndex::from(fni);

        let tni: u16 = to_node
            .to_string()
            .parse()
            .expect("Parsing to index was not successfull");
        let tni = NodeIndex::from(tni);

        //try committing the connection
        let added_connection = self.asset.bind_mut().get_inner_mut().add_node_connection(
            fni,
            from_port as u8,
            tni,
            to_port as u8,
        );

        if added_connection.is_ok() {
            self.base_mut()
                .connect_node(&from_node, from_port, &to_node, to_port);
        } else {
            godot_error!(
                " Cannot connect {} port {} to {} port {}",
                from_node,
                from_port,
                to_node,
                to_port
            )
        }
    }

    #[func]
    fn on_popup_request(&mut self, request_position: Vector2) {
        let position = request_position + self.base().get_scroll_offset();
        self.node_adder.bind_mut().make_visible(position);
    }

    #[func]
    fn on_node_deselected(&mut self, node: Gd<Node>) {
        //Safety: This clone is necessary! Otherwise godot frees the node/resource for some reason?
        let _ = node
            .clone()
            .cast::<OneiroiGraphNode>()
            .bind()
            .get_resource()
            .clone();
        //let __ = node.clone();
        //let ___ = node.clone();
        //let ____ = node.clone();
        //reset the inspector
        EditorInterface::singleton()
            .get_inspector()
            .unwrap()
            .edit(Gd::null_arg());
        //godot_print!("{:?}", resource.clone());
    }
    #[func]
    fn on_nodes_delete_request(&mut self, nodes: Array<StringName>) {
        godot_print!("{nodes}");
        for node in nodes.iter_shared() {
            let idx = NodeIndex::new(node.to_string().parse::<u16>().unwrap().into());
            let Ok(_) = self.asset.bind_mut().get_inner_mut().delete_node(idx) else {
                return;
            };
            //SAFETY: Node has to be there since it was passed us by Godot in this fn
            let node = self.base_mut().find_child(node.arg()).unwrap();
            self.base_mut().remove_child(&node);
            EditorInterface::singleton().edit_node(Gd::null_arg());
        }
    }

    #[func]
    fn on_node_selected(&mut self, node: Gd<Node>) {
        // Safety: Were only spawning OneiroiGraphNodes in the Editor so cast can be made
        //let index = node.cast::<OneiroiGraphNode>().bind().graph_index;
        //godot_print!("{:?}", node);

        EditorInterface::singleton()
            .edit_resource(&node.get("resource").to::<Gd<OneiroiNodeProxy>>());

        //TODO this was most likely for the Geometry sheet
        /* godot::classes::Engine::singleton()
        .get_singleton("OneiroiEditorServer")
        .expect("Oneiroi Suite Server not available HUH?")
        .cast::<OneiroiEditorServer>()
        .emit_signal("node_selected", &[resource.clone().to_variant()]); */
    }

    #[func]
    fn on_disconnection_request(
        &mut self,
        from_node: StringName,
        from_port: i32,
        to_node: StringName,
        to_port: i32,
    ) {
        let from: u16 = from_node
            .to_string()
            .parse()
            .expect("Parsing to index was not successfull");
        let from = NodeIndex::from(from);

        let to: u16 = to_node
            .to_string()
            .parse()
            .expect("Parsing to index was not successfull");
        let to = NodeIndex::from(to);
        self.base_mut()
            .disconnect_node(&from_node, from_port, &to_node, to_port);
        println!(
            "{:?}",
            self.asset.bind_mut().get_inner_mut().delete_connection(
                from,
                from_port as u8,
                to,
                to_port as u8,
            )
        );
    }
}
