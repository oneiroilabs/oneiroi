use std::{cell::RefCell, rc::Rc};

use godot::{
    classes::{
        Button, CenterContainer, Control, EditorInspector, EditorInterface, GraphNode, IGraphNode,
        StyleBoxFlat, Theme, notify::ContainerNotification,
    },
    prelude::*,
};
use oneiroi::{
    asset::{NodeIndex, NodeMetadata},
    operations::Nodes,
};

use super::node_proxy::OneiroiNodeProxy;

#[derive(GodotClass)]
#[class(tool,no_init,internal, base=GraphNode)]
pub struct OneiroiGraphNode {
    base: Base<GraphNode>,

    #[var]
    resource: Gd<OneiroiNodeProxy>,
}

#[godot_api]
impl IGraphNode for OneiroiGraphNode {
    fn ready(&mut self) {
        //get the metadata of given node
        let base_props = self.resource.bind().get_info();
        let static_metadata = self.resource.bind().static_metadata();

        let color = Color::from_html(static_metadata.color).unwrap();
        //godot_print!("{}, {color:?}", base_props.get_name());
        //TODO notification theme change hook up
        let mut titlebar = EditorInterface::singleton()
            .get_editor_theme()
            .unwrap()
            .get_stylebox("titlebar", "GraphNode")
            .unwrap()
            .duplicate()
            .unwrap()
            .try_cast::<StyleBoxFlat>()
            .unwrap()
            .clone();
        titlebar.set_bg_color(color);

        let mut titlebar_selected = EditorInterface::singleton()
            .get_editor_theme()
            .unwrap()
            .get_stylebox("titlebar_selected", "GraphNode")
            .unwrap()
            .duplicate()
            .unwrap()
            .try_cast::<StyleBoxFlat>()
            .unwrap()
            .clone();

        titlebar_selected.set_bg_color(color.lightened(0.2));

        self.base_mut()
            .set_custom_minimum_size(Vector2 { x: 85., y: 20. });

        self.base_mut()
            .add_theme_stylebox_override("titlebar", &titlebar);
        self.base_mut()
            .add_theme_stylebox_override("titlebar_selected", &titlebar_selected);

        let node_idx = self.resource.bind().get_node_id().index().to_string();

        let title = node_idx + " " + &base_props.get_name();
        self.base_mut().set_title(&title);

        let position = base_props.get_position();
        self.base_mut().set_position_offset(Vector2 {
            x: position.x,
            y: position.y,
        });

        //connect the dragged signal
        let on_dragged = self.base().callable("on_dragged");
        self.base_mut().connect("dragged", &on_dragged);

        //setup slots
        let sockets = self.resource.bind().get_sockets();
        let controls = std::cmp::max(sockets.1.len(), sockets.0.len());
        for _ in 0..controls {
            let mut control = Control::new_alloc();
            control.set_custom_minimum_size(Vector2 { x: 0., y: 25. });
            self.base_mut().add_child(&control);
        }
        for (index, slot) in sockets.1.into_iter().enumerate() {
            self.base_mut().set_slot_enabled_right(index as i32, true);
            let color = slot.get_color();
            self.base_mut()
                .set_slot_color_right(index as i32, Color::from_rgb(color.x, color.y, color.z));
            self.base_mut()
                .set_slot_type_right(index as i32, slot as i32);
        }
        for (index, slot) in sockets.0.into_iter().enumerate() {
            self.base_mut().set_slot_enabled_left(index as i32, true);
            let color = slot.get_color();
            self.base_mut()
                .set_slot_color_left(index as i32, Color::from_rgb(color.x, color.y, color.z));
            self.base_mut()
                .set_slot_type_left(index as i32, slot as i32);
        }

        //TODO fill with preview button
        let mut control = CenterContainer::new_alloc();
        control.set_custom_minimum_size(Vector2 { x: 0., y: 25. });
        self.base_mut().add_child(&control);
        let mut preview_button = Button::new_alloc();
        preview_button.set_text("Preview");
        control.add_child(&preview_button);
        /* self.preview_btn
        .set_pressed_no_signal(result.bind().base_props.bind().is_preview); */
    }

    /* fn gui_input(&mut self, event: Gd<InputEvent>) {
    if let Ok(mouse_event) = event.try_cast::<InputEventMouseButton>() {
        if mouse_event.is_pressed() && mouse_event.get_button_index() == MouseButton::LEFT {
                //TODO most likely remove
                }
            }
        } */

    /* fn on_notification(&mut self, not: ContainerNotification) {
        godot_print!("{not:?}");
    } */
}

#[godot_api]
impl OneiroiGraphNode {
    //#[func]
    //TODO most likely init with index
    pub fn init_with_node(
        node: Rc<(RefCell<NodeMetadata>, RefCell<Nodes>)>,
        node_index: NodeIndex,
    ) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            resource: OneiroiNodeProxy::init_with_node(node, node_index),
        })
    }

    /* pub fn init_with_id(node_index: NodeIndex) -> Gd<Self> {
        //TODO cast to right dings maybe not
        Gd::from_init_fn(|base| Self {
            base,
            resource: OneiroiNodeProxy::init_with_id(node_index),
            // properties: Vec::new(),
            // resource,
            //TODO
            //graph_index: 0.into(),
            // preview_btn: CheckBox::new_alloc(),
            // render_btn: CheckBox::new_alloc(),
        })
    } */

    #[func]
    fn on_dragged(&mut self, _from: Vector2, to: Vector2) {
        // update the graph pos when node position changed
        self.resource
            //has to be there cause all OneiroiNodes should have this -> composition
            .bind_mut()
            .set_position(to)
    }

    /* #[signal]
    fn render_toggled(resource: Gd<Resource>, state: bool);

    #[func]
    fn on_render_toggled(&mut self, state: bool) {
        let res_ref = self.resource.clone();
        godot_print!("huh");
        self.base_mut().emit_signal(
            "render_toggled",
            &[res_ref.to_variant(), state.to_variant()],
        );
    } */
}
