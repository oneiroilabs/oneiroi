use core::serialization::{register_input_output, unregister_input_output};

#[cfg(not(feature = "only_runtime"))]
use editor::editor_server::{register_preview_server, unregister_preview_server};
//use editor::Oneiroi_class_db_server::{register_classdb_server, unregister_classdb_server};
use godot::prelude::*;

pub mod core;
#[cfg(not(feature = "only_runtime"))]
mod editor;

//The Main Struct of the Library
struct Oneiroi;

/// Entrypoint into the library
#[gdextension]
unsafe impl ExtensionLibrary for Oneiroi {
    fn on_level_init(level: InitLevel) {
        if level == InitLevel::Scene {
            register_input_output();

            #[cfg(not(feature = "only_runtime"))]
            register_preview_server();
        }
        if level == InitLevel::Editor {
            //EditorNode::sing::get_log()->add_message("--- GDScript language server started on port " + itos(port) + " ---", EditorLog::MSG_TYPE_EDITOR);
            //EditorNode
            /* #[cfg(not(feature = "only_runtime"))]
            register_classdb_server(); */
        }
    }

    fn on_level_deinit(level: InitLevel) {
        if level == InitLevel::Scene {
            unregister_input_output();

            #[cfg(not(feature = "only_runtime"))]
            unregister_preview_server(level);
        }

        /* #[cfg(not(feature = "only_runtime"))]
        unregister_classdb_server(level); */
    }
}
