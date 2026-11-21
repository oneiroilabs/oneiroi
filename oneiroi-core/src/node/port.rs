//! The ports of a node: what flows through them, and how many of it.

use crate::types::{DataType, PortDefault, TypeInfo};

#[derive(Debug, Clone, Copy)]
pub struct PortDecl {
    pub name: &'static str,
    pub ty: TypeInfo,
    /// Only meaningful for inputs.
    pub default: PortDefault,
    pub card: Card,
    /// Only meaningful for grouped outputs.
    pub groups: Groups,
    /// Only meaningful for outputs of instances: the input whose shape
    /// they refer to. Either the input is the shape, or instances of it
    /// and the shape is theirs.
    pub prototype: Option<usize>,
    /// Only meaningful for inputs: the names of what a single `u32` can
    /// say, by its value. See [`PortDecl::choices`].
    pub choices: &'static [&'static str],
    /// Only meaningful for outputs: the inputs which say how many things
    /// it has, a bit for each. See [`PortDecl::follows`].
    pub follows: u32,
}

impl PortDecl {
    /// A port of many elements.
    pub const fn new<T: DataType>(name: &'static str) -> Self {
        Self {
            name,
            ty: T::INFO,
            default: PortDefault::EMPTY,
            card: Card::Many,
            groups: Groups::Own,
            prototype: None,
            choices: &[],
            follows: 0,
        }
    }

    /// A port of a type only known by its [`TypeInfo`], like instances.
    pub const fn of(name: &'static str, ty: TypeInfo) -> Self {
        Self {
            name,
            ty,
            default: PortDefault::EMPTY,
            card: Card::Many,
            groups: Groups::Own,
            prototype: None,
            choices: &[],
            follows: 0,
        }
    }

    /// An output of instances of the shape of input `input`, see
    /// [`PortDecl::prototype`].
    pub const fn instances_of(mut self, input: usize) -> Self {
        self.prototype = Some(input);
        self
    }

    /// What the input has while nothing is linked to it. A default of
    /// another type than the port's does not compile:
    ///
    /// ```compile_fail
    /// # use oneiroi_core::node::PortDecl;
    /// const OFFSET: PortDecl = PortDecl::new::<glam::Vec3>("offset").with_default(&[1.0f32]);
    /// ```
    pub const fn with_default<T: DataType>(mut self, values: &'static [T]) -> Self {
        self.default = PortDefault::of(values);
        assert!(self.default.fits(&self.ty), "a default must be of the type of its port");
        self
    }

    /// A port of a single element.
    pub const fn one(mut self) -> Self {
        self.card = Card::One;
        self
    }

    /// An input of a single `u32` which picks one of `choices`, by its
    /// index: an editor offers their names instead of a number. It starts
    /// at the first one. The kernel reads it with [`Inputs::choice`].
    pub const fn choices(mut self, choices: &'static [&'static str]) -> Self {
        self.card = Card::One;
        self.choices = choices;
        if self.default.is_empty() {
            self.default = PortDefault::of::<u32>(&[0]);
        }
        self
    }

    /// An input which gets the groups of what is linked to it, or an
    /// output whose groups the node makes: see [`Outputs::set_groups`].
    pub const fn grouped(mut self) -> Self {
        self.card = Card::Grouped;
        self.groups = Groups::Own;
        self
    }

    /// An output which is grouped the same way as the input `input`: one
    /// element for each of its elements, or as many in every group.
    pub const fn grouped_like(mut self, input: usize) -> Self {
        self.card = Card::Grouped;
        self.groups = Groups::Like(input);
        self.follows = 1 << input;
        self
    }

    /// An output with a thing for every thing of the inputs `inputs`: a
    /// single one if each of them is a single one, else a list. A kernel
    /// which makes one for each of a list serves a single one that way too.
    pub const fn follows(mut self, inputs: &[usize]) -> Self {
        let mut index = 0;
        while index < inputs.len() {
            self.follows |= 1 << inputs[index];
            index += 1;
        }
        self
    }

    /// The inputs this output follows, see [`PortDecl::follows`].
    pub fn followed(&self) -> impl Iterator<Item = usize> + '_ {
        (0..32).filter(|input| self.follows & (1 << input) != 0)
    }
}

