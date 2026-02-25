use core::serialization::{register_input_output, unregister_input_output};

#[cfg(feature = "editor")]
use editor::editor_server::{register_preview_server, unregister_preview_server};
use godot::prelude::*;

pub mod core;
#[cfg(feature = "editor")]
mod editor;

struct Oneiroi;

#[gdextension]
unsafe impl ExtensionLibrary for Oneiroi {
    fn on_level_init(level: InitLevel) {
        if level == InitLevel::Scene {
            register_input_output();

            #[cfg(feature = "editor")]
            register_preview_server();
        }
        if level == InitLevel::Editor {
            //godot_print!("--- Oneiroi initialized successfully! ---")
        }
    }

    fn on_level_deinit(level: InitLevel) {
        if level == InitLevel::Scene {
            unregister_input_output();

            #[cfg(feature = "editor")]
            unregister_preview_server(level);
        }
    }
}
