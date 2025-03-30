use godot::{
    classes::{
        Engine, FileAccess, IResourceFormatLoader, IResourceFormatSaver, ResourceFormatLoader,
        ResourceFormatSaver, ResourceLoader, ResourceSaver, file_access::ModeFlags,
    },
    prelude::*,
};
use oneiroi::serialization::{export_asset_v1, import_asset_v1};

use super::asset::{OneiroiAsset /* OneiroiAssetInstance */};

pub fn register_input_output() {
    Engine::singleton().register_singleton(
        "OneiroiInputOutputSingleton",
        &OneiroiInputOutputSingleton::new_alloc(),
    );
}

pub fn unregister_input_output() {
    Engine::singleton().unregister_singleton("OneiroiInputOutputSingleton");
}

#[derive(GodotClass)]
#[class(tool, base=Object)]
struct OneiroiInputOutputSingleton {
    base: Base<Object>,
    loader: Gd<OneiroiAssetLoader>,
    saver: Gd<OneiroiAssetSaver>,
    //loader_instance: Gd<OneiroiAssetInstanceLoader>,
    //saver_instance: Gd<OneiroiAssetInstanceSaver>,
}

#[godot_api]
impl IObject for OneiroiInputOutputSingleton {
    fn init(base: Base<Object>) -> Self {
        let plugin = Self {
            base,
            loader: OneiroiAssetLoader::new_gd(),
            saver: OneiroiAssetSaver::new_gd(),
            //loader_instance: OneiroiAssetInstanceLoader::new_gd(),
            //saver_instance: OneiroiAssetInstanceSaver::new_gd(),
        };
        /* ResourceSaver::singleton()
            .add_resource_format_saver_ex(&plugin.saver_instance)
            .at_front(true)
            .done();
        ResourceLoader::singleton().add_resource_format_loader(&plugin.loader_instance); */

        ResourceSaver::singleton()
            .add_resource_format_saver_ex(&plugin.saver)
            .at_front(true)
            .done();
        ResourceLoader::singleton().add_resource_format_loader(&plugin.loader);
        plugin
    }
}

#[derive(GodotClass)]
#[class(init,tool, base=ResourceFormatSaver)]
pub struct OneiroiAssetSaver {
    base: Base<ResourceFormatSaver>,
}

#[godot_api]
impl IResourceFormatSaver for OneiroiAssetSaver {
    fn get_recognized_extensions(&self, res: Option<Gd<Resource>>) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        if res.expect("Has to be there").is_class("OneiroiAsset") {
            arr.push("asset");
        }

        arr
    }

    fn recognize(&self, res: Option<Gd<Resource>>) -> bool {
        res.expect("This parameter should contain Some")
            .is_class("OneiroiAsset")
    }

    fn save(
        &mut self,
        resource: Option<Gd<Resource>>,
        path: GString,
        _flags: u32, //TODO dont know what this is for and no time for now
    ) -> godot::global::Error {
        //TODO error handling
        let res = resource
            .expect("The Resource Godot attepts to save is not there?")
            .try_cast::<OneiroiAsset>()
            .expect("Godot called this function with the wrong resource type");

        let string = export_asset_v1(res.bind().get_inner());

        let mut file =
            FileAccess::open(&path, ModeFlags::WRITE).expect("Cant obtain write access to path");

        file.store_string(&string);

        godot::global::Error::OK
    }
}

#[derive(GodotClass)]
#[class(init,tool, base=ResourceFormatLoader)]
pub struct OneiroiAssetLoader {
    base: Base<ResourceFormatLoader>,
}

#[godot_api]
impl IResourceFormatLoader for OneiroiAssetLoader {
    fn get_recognized_extensions(&self) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        arr.push("asset");
        arr
    }

    fn handles_type(&self, typ: StringName) -> bool {
        typ == "OneiroiAsset".into()
    }

    fn get_resource_type(&self, path: GString) -> GString {
        if path.get_extension().to_lower() == ".asset".into() {
            "OneiroiAsset".into()
        } else {
            "".into()
        }
    }

    fn load(
        &self,
        path: GString,
        original_path: GString, //TODO these things need to be figured out eventally
        use_sub_threads: bool,
        cache_mode: i32,
    ) -> Variant {
        let file =
            FileAccess::open(&path, ModeFlags::READ).expect("Cant obtain write access to path");

        let file_content: String = file.get_as_text().to_string();
        let raw_asset = import_asset_v1(file_content);
        OneiroiAsset::init_from_inner(raw_asset).to_variant()
    }
}

/* #[derive(GodotClass)]
#[class(init,tool, base=ResourceFormatSaver)]
pub struct OneiroiAssetInstanceSaver {
    base: Base<ResourceFormatSaver>,
}

#[godot_api]
impl IResourceFormatSaver for OneiroiAssetInstanceSaver {
    fn get_recognized_extensions(&self, res: Option<Gd<Resource>>) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        if res
            .expect("Has to be there")
            .is_class("OneiroiAssetInstance")
        {
            arr.push("tres");
        }

        arr
    }

    fn recognize(&self, res: Option<Gd<Resource>>) -> bool {
        if res
            .expect("This parameter should contain Some")
            .is_class("OneiroiAssetInstance")
        {
            godot_print!("hmm");
            return true;
        }
        false
    }

    fn save(
        &mut self,
        resource: Option<Gd<Resource>>,
        path: GString,
        _flags: u32, //TODO dont know what this is for and no time for now
    ) -> godot::global::Error {
        godot_error!("I HAVE BBEN CALLED");
        //TODO error handling
        let res = resource
            .expect("The Resource Godot attepts to save is not there?")
            .try_cast::<OneiroiAssetInstance>()
            .expect("Godot called this function with the wrong resource type");

        let string = export_asset_instance_v1(res.bind().get_inner());

        let mut file =
            FileAccess::open(&path, ModeFlags::WRITE).expect("Cant obtain write access to path");

        file.store_string(&string);

        godot::global::Error::OK
    }
} */

/* #[derive(GodotClass)]
#[class(init,tool, base=ResourceFormatLoader)]
pub struct OneiroiAssetInstanceLoader {
    base: Base<ResourceFormatLoader>,
}

#[godot_api]
impl IResourceFormatLoader for OneiroiAssetInstanceLoader {
    fn get_recognized_extensions(&self) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        arr.push("tscn");
        //arr.push("res"); TODO
        arr
    }

    fn handles_type(&self, typ: StringName) -> bool {
        if typ == "OneiroiAssetInstance".into() {
            godot_print!("hmm");
            return true;
        }
        false
    }

    fn get_resource_type(&self, path: GString) -> GString {
        if path.get_extension().to_lower() == ".tscn".into() {
            "OneiroiAssetInstance".into()
        } else {
            "".into()
        }
    }

    fn load(
        &self,
        path: GString,
        original_path: GString, //TODO these things need to be figured out eventally
        use_sub_threads: bool,
        cache_mode: i32,
    ) -> Variant {
        godot_error!("I HAVE BBEN CALLED");
        let file =
            FileAccess::open(&path, ModeFlags::READ).expect("Cant obtain write access to path");

        let file_content: String = file.get_as_text().to_string();
        let raw_asset = import_asset_instance_v1(file_content);
        OneiroiAssetInstance::init_from_inner(raw_asset).to_variant()
    }
} */
