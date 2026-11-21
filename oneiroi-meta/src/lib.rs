//! What people see of nodes, ports, data types and traits: their names,
//! their documentation in markdown, and the colors of types.
//!
//! Nothing here is needed to run a graph, so none of it lives with what
//! runs. Everything is looked up by the ids which are there already: the
//! name of a type or trait, the id of a node, the name of a port.
//!
//! What an editor draws of the inputs of a node in its viewport, and how
//! dragging changes them, is described here as well: see [`gizmo`].
//!
//! Documentation is markdown. Its first paragraph is a summary, short
//! enough for a tooltip, and the rest goes into detail.

use std::collections::HashMap;

pub mod gizmo;

use gizmo::GizmoMeta;

/// The hues of the Tailwind palette. Types are drawn in the shade 300 of
/// their hue, a pastel. Similar types take neighbouring hues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Hue {
    Red,
    Orange,
    Amber,
    Yellow,
    Lime,
    Green,
    Emerald,
    Teal,
    Cyan,
    Sky,
    Blue,
    Indigo,
    Violet,
    Purple,
    Fuchsia,
    Pink,
    Rose,
    Slate,
    Gray,
    Zinc,
    Neutral,
    Stone,
}

impl Hue {
    pub const ALL: [Hue; 22] = [
        Hue::Red,
        Hue::Orange,
        Hue::Amber,
        Hue::Yellow,
        Hue::Lime,
        Hue::Green,
        Hue::Emerald,
        Hue::Teal,
        Hue::Cyan,
        Hue::Sky,
        Hue::Blue,
        Hue::Indigo,
        Hue::Violet,
        Hue::Purple,
        Hue::Fuchsia,
        Hue::Pink,
        Hue::Rose,
        Hue::Slate,
        Hue::Gray,
        Hue::Zinc,
        Hue::Neutral,
        Hue::Stone,
    ];

    /// The name in the palette, like `sky`.
    pub fn name(self) -> &'static str {
        match self {
            Hue::Red => "red",
            Hue::Orange => "orange",
            Hue::Amber => "amber",
            Hue::Yellow => "yellow",
            Hue::Lime => "lime",
            Hue::Green => "green",
            Hue::Emerald => "emerald",
            Hue::Teal => "teal",
            Hue::Cyan => "cyan",
            Hue::Sky => "sky",
            Hue::Blue => "blue",
            Hue::Indigo => "indigo",
            Hue::Violet => "violet",
            Hue::Purple => "purple",
            Hue::Fuchsia => "fuchsia",
            Hue::Pink => "pink",
            Hue::Rose => "rose",
            Hue::Slate => "slate",
            Hue::Gray => "gray",
            Hue::Zinc => "zinc",
            Hue::Neutral => "neutral",
            Hue::Stone => "stone",
        }
    }
}

/// A data type, by the name it is registered under.
#[derive(Debug, Clone, Copy)]
pub struct TypeMeta {
    pub name: &'static str,
    pub label: &'static str,
    pub hue: Hue,
    pub doc: &'static str,
}

/// A trait, by its name.
#[derive(Debug, Clone, Copy)]
pub struct TraitMeta {
    pub name: &'static str,
    pub label: &'static str,
    pub hue: Hue,
    pub doc: &'static str,
}

/// A node or a generic node, by its id. The versions of a generic node are
/// described by it.
#[derive(Debug, Clone, Copy)]
pub struct NodeMeta {
    pub id: &'static str,
    pub label: &'static str,
    /// Where it is found in a palette, like `Curve`.
    pub category: &'static str,
    pub doc: &'static str,
    pub inputs: &'static [PortMeta],
    pub outputs: &'static [PortMeta],
}

/// A port of a node, by its name: how the node uses it.
#[derive(Debug, Clone, Copy)]
pub struct PortMeta {
    pub name: &'static str,
    /// Empty to make one of the name.
    pub label: &'static str,
    pub doc: &'static str,
}

