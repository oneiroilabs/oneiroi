//! Versions of generic nodes which are made when a type registers itself.

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use oneiroi_core::node::{GenericNode, NodeDef};

/// The version of `generic` for the type called `ty`, made by `make` from
/// its id the first time it is asked for.
///
/// A version lives as long as the program does, like the nodes which are
/// written out as statics, and there is only ever one per id.
pub(crate) fn version(
    generic: &GenericNode,
    ty: &str,
    make: impl FnOnce(&'static str) -> NodeDef,
) -> &'static NodeDef {
    static VERSIONS: LazyLock<Mutex<HashMap<String, &'static NodeDef>>> =
        LazyLock::new(Mutex::default);

    let id = format!("{}<{ty}>", generic.id);
    let mut versions = VERSIONS.lock().unwrap();
    *versions
        .entry(id.clone())
        .or_insert_with(|| Box::leak(Box::new(make(id.leak()))))
}
