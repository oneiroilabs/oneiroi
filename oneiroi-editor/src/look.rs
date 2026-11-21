//! How what a pin carries looks. It is always a circle: brackets beside it
//! say how many, its fill whether it takes one type or any of a trait, and
//! its colour which type.

use iced::Color;
use iced_nodegraph::{PinBrackets, PinFill};
use oneiroi_core::node::{Card, Items};
use oneiroi_nodes::meta::Hue;

use crate::document::{Document, PinType, PortView};

/// The colour of the label of a number: red, green and blue for `x`, `y`
/// and `z`, as axes are told apart by convention, and grey for anything
/// else.
pub fn axis_color(axis: Option<&str>) -> Color {
    let rgb = match axis {
        Some("x") => 0xf87171,
        Some("y") => 0x4ade80,
        Some("z") => 0x60a5fa,
        _ => 0xa1a1aa,
    };
    Color::from_rgb8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// The colour of `hue`, its shade 300 of the palette of Tailwind CSS: light
/// enough to read on a dark node, and still telling the hues apart.
pub fn color(hue: Hue) -> Color {
    let rgb = match hue {
        Hue::Red => 0xfca5a5,
        Hue::Orange => 0xfdba74,
        Hue::Amber => 0xfcd34d,
        Hue::Yellow => 0xfde047,
        Hue::Lime => 0xbef264,
        Hue::Green => 0x86efac,
        Hue::Emerald => 0x6ee7b7,
        Hue::Teal => 0x5eead4,
        Hue::Cyan => 0x67e8f9,
        Hue::Sky => 0x7dd3fc,
        Hue::Blue => 0x93c5fd,
        Hue::Indigo => 0xa5b4fc,
        Hue::Violet => 0xc4b5fd,
        Hue::Purple => 0xd8b4fe,
        Hue::Fuchsia => 0xf0abfc,
        Hue::Pink => 0xf9a8d4,
        Hue::Rose => 0xfda4af,
        Hue::Slate => 0xcbd5e1,
        Hue::Gray => 0xd1d5db,
        Hue::Zinc => 0xd4d4d8,
        Hue::Neutral => 0xd4d4d4,
        Hue::Stone => 0xd6d3d1,
    };
    Color::from_rgb8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// What a pin looks like.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PinLook {
    pub color: Color,
    pub brackets: PinBrackets,
    pub fill: PinFill,
}

/// How many items a pin carries: a whole shape is one, however many
/// elements it is made of. See [`Card::items`].
pub fn items(card: Card, ty: &PinType) -> Items {
    match ty {
        PinType::Exact(ty) => card.items(ty),
        // Any type of a trait is seen as a shape.
        PinType::Trait(_) => card.items_of(true),
        PinType::Unknown => card.items_of(false),
    }
}

/// Nothing for one, `[ ]` for a list, `{ }` for a list of lists.
pub fn brackets(items: Items) -> PinBrackets {
    match items {
        Items::One => PinBrackets::None,
        Items::Many => PinBrackets::Square,
        Items::Nested => PinBrackets::Curly,
    }
}

/// Solid for exactly one type, a ring for any type of a trait, or for a
/// pin nothing is known about.
pub fn fill(ty: &PinType) -> PinFill {
    match ty {
        PinType::Exact(_) => PinFill::Solid,
        PinType::Trait(_) | PinType::Unknown => PinFill::Ring,
    }
}

pub fn pin_look(document: &Document, port: &PortView) -> PinLook {
    PinLook {
        color: color(document.hue(&port.ty)),
        brackets: brackets(items(port.card, &port.ty)),
        fill: fill(&port.ty),
    }
}

/// How many a pin carries, in words.
pub fn count(items: Items) -> &'static str {
    match items {
        Items::One => "one",
        Items::Many => "a list",
        Items::Nested => "a list of lists",
    }
}

/// What the tooltip of a pin says, as markdown: what it is called, what it
/// carries, how the node uses it, and what what it carries is.
pub fn port_tip(document: &Document, port: &PortView) -> String {
    let (ty, ty_doc) = document.type_text(&port.ty);
    let mut tip = format!("**{}** · {}, {}", port.label, ty, count(items(port.card, &port.ty)));
    if !port.doc.trim().is_empty() {
        tip += "\n\n";
        tip += port.doc.trim();
    }
    if !port.choices.is_empty() {
        let choices: Vec<_> = port.choices.iter().map(|choice| format!("`{choice}`")).collect();
        tip += &format!("\n\nOne of {}.", choices.join(", "));
    }
    if !ty_doc.trim().is_empty() {
        tip += &format!("\n\n*{ty}:* {}", oneiroi_nodes::meta::summary(&ty_doc));
    }
    tip
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_hue_has_its_own_colour() {
        let colors: Vec<_> = Hue::ALL.iter().map(|hue| color(*hue)).collect();
        for (i, a) in colors.iter().enumerate() {
            assert!(colors[i + 1..].iter().all(|b| a != b), "{:?}", Hue::ALL[i]);
        }
        assert_eq!(color(Hue::Sky), Color::from_rgb8(0x7d, 0xd3, 0xfc));
    }

    #[test]
    fn the_brackets_say_how_many_and_the_fill_how_exact() {
        let document = Document::example();
        let sweep = document
            .graph()
            .nodes()
            .find(|(_, node)| matches!(&node.kind, oneiroi_graph::NodeKind::Node(id) if id == "curve.sweep"))
            .unwrap()
            .0;
        let view = document.view(sweep).unwrap();
        let spine = pin_look(&document, &view.inputs[0]);
        let profile = pin_look(&document, &view.inputs[1]);
        assert_eq!((spine.brackets, spine.fill), (PinBrackets::Curly, PinFill::Solid));
        assert_eq!((profile.brackets, profile.fill), (PinBrackets::Square, PinFill::Ring));
        assert_eq!(spine.color, color(Hue::Amber));

        let tip = port_tip(&document, &view.inputs[1]);
        assert!(tip.starts_with("**Profile** · any Profile, a list\n\nThe cross-section"), "{tip}");
        assert!(tip.contains("*any Profile:* A flat outline"), "{tip}");
    }
}
