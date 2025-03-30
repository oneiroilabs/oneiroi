use godot::{
    classes::{Engine, World3D},
    prelude::*,
};

use crate::core::asset::OneiroiAsset;

#[derive(GodotClass)]
#[class(init,internal, base=Object)]
pub struct OneiroiEditorServer {
    base: Base<Object>,
    preview_world: Gd<World3D>,
    registered_assets: Vec<Gd<OneiroiAsset>>,
    #[var]
    active_asset: Option<Gd<OneiroiAsset>>,
}

#[godot_api]
impl OneiroiEditorServer {
    #[func]
    pub fn get_preview_world(&mut self) -> Gd<World3D> {
        //let env = self.preview_world.get_fallback_environment().unwrap();
        //self.preview_world.set_environment(&env);
        self.preview_world.clone()
    }

    #[func]
    pub fn register_object_to_edit(&mut self, obj: Gd<OneiroiAsset>) {
        //if tab with given resource is already open just push it to active
        if self.registered_assets.contains(&obj) {
            //TODO make the selected reesource the active edited tab
            return;
        }
        self.registered_assets.push(obj.clone());
        self.active_asset = Some(obj.clone());
        self.base_mut()
            .emit_signal("new_active_asset", &[obj.to_variant()]);
    }

    /* pub fn get_active_asset(&self) -> Gd<OneiroiAsset> {
        self.active_asset.as_ref().expect("").clone()
    } */

    #[signal]
    fn new_active_asset(active_asset: Gd<OneiroiAsset>);

    #[signal]
    fn node_selected(active_node: Gd<Resource>);
}

pub fn register_preview_server() {
    // The StringName identifies your singleton and can be
    // used later to access it.
    Engine::singleton().register_singleton(
        //TODO rename this most likely
        "OneiroiEditorServer",
        &OneiroiEditorServer::new_alloc(),
    );
}

pub fn unregister_preview_server(level: InitLevel) {
    if level == InitLevel::Scene {
        // Get the `Engine` instance and `StringName` for your singleton.
        let mut engine = Engine::singleton();
        //TODO rename this most likely
        let singleton_name = StringName::from("OneiroiEditorServer");

        // We need to retrieve the pointer to the singleton object,
        // as it has to be freed manually - unregistering singleton
        // doesn't do it automatically.
        let singleton = engine
            .get_singleton(&singleton_name)
            .expect("cannot retrieve the singleton");

        // Unregistering singleton and freeing the object itself is needed
        // to avoid memory leaks and warnings, especially for hot reloading.
        engine.unregister_singleton(&singleton_name);
        singleton.free();
    }
}
