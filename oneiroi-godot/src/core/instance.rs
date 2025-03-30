use std::time::Instant;

use godot::{
    classes::{ClassDb, RenderingServer, notify::Node3DNotification},
    global::PropertyUsageFlags,
    meta::{ClassName, PropertyHintInfo, PropertyInfo},
    prelude::*,
};
use oneiroi::{
    asset::instance::AssetInstance,
    data_types::DataTypeInstance,
    operations::{Operation, PropertyInterface},
};

use crate::core::data_conversion::GodotDataTypeToOneiroiDataType;

use super::{
    asset::{OneiroiAsset /* OneiroiAssetInstance */},
    data_conversion::{DataTypeConversion, OneiroiToGodot},
    mesh_conversion_todo_remove_unify_with_data_conversion::OneiroiMeshGD,
};

#[derive(GodotClass)]
#[class(tool, init, base=Node3D)]
pub struct OneiroiInstance {
    base: Base<Node3D>,

    asset_instance: Option<AssetInstance>,

    #[var(get,set=set_asset)]
    #[export]
    asset: Option<Gd<OneiroiAsset>>,

    instance: Option<OneiroiMeshGD>,
}

#[godot_api]
impl INode3D for OneiroiInstance {
    fn ready(&mut self) {
        self.base_mut().set_ignore_transform_notification(false);
        self.base_mut().set_notify_transform(true);
        self.base_mut().set_notify_local_transform(true);

        //godot_print!("{:?}", self.get_property_list());

        #[cfg(not(feature = "only_runtime"))]
        if godot::classes::Engine::singleton().is_editor_hint() {
            use godot::classes::Timer;
            let mut timer = Timer::new_alloc();

            timer.set_wait_time(0.5);
            timer.set_autostart(true);

            let callable = self.base().callable("on_timer_timeout");
            timer.connect("timeout", &callable);
            self.base_mut().add_child(&timer);
        }

        //Make the initial compute if an Asset is present
        if self.asset.is_some() {
            self.compute();
        }
    }

    fn get_property_list(&mut self) -> Vec<PropertyInfo> {
        let mut properties = Vec::<PropertyInfo>::new();
        if self.asset.is_none() {
            return properties;
        }
        if self.asset_instance.is_none() {
            return properties;
        }

        for prop in self.asset_instance.as_ref().unwrap().get_properties() {
            properties.push(PropertyInfo {
                variant_type: prop.get_type().variant_type(),
                class_name: ClassName::none(), //prop.get_type().get_class_name(), //instead we get the class name
                property_name: prop.name().into(),
                hint_info: PropertyHintInfo::none(),
                usage: PropertyUsageFlags::DEFAULT,
            });
        }
        properties
    }

    fn property_get_revert(&self, property: StringName) -> Option<Variant> {
        self.asset.as_ref()?;
        for prop in self.asset_instance.as_ref().unwrap().get_properties() {
            if prop.name() == property.to_string() {
                //godot_print!("{} {}", property, prop.at("prop_default"));
                return Some(prop.get_default().to_godot());
            }
        }
        None
    }

    fn get_property(&self, property: StringName) -> Option<Variant> {
        /* if ClassDb::singleton()
            .class_get_property_list("Node3D")
            .iter_shared()
            .any(|d| d.at("name").to::<GString>() == property.clone().into())
        {
            return None;
        } */

        self.asset_instance.as_ref()?;
        if let Ok(prop) = self
            .asset_instance
            .as_ref()
            .unwrap()
            /* .bind()
            .get_inner() */
            .try_get_property(&property.to_string())
        {
            return Some(prop.to_godot());
        }
        None
    }

    fn set_property(&mut self, property: StringName, value: Variant) -> bool {
        if self.asset_instance.is_none() {
            return false;
        }
        if self
            .asset_instance
            .as_mut()
            .unwrap()
            .try_set_property(&property.to_string(), value.to_oneiroi())
            .is_err()
        {
            return false;
        };
        if self.base().is_node_ready() {
            self.compute();
        }

        true
    }

    fn get_configuration_warnings(&self) -> PackedStringArray {
        let mut array = PackedStringArray::new();
        if self.asset.is_none() {
            array.push(
                "Please select the OneiroiAsset resource this OneiroiInstance is going to use.",
            );
        }
        array
    }

    fn on_notification(&mut self, what: Node3DNotification) {
        if what == Node3DNotification::TRANSFORM_CHANGED
            || what == Node3DNotification::LOCAL_TRANSFORM_CHANGED
        {
            if let Some(mesh) = &self.instance {
                RenderingServer::singleton().instance_set_transform(
                    mesh.get_instance(),
                    self.base().get_global_transform(),
                );
            }
        }
    }

    fn exit_tree(&mut self) {
        if let Some(mesh) = &self.instance {
            RenderingServer::singleton().free_rid(mesh.get_instance());
        }
    }
}

#[godot_api]
impl OneiroiInstance {
    #[func]
    pub fn set_asset(&mut self, value: Option<Gd<OneiroiAsset>>) {
        //update the value
        self.asset = value;
        self.base_mut().update_configuration_warnings();
        if let Some(asset) = &self.asset {
            if self.asset_instance.is_none() {
                self.asset_instance = Some(asset.bind().get_inner().get_instance());
            }

            //only compute if the node was already ready before the setter because this means that it was changed at edit time
            if self.base().is_node_ready() {
                self.compute();
            }
        } else {
            //TODO also clear the saved props in that case
            //self.asset_instance.bind_mut().get_props().clear();
            godot_print!("TODO no asset so clean up everything thats left over")
        }
        //whenever the asset gets changed change the property list
        self.base_mut().notify_property_list_changed();
    }

    fn compute(&mut self) {
        let computation_instant = Instant::now();
        //TODO inject the child node data types once that system is complete
        let instances = self.asset_instance.as_ref().unwrap().compute(vec![]);
        //godot_print!("{:?}", instances);
        //TODO not hardcode in here handle all outputs
        for instance in instances {
            match instance {
                DataTypeInstance::Mesh(mesh) => {
                    self.instance = Some(OneiroiMeshGD::new(mesh.consume()))
                }
                _ => panic!(),
            }
        }

        RenderingServer::singleton().instance_set_scenario(
            self.instance.as_ref().unwrap().get_instance(),
            self.base().get_world_3d().unwrap().get_scenario(),
        );

        RenderingServer::singleton().instance_set_transform(
            self.instance.as_ref().unwrap().get_instance(),
            self.base().get_global_transform(),
        );

        println!(
            "Godot instance computation took {} nanoseconds",
            computation_instant.elapsed().as_nanos()
        );
    }

    #[cfg(not(feature = "only_runtime"))]
    #[func]
    fn on_timer_timeout(&mut self) {
        if let Some(instance) = self.asset_instance.as_mut() {
            if instance.is_dirty() {
                self.compute();
            }
        }
    }
}
