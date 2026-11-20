use std::collections::HashMap;

use iced::widget::{
    button, checkbox, column, container, markdown, pick_list, row, scrollable, shader, space,
    text, text_input, tooltip,
};
use iced::{Center, Color, Element, Fill, Point, Theme, Vector};
use iced_nodegraph::{
    ColorQuad, EdgeStyle, Keymap, NodeGraph, NodeStyle, Pattern, PinDirection, PinRef,
    default_can_connect, default_edge_style,
    default_node_style, edge, node, node_header, pin,
};
use oneiroi_editor::document::{Document, PaletteEntry, Side};
use oneiroi_editor::look::{self, PinLook};
use oneiroi_graph::Note;
use oneiroi_editor::preview::{Detail, Preview};
use oneiroi_graph::{Input, NodeId};

/// How finely the preview draws surfaces, from fine to coarse, and last
/// as fine as a budget of triangles allows.
const DETAIL: [(&str, Detail); 5] = [
    ("finest", Detail::Pixels(4.0)),
    ("fine", Detail::Pixels(8.0)),
    ("coarse", Detail::Pixels(16.0)),
    ("coarsest", Detail::Pixels(32.0)),
    ("20k triangles", Detail::Triangles(20_000)),
];

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
    Add(PaletteEntry),
    Open(String),
    NewGraph,
    Expose(u32, String),
    Export(u32, String),
    Each(u32, String),
    Display(u32),
    Edited(u32, String, String),
    Committed(u32, String),
    /// One of the choices of an input to pick from was picked.
    Chosen(u32, String, String),
    Reset(u32, String),
    Frame,
    Detail(usize),
    ShowDetail(bool),
    Save,
    Load,
    /// Whether the inspector shows the notes of the ports to edit.
    EditNotes(bool),
    NoteLabel(u32, Side, String, String),
    NoteDoc(u32, Side, String, String),
    /// What the open graph says about itself.
    GraphDoc(String),
    /// A link in documentation was clicked, which leads nowhere yet.
    Link,
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
    /// Which of [`DETAIL`] the preview uses.
    detail: usize,
    show_detail: bool,
    edit_notes: bool,
    /// The documentation shown in tooltips, parsed, by its markdown. Made
    /// anew after every update, since a view can only borrow it.
    docs: HashMap<String, markdown::Content>,
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