/// How a port hands its elements to the kernel. What that means to whoever
/// links it depends on the type, see [`Card::items`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Card {
    /// A single element.
    One,
    /// A list of elements, without offsets. Groups of what is linked to it
    /// are dropped.
    Many,
    /// Elements with the offsets where every group starts.
    Grouped,
}

/// How many things a port or a value carries, as whoever links it sees it:
/// elements of most types, but whole groups of a type whose elements are
/// only parts, like the segments of a curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Items {
    One,
    Many,
    /// A list of lists: elements in groups, like the frames along each of
    /// several curves.
    Nested,
}

impl Card {
    /// What a port of this card carries of `ty`. All elements of a type
    /// made of parts are one shape, and only offsets make them several.
    pub fn items(self, ty: &TypeInfo) -> Items {
        self.items_of(ty.parts)
    }

    /// [`Card::items`] by whether the type is made of parts. A port which
    /// takes any type of a trait is seen as one of shapes.
    pub fn items_of(self, parts: bool) -> Items {
        match (self, parts) {
            (Card::One, _) | (Card::Many, true) => Items::One,
            (Card::Many, false) | (Card::Grouped, true) => Items::Many,
            (Card::Grouped, false) => Items::Nested,
        }
    }
}

impl Items {
    /// Whether a value of these items can go where `to` are wanted. One
    /// thing is a list of one, and a list of lists goes into a list as
    /// all of its elements, but only one thing goes where one is wanted.
    pub fn fits(self, to: Items) -> bool {
        to != Items::One || self == Items::One
    }
}

/// Where the groups of a grouped output come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Groups {
    /// The node makes them, as an output of its own.
    Own,
    /// They are the groups of an input, and nothing has to be made.
    Like(usize),
}

/// Where the offsets of grouped input `index` are handed over: behind all
/// declared inputs, in the order of the grouped ones.
pub(crate) fn hidden_input(ports: &[PortDecl], index: usize) -> Option<usize> {
    (ports[index].card == Card::Grouped).then(|| {
        ports.len() + ports[..index].iter().filter(|p| p.card == Card::Grouped).count()
    })
}

/// Where the shape of input `index` of instances is handed over, its
/// offsets right after and then the total of the instances: behind the
/// offsets of all grouped inputs, in the order of the inputs of instances.
pub(crate) fn hidden_prototype(ports: &[PortDecl], index: usize) -> Option<usize> {
    ports[index].ty.prototype.map(|_| {
        let grouped = ports.iter().filter(|p| p.card == Card::Grouped).count();
        let before = ports[..index].iter().filter(|p| p.ty.prototype.is_some()).count();
        ports.len() + grouped + 3 * before
    })
}

/// Where the total of output `index` of instances goes: behind the
/// offsets of the outputs with groups of their own, in the order of the
/// outputs of instances.
pub(crate) fn hidden_total(ports: &[PortDecl], index: usize) -> Option<usize> {
    let own = |port: &PortDecl| port.card == Card::Grouped && port.groups == Groups::Own;
    ports[index].ty.prototype.map(|_| {
        let before = ports[..index].iter().filter(|p| p.ty.prototype.is_some()).count();
        ports.len() + ports.iter().filter(|p| own(p)).count() + before
    })
}

/// Where the offsets of output `index` go, if the node makes them: behind
/// all declared outputs, in the order of the outputs which have their own.
pub(crate) fn hidden_output(ports: &[PortDecl], index: usize) -> Option<usize> {
    let own = |port: &PortDecl| port.card == Card::Grouped && port.groups == Groups::Own;
    own(&ports[index]).then(|| ports.len() + ports[..index].iter().filter(|p| own(p)).count())
}
