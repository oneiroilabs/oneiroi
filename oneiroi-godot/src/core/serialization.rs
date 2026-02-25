use godot::{
    classes::{
        Engine, FileAccess, IResourceFormatLoader, IResourceFormatSaver, ResourceFormatLoader,
        ResourceFormatSaver, ResourceLoader, ResourceSaver, file_access::ModeFlags,
    },
    prelude::*,
};
use oneiroi::serialization::{deserialize_asset_v1, serialize_asset_v1};

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
}

#[godot_api]
impl IObject for OneiroiInputOutputSingleton {
    fn init(base: Base<Object>) -> Self {
        let plugin = Self {
            base,
            loader: OneiroiAssetLoader::new_gd(),
            saver: OneiroiAssetSaver::new_gd(),
        };

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
        if res.unwrap().is_class("OneiroiAsset") {
            arr.push("oni");
        }

        arr
    }

    fn recognize(&self, res: Option<Gd<Resource>>) -> bool {
        res.unwrap().is_class("OneiroiAsset")
    }

    fn save(
        &mut self,
        resource: Option<Gd<Resource>>,
        path: GString,
        _flags: u32, //TODO dont know what this is for and no time for now
    ) -> godot::global::Error {
        let res = resource
            .unwrap()
            .try_cast::<OneiroiAsset>()
            .expect("Godot called this function with the wrong resource type");

        let string = serialize_asset_v1(&res.bind());

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
        arr.push("oni");
        arr
    }

    fn handles_type(&self, typ: StringName) -> bool {
        typ == "OneiroiAsset".into()
    }

    fn get_resource_type(&self, path: GString) -> GString {
        if path.get_extension().to_lower() == ".oni".into() {
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
            FileAccess::open(&path, ModeFlags::READ).expect("Cant obtain read access to path");

        let file_content: String = file.get_as_text().to_string();
        let raw_asset = deserialize_asset_v1(&file_content);
        OneiroiAsset::init_from_inner(raw_asset).to_variant()
    }
}
