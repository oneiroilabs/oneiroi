use std::collections::HashMap;

use iced::widget::{
    button, column, container, pick_list, row, scrollable, shader, space, text, text_input,
};
use iced::{Center, Color, Element, Fill, Point, Theme, Vector};
use iced_nodegraph::{
    ColorQuad, Keymap, NodeGraph, NodeStyle, PinDirection, PinRef, default_can_connect,
    default_node_style, edge, node, node_header, pin,
};
use oneiroi_editor::document::{Document, PinType};
use oneiroi_editor::preview::Preview;
use oneiroi_graph::{Input, NodeId};

/// Where a graph is saved to and loaded from, next to where the editor runs.
const FILE: &str = "graph.json";

fn main() -> iced::Result {
    env_logger::init();

    iced::application(Editor::new, Editor::update, Editor::view)
        .title("Oneiroi")
        .theme(Editor::theme)
        .window_size((1280.0, 720.0))
        .run()
}

type Pin = PinRef<u32, String>;

#[derive(Debug, Clone)]
enum Message {
    Connected(Pin, Pin),
    Disconnected(Pin, Pin),
    Moved(Vector, Vec<u32>),
    Deleted(Vec<u32>),
    Selected(Vec<u32>),
    Panned(Point, f32),
    EdgesCut(Vec<String>),
    Add(&'static str),
    Display(u32),
    Edited(u32, String, String),
    Committed(u32, String),
    Reset(u32, String),
    Frame,
    Save,
    Load,
}

struct Editor {
    document: Document,
    selected: Vec<NodeId>,
    /// Pan and zoom of the graph.
    camera: (Point, f32),
    /// What is typed into an input and not committed yet.
    edits: HashMap<(NodeId, String), String>,
    /// Counts up to aim the preview at the mesh again.
    frame: u64,
    /// What the last save, load or edit had to say.
    note: Option<String>,
}

/// The pins of a node are named after its ports. An input and an output
/// may share a name, so they get told apart.
fn input_pin(name: &str) -> String {
    format!("in:{name}")
}

fn output_pin(name: &str) -> String {
    format!("out:{name}")
}

/// Sorts the two ends of a link into `(node, output)` and `(node, input)`.
fn ends(a: &Pin, b: &Pin) -> Option<((NodeId, String), (NodeId, String))> {
    let output = |pin: &Pin| Some((pin.node_id.into(), pin.pin_id.strip_prefix("out:")?.to_owned()));
    let input = |pin: &Pin| Some((pin.node_id.into(), pin.pin_id.strip_prefix("in:")?.to_owned()));
    output(a)
        .zip(input(b))
        .or_else(|| output(b).zip(input(a)))
}

/// A colour per type, the same every time.
fn pin_color(ty: &PinType) -> Color {
    let hash = ty
        .name()
        .bytes()
        .fold(2166136261u32, |hash, byte| (hash ^ byte as u32).wrapping_mul(16777619));
    let hue = (hash % 360) as f32;
    let channel = |offset: f32| {
        let k = (offset + hue / 60.0) % 6.0;
        0.85 - 0.5 * k.min(4.0 - k).clamp(0.0, 1.0)
    };
    Color::from_rgb(channel(5.0), channel(3.0), channel(1.0))
}

impl Editor {
    fn new() -> Self {
        Self {
            document: Document::example(),
            selected: Vec::new(),
            camera: (Point::ORIGIN, 1.0),
            edits: HashMap::new(),
            frame: 0,
            note: None,
        }
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Message) {
        self.note = None;
        match message {
            Message::Connected(a, b) => {
                if let Some(((from, output), (to, input))) = ends(&a, &b) {
                    self.document.connect(from, &output, to, &input);
                }
            }
            Message::Disconnected(a, b) => {
                if let Some(((from, output), (to, input))) = ends(&a, &b) {
                    self.document.disconnect(from, &output, to, &input);
                }
            }
            Message::Moved(delta, nodes) => {
                let nodes: Vec<_> = nodes.into_iter().map(NodeId::from).collect();
                self.document.move_nodes(&nodes, [delta.x, delta.y]);
            }
            Message::Deleted(nodes) => {
                let nodes: Vec<_> = nodes.into_iter().map(NodeId::from).collect();
                self.document.remove_nodes(&nodes);
                self.selected.retain(|node| !nodes.contains(node));
            }
            Message::Selected(nodes) => {
                self.selected = nodes.into_iter().map(NodeId::from).collect();
            }
            Message::Panned(position, zoom) => self.camera = (position, zoom),
            Message::EdgesCut(edges) => {
                for edge in edges {
                    if let Some((node, input)) = edge.split_once(':')
                        && let Ok(node) = node.parse::<u32>()
                    {
                        self.document.reset(node.into(), input);
                    }
                }
            }
            Message::Add(id) => {
                // Roughly the middle of what is visible of the graph.
                let (pan, zoom) = self.camera;
                let position = [300.0 / zoom - pan.x, 200.0 / zoom - pan.y];
                let node = self.document.add_node(id, position);
                self.selected = vec![node];
            }
            Message::Display(node) => {
                let had_mesh = self.document.mesh().is_some();
                self.document.set_display(Some(node.into()));
                if !had_mesh {
                    self.frame += 1;
                }
            }
            Message::Edited(node, input, text) => {
                self.edits.insert((node.into(), input), text);
            }
            Message::Committed(node, input) => {
                let key = (NodeId::from(node), input);
                if let Some(text) = self.edits.remove(&key)
                    && !self.document.set_input(key.0, &key.1, &text)
                {
                    self.note = Some(format!("`{text}` is not a value for `{}`", key.1));
                }
            }
            Message::Reset(node, input) => {
                self.edits.remove(&(node.into(), input.clone()));
                self.document.reset(node.into(), &input);
            }
            Message::Frame => self.frame += 1,
            Message::Save => {
                self.note = Some(match std::fs::write(FILE, self.document.to_json()) {
                    Ok(()) => format!("saved to {FILE}"),
                    Err(error) => format!("could not save to {FILE}: {error}"),
                });
            }
            Message::Load => {
                let loaded = std::fs::read_to_string(FILE)
                    .map_err(|error| error.to_string())
                    .and_then(|json| Document::from_json(&json).map_err(|error| error.to_string()));
                self.note = Some(match loaded {
                    Ok(document) => {
                        self.document = document;
                        self.selected.clear();
                        self.edits.clear();
                        self.frame += 1;
                        format!("loaded {FILE}")
                    }
                    Err(error) => format!("could not load {FILE}: {error}"),
                });
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let toolbar = row![
            pick_list(None::<&'static str>, self.document.palette(), |id| id.to_string())
                .placeholder("Add node")
                .on_select(Message::Add),
            button("Save").on_press(Message::Save),
            button("Load").on_press(Message::Load),
            button("Frame").on_press(Message::Frame),
            text(self.note.clone().unwrap_or_else(|| self.document.status())).size(13),
        ]
        .spacing(8)
        .align_y(Center);

        let preview = shader(Preview {
            mesh: self.document.mesh().cloned(),
            frame: self.frame,
        })
        .width(Fill)
        .height(Fill);

        // The inspector has to come before the graph: a key typed into one
        // of its fields is then taken by the field, and the graph only
        // treats it as a shortcut (like deleting the selection) otherwise.
        let side = column![preview, self.inspector()].spacing(8).width(420);
        let content = row![side, column![toolbar, self.graph()].spacing(8)].spacing(8);
        container(content).padding(8).into()
    }

    fn graph(&self) -> Element<'_, Message> {
        let document = &self.document;
        let mut graph: NodeGraph<'_, u32, String, PinType, Message, iced::Renderer, String> =
            NodeGraph::default()
                .view(self.camera.0, self.camera.1)
                .on_connect(Message::Connected)
                .on_disconnect(Message::Disconnected)
                .on_move(Message::Moved)
                .on_delete(Message::Deleted)
                .on_select(Message::Selected)
                .on_pan(Message::Panned)
                .on_edge_delete(Message::EdgesCut)
                // Selecting everything would fire while selecting the text
                // of an input, and swap the inspector to another node.
                .keymap(Keymap {
                    select_all: None,
                    ..Keymap::default()
                })
                .can_connect(move |a, b| {
                    let (from, to) = match a.direction() {
                        PinDirection::Output => (a, b),
                        _ => (b, a),
                    };
                    let output = from.pin_id().strip_prefix("out:");
                    let input = to.pin_id().strip_prefix("in:");
                    default_can_connect(a, b)
                        && output.zip(input).is_some_and(|(output, input)| {
                            let (from, to) = ((*from.node_id()).into(), (*to.node_id()).into());
                            document.accepts(from, output, to, input)
                        })
                })
                .width(Fill)
                .height(Fill);

        for (id, data) in document.graph().nodes() {
            let Some(view) = document.view(id) else {
                continue;
            };
            let displayed = document.graph().display == Some(id);
            let broken = !view.known || document.error().is_some_and(|e| e.node == Some(id));

            let header = node_header(
                container(
                    row![
                        text(view.title).size(13).color(Color::WHITE),
                        space::horizontal(),
                        button(text(if displayed { "shown" } else { "show" }).size(11))
                            .padding([1, 6])
                            .on_press(Message::Display(id.raw())),
                    ]
                    .align_y(Center),
                )
                .padding([4, 8]),
                if displayed {
                    Color::from_rgb(0.15, 0.35, 0.6)
                } else {
                    Color::BLACK
                },
                1.0,
            );

            let mut pins = column![].spacing(4);
            for port in view.inputs {
                let label = text(port.name.clone()).size(12);
                pins = pins.push(
                    container(pin!(Left, input_pin(&port.name), label, Input, port.ty))
                        .width(Fill)
                        .align_left(Fill),
                );
            }
            for port in view.outputs {
                let label = text(port.name.clone()).size(12);
                pins = pins.push(
                    container(pin!(Right, output_pin(&port.name), label, Output, port.ty))
                        .width(Fill)
                        .align_right(Fill),
                );
            }

            let body = column![header, container(pins).padding([8, 10])].width(190);
            let position = Point::new(data.meta.position[0], data.meta.position[1]);
            graph.push_node(
                node(id.raw(), position, body)
                    .selected(self.selected.contains(&id))
                    .style(move |theme, status| {
                        let base = default_node_style(theme, status);
                        if broken {
                            NodeStyle {
                                border_color: ColorQuad::solid(Color::from_rgb(0.9, 0.2, 0.2)),
                                ..base
                            }
                        } else {
                            base
                        }
                    })
                    .pin_style(|theme, pin, _, status| iced_nodegraph::PinStyle {
                        color: ColorQuad::solid(pin_color(pin.info())),
                        ..iced_nodegraph::default_pin_style(theme, status)
                    }),
            );

            for (name, input) in &data.inputs {
                if let Input::Link { node, output } = input {
                    graph.push_edge(edge!(
                        PinRef::new(node.raw(), output_pin(output)),
                        PinRef::new(id.raw(), input_pin(name)),
                        format!("{}:{name}", id.raw()),
                    ));
                }
            }
        }

        container(graph).width(Fill).height(Fill).into()
    }

    /// The inputs of the selected node, to type values into.
    fn inspector(&self) -> Element<'_, Message> {
        let selected = self.selected.first().copied();
        let Some((node, view)) = selected.and_then(|node| Some((node, self.document.view(node)?)))
        else {
            return container(text("Select a node to edit its inputs.").size(13))
                .height(260)
                .into();
        };

        let mut rows = column![text(view.title).size(15)].spacing(6);
        for port in &view.inputs {
            let key = (node, port.name.clone());
            let (stored, editable) = self
                .document
                .input_text(node, &port.name)
                .unwrap_or_default();
            let label = text(format!("{} ({})", port.name, port.ty.name()))
                .size(12)
                .width(150);

            let field: Element<'_, Message> = if editable {
                let name = port.name.clone();
                let shown = self.edits.get(&key).cloned().unwrap_or(stored);
                text_input("", shown)
                    .size(12)
                    .on_input(move |text| Message::Edited(node.raw(), name.clone(), text))
                    .on_submit(Message::Committed(node.raw(), port.name.clone()))
                    .into()
            } else {
                text(stored).size(12).width(Fill).into()
            };
            let reset = button(text("reset").size(11))
                .padding([1, 6])
                .on_press(Message::Reset(node.raw(), port.name.clone()));
            rows = rows.push(row![label, field, reset].spacing(6).align_y(Center));
        }
        container(scrollable(rows)).height(260).into()
    }
}
