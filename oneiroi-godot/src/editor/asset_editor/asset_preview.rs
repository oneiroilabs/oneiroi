use godot::{
    classes::{DirectionalLight3D, ISubViewport, SubViewport},
    prelude::*,
};

use crate::{
    core::{asset::OneiroiAsset, instance::OneiroiInstance},
    editor::editor_server::OneiroiEditorServer,
};

#[derive(GodotClass)]
#[class(tool,internal,init, base=SubViewport)]
pub struct OneiroiAssetPreview {
    base: Base<SubViewport>,

    #[init(val=Camera3D::new_alloc())]
    vp_cam: Gd<Camera3D>,

    #[init(val=Node3D::new_alloc())]
    root_node: Gd<Node3D>,

    #[init(val=OneiroiInstance::new_alloc())]
    oneiroi_instance: Gd<OneiroiInstance>,
}

#[godot_api]
impl ISubViewport for OneiroiAssetPreview {
    fn ready(&mut self) {
        self.base_mut().set_use_own_world_3d(true);
        self.base_mut().set_world_3d(
            &godot::classes::Engine::singleton()
                .get_singleton(&StringName::from("OneiroiEditorServer"))
                .expect("Cant find Preview Server")
                .cast::<OneiroiEditorServer>()
                .bind_mut()
                .get_preview_world(),
        );

        self.root_node.add_child(&DirectionalLight3D::new_alloc());

        self.vp_cam.translate(Vector3 {
            x: 0.0,
            y: 3.0,
            z: 5.0,
        });
        self.vp_cam.rotate_x(-0.3);
        self.root_node.add_child(&self.vp_cam);
        self.root_node.add_child(&self.oneiroi_instance);

        let on_active_asset_changed = self.base().callable("set_active_asset");
        godot::classes::Engine::singleton()
            .get_singleton(&StringName::from("OneiroiEditorServer"))
            .expect("Cant find Preview Server")
            .cast::<OneiroiEditorServer>()
            .bind_mut()
            .base_mut()
            .connect("new_active_asset", &on_active_asset_changed);

        let root_node = self.root_node.clone();
        self.base_mut().add_child(&root_node);
    }
}

#[godot_api]
impl OneiroiAssetPreview {
    #[func]
    fn set_active_asset(&mut self, active_asset: Option<Gd<OneiroiAsset>>) {
        self.oneiroi_instance.bind_mut().set_asset(active_asset);
    }
}
