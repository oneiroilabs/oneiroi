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
    fn on_stage_init(level: InitStage) {
        if level == InitStage::Scene {
            register_input_output();

            #[cfg(feature = "editor")]
            register_preview_server();
        }
        if level == InitStage::Editor {
            //godot_print!("--- Oneiroi initialized successfully! ---")
        }
    }

    fn on_stage_deinit(level: InitStage) {
        if level == InitStage::Scene {
            unregister_input_output();

            #[cfg(feature = "editor")]
            unregister_preview_server(level);
        }
    }
}