/// What the tooltip of a node title says, as markdown.
fn node_tip(title: &str, doc: &str) -> String {
    match doc.trim().is_empty() {
        true => format!("**{title}**"),
        false => format!("**{title}**\n\n{}", doc.trim()),
    }
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
            detail: 1,
            show_detail: false,
            edit_notes: false,
            docs: HashMap::new(),
        }
        .with_docs()
    }

    fn with_docs(mut self) -> Self {
        self.parse_docs();
        self
    }

    /// Parses the documentation of everything on the screen which is not
    /// parsed yet, and forgets what is not on it anymore.
    fn parse_docs(&mut self) {
        let mut shown = Vec::new();
        for (id, _) in self.document.graph().nodes() {
            let Some(view) = self.document.view(id) else {
                continue;
            };
            shown.push(node_tip(&view.title, &view.doc));
            for port in view.inputs.iter().chain(&view.outputs) {
                shown.push(look::port_tip(&self.document, port));
            }
        }
        self.docs.retain(|doc, _| shown.contains(doc));
        for doc in shown {
            self.docs.entry(doc).or_insert_with_key(|doc| markdown::Content::parse(doc));
        }
    }

    /// `doc` rendered, in a box to float over the graph.
    fn tip(&self, doc: &str) -> Element<'_, Message> {
        let content: Element<'_, Message> = match self.docs.get(doc) {
            Some(content) => markdown::view(content.items(), markdown::Settings::with_text_size(12, Theme::Dark))
                .map(|_| Message::Link),
            None => text(doc.to_string()).size(12).into(),
        };
        container(content)
            .width(320)
            .padding([6, 8])
            .style(container::rounded_box)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Message) {
        self.on(message);
        self.parse_docs();
    }

    fn on(&mut self, message: Message) {
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
            Message::Add(entry) => {
                // Roughly the middle of what is visible of the graph.
                let (pan, zoom) = self.camera;
                let position = [300.0 / zoom - pan.x, 200.0 / zoom - pan.y];
                let node = self.document.add(entry.kind, position);
                self.selected = vec![node];
            }
            Message::Open(name) => {
                if self.document.open(&name) {
                    self.switched();
                }
            }
            Message::NewGraph => {
                self.document.new_graph();
                self.switched();
            }
            Message::Expose(node, input) => {
                self.edits.remove(&(node.into(), input.clone()));
                self.document.toggle_exposed(node.into(), &input);
            }
            Message::Export(node, output) => {
                self.document.toggle_output(node.into(), &output);
            }
            Message::Each(node, input) => {
                self.document.toggle_each(node.into(), &input);
            }
            Message::Display(node) => {
                // Another node is another thing to look at, which can be
                // anywhere and of any size. Edits leave the camera alone.
                if self.document.graph().display != Some(node.into()) {
                    self.document.set_display(Some(node.into()));
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
            Message::Chosen(node, input, choice) => {
                self.edits.remove(&(node.into(), input.clone()));
                self.document.choose(node.into(), &input, &choice);
            }
            Message::Reset(node, input) => {
                self.edits.remove(&(node.into(), input.clone()));
                self.document.reset(node.into(), &input);
            }
            Message::Frame => self.frame += 1,
            Message::Detail(detail) => {
                self.detail = detail;
                // A budget needs some frames to find its detail.
                self.document.session().lock().unwrap().settling = 30;
            }
            Message::ShowDetail(show) => self.show_detail = show,
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
                        self.switched();
                        format!("loaded {FILE}")
                    }
                    Err(error) => format!("could not load {FILE}: {error}"),
                });
            }
            Message::EditNotes(edit) => self.edit_notes = edit,
            Message::NoteLabel(node, side, port, label) => {
                let note = self.document.note(node.into(), side, &port);
                let note = Note { label: Some(label), ..note };
                self.document.set_note(node.into(), side, &port, note);
            }
            Message::NoteDoc(node, side, port, doc) => {
                let note = self.document.note(node.into(), side, &port);
                let note = Note { doc: Some(doc), ..note };
                self.document.set_note(node.into(), side, &port, note);
            }
            Message::GraphDoc(doc) => {
                let mut about = self.document.about().clone();
                about.doc = Some(doc).filter(|doc| !doc.trim().is_empty());
                self.document.set_about(about);
            }
            Message::Link => {}
        }
    }

    /// Another graph is on the screen: nothing of the old one applies.
    fn switched(&mut self) {
        self.selected.clear();
        self.edits.clear();
        self.camera = (Point::ORIGIN, 1.0);
        self.frame += 1;
    }

    fn view(&self) -> Element<'_, Message> {
        let toolbar = row![
            pick_list(
                Some(self.document.current().to_string()),
                self.document.graphs(),
                |name| name.clone()
            )
            .on_select(Message::Open),
            button("New graph").on_press(Message::NewGraph),
            pick_list(None::<PaletteEntry>, self.document.palette(), |entry| entry.to_string())
                .placeholder("Add node")
                .on_select(Message::Add),
            button("Save").on_press(Message::Save),
            button("Load").on_press(Message::Load),
            button("Frame").on_press(Message::Frame),
            text(self.note.clone().unwrap_or_else(|| self.document.status())).size(13),
        ]
        .spacing(8)
        .align_y(Center);
        let legend = text(
            "●  one     [●]  a list     {●}  a list of groups     filled: exactly this type     ring: any type of a trait     dotted link: instances copied",
        )
        .size(11)
        .color(Color::from_rgb(0.6, 0.6, 0.65));

        let preview = shader(Preview {
            session: self.document.session(),
            frame: self.frame,
            detail: DETAIL[self.detail].1,
            show_detail: self.show_detail,
        })
        .width(Fill)
        .height(Fill);

        let mut detail = row![text("Detail").size(12)].spacing(4).align_y(Center);
        for (index, (label, _)) in DETAIL.into_iter().enumerate() {
            let button = button(text(label).size(11)).padding([1, 6]);
            detail = detail.push(match index == self.detail {
                true => button,
                false => button.on_press(Message::Detail(index)),
            });
        }
        let detail = detail.push(
            checkbox(self.show_detail)
                .label("colours")
                .text_size(11)
                .on_toggle(Message::ShowDetail),
        );

        let preview = column![preview, detail].spacing(4);

        // The inspector has to come before the graph: a key typed into one
        // of its fields is then taken by the field, and the graph only
        // treats it as a shortcut (like deleting the selection) otherwise.
        let side = column![preview, self.inspector()].spacing(8).width(420);
        let content = row![side, column![toolbar, legend, self.graph()].spacing(8)].spacing(8);
        container(content).padding(8).into()
    }

    fn graph(&self) -> Element<'_, Message> {
        let document = &self.document;
        let mut graph: NodeGraph<'_, u32, String, PinLook, Message, iced::Renderer, String> =
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
                        tooltip(
                            text(view.title.clone()).size(13).color(Color::WHITE),
                            self.tip(&node_tip(&view.title, &view.doc)),
                            tooltip::Position::Top,
                        ),
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
            for port in &view.inputs {
                let tip = self.tip(&look::port_tip(document, port));
                let label = container(text(port.label.clone()).size(12)).padding(iced::Padding::ZERO.left(6));
                let label = tooltip(label, tip, tooltip::Position::Left);
                let look = look::pin_look(document, port);
                pins = pins.push(
                    container(pin!(Left, input_pin(&port.name), label, Input, look))
                        .width(Fill)
                        .align_left(Fill),
                );
            }
            for port in &view.outputs {
                let tip = self.tip(&look::port_tip(document, port));
                let label = container(text(port.label.clone()).size(12)).padding(iced::Padding::ZERO.right(6));
                let label = tooltip(label, tip, tooltip::Position::Right);
                let look = look::pin_look(document, port);
                pins = pins.push(
                    container(pin!(Right, output_pin(&port.name), label, Output, look))
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
                    .pin_style(|theme, pin, _, status| {
                        let look: &PinLook = pin.info();
                        iced_nodegraph::PinStyle {
                            color: ColorQuad::solid(look.color),
                            brackets: look.brackets,
                            fill: look.fill,
                            ..iced_nodegraph::default_pin_style(theme, status)
                        }
                    }),
            );

            for (name, input) in &data.inputs {
                if let Input::Link { node, output, each } = input {
                    let edge = edge!(
                        PinRef::new(node.raw(), output_pin(output)),
                        PinRef::new(id.raw(), input_pin(name)),
                        format!("{}:{name}", id.raw()),
                    );
                    // A link a graph runs for each element of is dashed, one
                    // which copies instances into their shape dotted: both
                    // cost work for every element.
                    let realizes = document.realizes(*node, output, id, name);
                    let edge = match (each, realizes) {
                        (true, _) => edge.style(|theme, status, _, _| EdgeStyle {
                            pattern: Pattern::dashed(2.5, 8.0, 5.0),
                            ..default_edge_style(theme, status)
                        }),
                        (false, true) => edge.style(|theme, status, _, _| EdgeStyle {
                            pattern: Pattern::dashed(2.5, 2.5, 3.5),
                            ..default_edge_style(theme, status)
                        }),
                        (false, false) => edge,
                    };
                    graph.push_edge(edge);
                }
            }
        }

        container(graph).width(Fill).height(Fill).into()
    }

    /// The inputs of the selected node, to type values into.
    fn inspector(&self) -> Element<'_, Message> {
        let selected = self.selected.first().copied();
        let notes = checkbox(self.edit_notes)
            .label("notes")
            .text_size(11)
            .on_toggle(Message::EditNotes);
        let Some((node, view)) = selected.and_then(|node| Some((node, self.document.view(node)?)))
        else {
            // Without a node, what the graph says about itself where it is
            // used in another.
            let mut rows = column![row![text("Select a node to edit its inputs.").size(13), space::horizontal(), notes]]
                .spacing(6);
            if self.edit_notes {
                let doc = self.document.about().doc.clone().unwrap_or_default();
                rows = rows.push(
                    text_input("What this graph does, where it is used as a node (markdown)", doc)
                        .size(12)
                        .on_input(Message::GraphDoc),
                );
            }
            return container(rows).height(260).into();
        };

        let title = tooltip(
            text(view.title.clone()).size(15),
            self.tip(&node_tip(&view.title, &view.doc)),
            tooltip::Position::Bottom,
        );
        let mut rows = column![row![title, space::horizontal(), notes].align_y(Center)].spacing(6);
        for port in &view.inputs {
            let key = (node, port.name.clone());
            let (stored, editable) = self
                .document
                .input_text(node, &port.name)
                .unwrap_or_default();
            let label = tooltip(
                text(port.label.clone()).size(12).width(150),
                self.tip(&look::port_tip(&self.document, port)),
                tooltip::Position::Right,
            );

            let choice = self.document.input_choice(node, &port.name);
            let field: Element<'_, Message> = if let Some((choices, picked)) = choice {
                let name = port.name.clone();
                let picked = choices[picked].clone();
                pick_list(Some(picked), choices, |choice: &String| choice.clone())
                    .text_size(12)
                    .on_select(move |choice| Message::Chosen(node.raw(), name.clone(), choice))
                    .into()
            } else if editable {
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
            let small = |label: &'static str, message| {
                button(text(label).size(11)).padding([1, 6]).on_press(message)
            };
            // An exposed input is a parameter of the graph, which is what
            // another graph sees of it as an input.
            let exposed = self.document.is_exposed(node, &port.name);
            let expose = small(
                if exposed { "exposed" } else { "expose" },
                Message::Expose(node.raw(), port.name.clone()),
            );
            let reset = small("reset", Message::Reset(node.raw(), port.name.clone()));
            // A graph can run once for each element of what is linked.
            let middle: Element<'_, Message> = if self.document.can_be_each(node, &port.name) {
                let each = self.document.is_each(node, &port.name);
                small(
                    if each { "each element" } else { "all at once" },
                    Message::Each(node.raw(), port.name.clone()),
                )
                .into()
            } else {
                expose.into()
            };
            rows = rows.push(row![label, field, middle, reset].spacing(6).align_y(Center));
            if self.edit_notes {
                rows = rows.push(self.note_fields(node, Side::Input, port));
            }
        }
        for port in &view.outputs {
            let label = tooltip(
                text(port.label.clone()).size(12).width(Fill),
                self.tip(&look::port_tip(&self.document, port)),
                tooltip::Position::Right,
            );
            // An exported output is one of the whole graph.
            let exported = self.document.is_output(node, &port.name);
            let export = button(text(if exported { "exported" } else { "export" }).size(11))
                .padding([1, 6])
                .on_press(Message::Export(node.raw(), port.name.clone()));
            rows = rows.push(row![label, export].spacing(6).align_y(Center));
            if self.edit_notes {
                rows = rows.push(self.note_fields(node, Side::Output, port));
            }
        }
        container(scrollable(rows)).height(260).into()
    }

    /// Fields for another name of `port` and how this node uses it. Left
    /// empty, the port is described as its node describes it.
    fn note_fields(
        &self,
        node: NodeId,
        side: Side,
        port: &oneiroi_editor::document::PortView,
    ) -> Element<'_, Message> {
        let note = self.document.note(node, side, &port.name);
        let name = port.name.clone();
        let label = text_input(port.label.clone(), note.label.unwrap_or_default())
            .size(11)
            .width(150)
            .on_input(move |label| Message::NoteLabel(node.raw(), side, name.clone(), label));
        let name = port.name.clone();
        let doc = text_input("How it is used here (markdown)", note.doc.unwrap_or_default())
            .size(11)
            .on_input(move |doc| Message::NoteDoc(node.raw(), side, name.clone(), doc));
        row![label, doc].spacing(6).padding([0, 0]).into()
    }
}
