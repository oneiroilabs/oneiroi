//! Macros which write a node, a generic node or a trait together with
//! what people see of it: its name, where it is found, and documentation
//! for it and every port, taken from its doc comments.
//!
//! What runs gets none of it. The descriptions are separate statics,
//! called like the item with `_META` behind, and only there with the
//! feature `meta`.

/// Writes its input only with the feature `meta`.
#[cfg(feature = "meta")]
#[doc(hidden)]
#[macro_export]
macro_rules! __if_meta {
    ($($tokens:tt)*) => { $($tokens)* };
}

#[cfg(not(feature = "meta"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __if_meta {
    ($($tokens:tt)*) => {};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __or_empty {
    () => {
        ""
    };
    ($label:literal) => {
        $label
    };
}

/// A [`NodeDef`](oneiroi_core::node::NodeDef) with its documentation.
///
/// ```ignore
/// node! {
///     /// What the node does, the first paragraph is its summary.
///     pub static TRANSLATE = "curve.translate" {
///         label: "Translate Curve",
///         category: "Curve",
///         version: 1,
///         inputs {
///             /// How the node uses the port.
///             port PortDecl::new::<CubicNurbsSegmentCache>("segments").grouped();
///             /// A port can have a label other than its name.
///             port PortDecl::new::<Vec3>("offset").one(), "Offset";
///         }
///         outputs {
///             /// ...
///             port PortDecl::new::<CubicNurbsSegmentCache>("segments").grouped_like(0);
///         }
///         cpu: translate,
///         gpu: &[],
///     }
/// }
/// ```
#[macro_export]
macro_rules! node {
    (
        $(#[doc = $doc:literal])*
        $vis:vis static $name:ident = $id:literal {
            label: $label:literal,
            category: $category:literal,
            version: $version:expr,
            inputs { $( $(#[doc = $idoc:literal])* port $input:expr $(, $ilabel:literal)? ; )* }
            outputs { $( $(#[doc = $odoc:literal])* port $output:expr $(, $olabel:literal)? ; )* }
            cpu: $cpu:expr,
            gpu: $gpu:expr $(,)?
        }
    ) => {
        $(#[doc = $doc])*
        $vis static $name: $crate::__core::node::NodeDef = $crate::__core::node::NodeDef {
            id: $id,
            version: $version,
            inputs: &[$($input),*],
            outputs: &[$($output),*],
            cpu: $cpu,
            gpu: $gpu,
        };
        $crate::__if_meta! {
            $crate::__paste::paste! {
                #[doc = concat!("What people see of [`", stringify!($name), "`].")]
                $vis static [<$name _META>]: $crate::__meta::NodeMeta = $crate::__meta::NodeMeta {
                    id: $id,
                    label: $label,
                    category: $category,
                    doc: concat!($($doc, "\n"),*),
                    inputs: &[$($crate::__meta::PortMeta {
                        name: $input.name,
                        label: $crate::__or_empty!($($ilabel)?),
                        doc: concat!($($idoc, "\n"),*),
                    }),*],
                    outputs: &[$($crate::__meta::PortMeta {
                        name: $output.name,
                        label: $crate::__or_empty!($($olabel)?),
                        doc: concat!($($odoc, "\n"),*),
                    }),*],
                };
            }
        }
    };
}

/// A [`GenericNode`](oneiroi_core::node::GenericNode) with its
/// documentation, written like [`node!`] without version and kernels. Its
/// versions are described by it.
#[macro_export]
macro_rules! generic_node {
    (
        $(#[doc = $doc:literal])*
        $vis:vis static $name:ident = $id:literal {
            label: $label:literal,
            category: $category:literal,
            inputs { $( $(#[doc = $idoc:literal])* port $input:expr $(, $ilabel:literal)? ; )* }
            outputs { $( $(#[doc = $odoc:literal])* port $output:expr $(, $olabel:literal)? ; )* }
        }
    ) => {
        $(#[doc = $doc])*
        $vis static $name: $crate::__core::node::GenericNode = $crate::__core::node::GenericNode {
            id: $id,
            inputs: &[$($input),*],
            outputs: &[$($output),*],
        };
        $crate::__if_meta! {
            $crate::__paste::paste! {
                #[doc = concat!("What people see of [`", stringify!($name), "`].")]
                $vis static [<$name _META>]: $crate::__meta::NodeMeta = $crate::__meta::NodeMeta {
                    id: $id,
                    label: $label,
                    category: $category,
                    doc: concat!($($doc, "\n"),*),
                    inputs: &[$($crate::__meta::PortMeta {
                        name: $input.name,
                        label: $crate::__or_empty!($($ilabel)?),
                        doc: concat!($($idoc, "\n"),*),
                    }),*],
                    outputs: &[$($crate::__meta::PortMeta {
                        name: $output.name,
                        label: $crate::__or_empty!($($olabel)?),
                        doc: concat!($($odoc, "\n"),*),
                    }),*],
                };
            }
        }
    };
}

/// A [`Trait`](oneiroi_core::types::Trait) with its documentation and the
/// hue of the types which implement it.
#[macro_export]
macro_rules! describe_trait {
    (
        $(#[doc = $doc:literal])*
        $vis:vis const $name:ident = $trait_name:literal {
            label: $label:literal,
            hue: $hue:ident $(,)?
        }
    ) => {
        $(#[doc = $doc])*
        $vis const $name: $crate::__core::types::Trait = $crate::__core::types::Trait { name: $trait_name };
        $crate::__if_meta! {
            $crate::__paste::paste! {
                #[doc = concat!("What people see of [`", stringify!($name), "`].")]
                $vis static [<$name _META>]: $crate::__meta::TraitMeta = $crate::__meta::TraitMeta {
                    name: $trait_name,
                    label: $label,
                    hue: $crate::__meta::Hue::$hue,
                    doc: concat!($($doc, "\n"),*),
                };
            }
        }
    };
}

/// Documentation, a label and a hue for a data type, as
/// [`Described`](crate::Described). The doc comments here are for people
/// using the type in a graph, the ones on the type itself for its code.
#[macro_export]
macro_rules! describe_type {
    (
        $(#[doc = $doc:literal])*
        $ty:ty {
            label: $label:literal,
            hue: $hue:ident $(,)?
        }
    ) => {
        $crate::__if_meta! {
            impl $crate::Described for $ty {
                const META: $crate::__meta::TypeMeta = $crate::__meta::TypeMeta {
                    name: <$ty as $crate::__core::types::DataType>::NAME,
                    label: $label,
                    hue: $crate::__meta::Hue::$hue,
                    doc: concat!($($doc, "\n"),*),
                };
            }
        }
    };
}