impl PortMeta {
    /// The label, or one made of the name: `ring_count` is `Ring count`.
    pub fn label(&self) -> String {
        match self.label.is_empty() {
            true => label_of(self.name),
            false => self.label.into(),
        }
    }
}

/// A name for people made of an identifier: `ring_count` is `Ring count`.
pub fn label_of(name: &str) -> String {
    let mut label = name.replace(['_', '-'], " ");
    if let Some(first) = label.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    label
}

/// Documentation as written in doc comments, as markdown: the space every
/// line starts with in a doc comment taken off.
pub fn markdown(doc: &str) -> String {
    doc.lines()
        .map(|line| line.strip_prefix(' ').unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// The first paragraph of some documentation: what a tooltip shows.
pub fn summary(doc: &str) -> String {
    let doc = markdown(doc);
    doc.split("\n\n")
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Everything that is described, by id.
#[derive(Debug, Default, Clone)]
pub struct MetaRegistry {
    types: HashMap<&'static str, TypeMeta>,
    traits: HashMap<&'static str, TraitMeta>,
    nodes: HashMap<&'static str, NodeMeta>,
    gizmos: HashMap<&'static str, GizmoMeta>,
}

impl MetaRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_type(&mut self, meta: TypeMeta) {
        self.types.insert(meta.name, meta);
    }

    pub fn add_trait(&mut self, meta: TraitMeta) {
        self.traits.insert(meta.name, meta);
    }

    pub fn add_node(&mut self, meta: NodeMeta) {
        self.nodes.insert(meta.id, meta);
    }

    pub fn add_gizmo(&mut self, gizmo: GizmoMeta) {
        self.gizmos.insert(gizmo.node, gizmo);
    }

    /// The gizmo of the node `id`, if it has one.
    pub fn gizmo(&self, id: &str) -> Option<&GizmoMeta> {
        self.gizmos.get(id)
    }

    pub fn ty(&self, name: &str) -> Option<&TypeMeta> {
        self.types.get(name)
    }

    pub fn tr(&self, name: &str) -> Option<&TraitMeta> {
        self.traits.get(name)
    }

    /// A node, or for a version of a generic node, which is called like
    /// `curve.sweep<circle>`, the generic node.
    pub fn node(&self, id: &str) -> Option<&NodeMeta> {
        self.nodes.get(id).or_else(|| {
            let generic = id.split_once('<')?.0;
            self.nodes.get(generic)
        })
    }

    pub fn types(&self) -> impl Iterator<Item = &TypeMeta> {
        self.types.values()
    }

    pub fn traits(&self) -> impl Iterator<Item = &TraitMeta> {
        self.traits.values()
    }

    pub fn nodes(&self) -> impl Iterator<Item = &NodeMeta> {
        self.nodes.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_comments_become_markdown() {
        let doc = " The first line.\n continued.\n\n More `detail`.\n";
        assert_eq!(
            markdown(doc),
            "The first line.\ncontinued.\n\nMore `detail`."
        );
        assert_eq!(summary(doc), "The first line.\ncontinued.");
    }

    #[test]
    fn labels_are_made_of_names() {
        assert_eq!(label_of("ring_count"), "Ring count");
        assert_eq!(label_of("t"), "T");
        let port = PortMeta {
            name: "points",
            label: "Control points",
            doc: "",
        };
        assert_eq!(port.label(), "Control points");
    }

    #[test]
    fn versions_are_described_by_their_generic_node() {
        let mut meta = MetaRegistry::new();
        meta.add_node(NodeMeta {
            id: "curve.sweep",
            label: "Sweep",
            category: "Curve",
            doc: "",
            inputs: &[],
            outputs: &[],
        });
        assert_eq!(meta.node("curve.sweep<circle>").unwrap().label, "Sweep");
        assert!(meta.node("curve.nope").is_none());
    }
}
