use std::{collections::HashMap, mem::MaybeUninit, time::Instant};

use godot::{
    classes::{
        ArrayMesh, Curve3D, Material, Path3D, RenderingServer, notify::Node3DNotification,
        rendering_server::MultimeshTransformFormat,
    },
    prelude::*,
    register::info::{PropertyHintInfo, PropertyInfo, PropertyUsageFlags},
};
use oneiroi::{
    asset::{NodeIndex, instance::AssetInstance},
    nodes::{ContextProvider, Node, PropertyInterface, SocketInterface},
    type_system::{
        OwnedDataType, Reference, TypeRef,
        data_types::{DataTypeKind, Instance, Mesh},
    },
};

use crate::core::data_conversion::TypeConvert;

use super::{asset::OneiroiAsset, data_conversion::OneiroiToGodot};

/// The Instance produces an output of an Asset specified in the asset field.
/// Upon instantiation in the SceneTree it spawns Nodes and Properties for the exposed Types.
/// These can be modified by users to customize each Instance individually.
/// Modifications are saved to Disk in the native Godot scene file.
#[derive(GodotClass, Debug)]
#[class(tool, init, base=Node3D)]
pub struct OneiroiInstance {
    base: Base<Node3D>,

    /// The internal Instance which conceptually serves as a template in this Context.
    /// This is because OneiroiInstance provides and caches all data type related things.
    asset_instance: Option<AssetInstance>,

    /// The associated asset this OneiroiInstance processes.
    #[var(set=set_asset)]
    #[export]
    asset: Option<Gd<OneiroiAsset>>,

    cached_outputs: Box<[MaybeUninit<Variant>]>,
    managed_instances: Box<[Rid]>,
    /// Stores all converted References received from the AssetIntance.
    reference_values: HashMap<Reference, Variant>,

    /// Manages the input sockets and tracks if they changed.
    input_sockets: HashMap<Reference, (Variant, bool, OwnedDataType)>,
}

#[godot_api]
impl INode3D for OneiroiInstance {
    fn ready(&mut self) {
        self.base_mut().set_ignore_transform_notification(false);
        self.base_mut().set_notify_transform(true);
        self.base_mut().set_notify_local_transform(true);

        //Make the initial compute if an Asset is present
        if self.asset.is_some() {
            self.initialize();
            self.compute();
        }
    }

    fn on_get_property_list(&mut self) -> Vec<PropertyInfo> {
        let mut properties = Vec::<PropertyInfo>::new();
        if self.asset.is_none() || self.asset_instance.is_none() {
            return properties;
        }

        for prop in self.asset_instance.as_ref().unwrap().get_properties() {
            properties.push(PropertyInfo {
                variant_type: prop.get_type().variant_type(),
                class_name: StringName::default(), //prop.get_type().get_class_name(), //instead we get the class name
                property_name: prop.name().into(),
                hint_info: PropertyHintInfo::none(),
                usage: PropertyUsageFlags::DEFAULT,
            });
        }
        properties
    }

    fn on_property_get_revert(&self, property: StringName) -> Option<Variant> {
        self.asset_instance.as_ref()?;
        for prop in self.asset_instance.as_ref().unwrap().get_properties() {
            if prop.name() == property.to_string() {
                //godot_print!("{} {}", property, prop.at("prop_default"));
                return Some(prop.get_default().convert());
            }
        }
        None
    }

    fn on_get(&self, property: StringName) -> Option<Variant> {
        self.asset_instance.as_ref()?;
        if let Ok(prop) = self
            .asset_instance
            .as_ref()
            .unwrap()
            .try_get_property(&property.to_string())
        {
            return Some(prop.convert());
        }
        None
    }

    fn on_set(&mut self, property: StringName, value: Variant) -> bool {
        if self.asset_instance.is_none() {
            return false;
        }
        if self
            .asset_instance
            .as_mut()
            .unwrap()
            .try_set_property(&property.to_string(), value.convert())
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
            let mut rs = RenderingServer::singleton();
            let new_transform = self.base().get_global_transform();
            for rid in &self.managed_instances {
                rs.instance_set_transform(*rid, new_transform);
            }
        }
    }

    fn exit_tree(&mut self) {
        let mut rs = RenderingServer::singleton();
        for rid in &self.managed_instances {
            rs.free_rid(*rid);
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
                self.asset_instance = Some(asset.bind().get_instance());
            }

            //only compute if the node was already ready before the setter because this means that it was changed at edit time
            if self.base().is_node_ready() {
                self.initialize();
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

    #[func]
    fn compute(&mut self) {
        let computation_instant = Instant::now();

        //TODO receive right References and use Indexmap for that
        let inputs: Box<[Reference]> = Box::new([Reference::Standard {
            node: NodeIndex::from(0),
            socket: 0,
        }]);

        //TODO inject the child node data types once that system is complete
        let outputs = self
            .asset_instance
            .as_ref()
            .unwrap()
            .compute(Some(&inputs), self);
        let mut rs = RenderingServer::singleton();
        for (index, output) in outputs.into_iter().enumerate() {
            if output.is_none() {
                continue;
            }
            let output = output.unwrap();
            if !output.is_processable() {
                continue;
            }
            match output {
                OwnedDataType::Mesh(_) => {
                    let mesh: &Mesh = output.to_ref().dispatch_ref().unwrap();
                    let material_ref = mesh.get_material_ref();
                    println!(
                        "WOOW we made it this far we got a mesh and material is {material_ref:?}"
                    );
                    self.cached_outputs[index].write(output.convert());
                    let mut mesh_res = unsafe {
                        self.cached_outputs[index]
                            .assume_init_ref()
                            .to::<Gd<ArrayMesh>>()
                    };
                    if let Some(reference) = material_ref {
                        let material = self
                            .asset_instance
                            .as_ref()
                            .unwrap()
                            .get_reference(reference)
                            .convert()
                            .to::<Gd<Material>>();
                        self.reference_values
                            .insert(reference, material.to_variant());
                        mesh_res.surface_set_material(0, &material);
                        println!("WOOW we made it this far Material is set");
                    }
                    println!("{}", self.managed_instances[index]);
                    rs.instance_set_base(self.managed_instances[index], mesh_res.get_rid());
                }
                OwnedDataType::Curve(curve) => todo!(),
                OwnedDataType::Instance(instance) => {
                    println!("{instance:?}");
                    //Only do the following if the instance is not yet converted
                    self.reference_values
                        .entry(instance.get_reference())
                        .or_insert(
                            self.asset_instance
                                .as_ref()
                                .unwrap()
                                .get_reference(instance.get_reference())
                                .convert(),
                        );
                    rs.instance_set_base(
                        self.managed_instances[index],
                        self.reference_values[&instance.get_reference()]
                            .to::<Gd<Resource>>()
                            .get_rid(),
                    );
                    let transform = instance.get_transform();
                    // println!("{position:?}");
                    rs.instance_set_transform(
                        self.managed_instances[index],
                        self.base().get_global_transform() * transform.convert(),
                    );
                }
                OwnedDataType::Collection(collection) => {
                    //check for variance
                    let rid = rs.multimesh_create();

                    let huh_instance = rs.instance_create();
                    rs.instance_set_base(huh_instance, rid);
                    rs.instance_set_scenario(
                        huh_instance,
                        self.base().get_world_3d().unwrap().get_scenario(),
                    );

                    self.managed_instances[index] = rid;

                    rs.multimesh_allocate_data(
                        rid,
                        collection.length() as i32,
                        MultimeshTransformFormat::TRANSFORM_3D,
                    );

                    /* let mut buffer = PackedFloat32Array::new();
                    buffer.resize(collection.length() * 12);
                    rs.multimesh_set_buffer(rid, &buffer); */

                    //rs.instance_set_transform(rid, self.base().get_global_transform());
                    for (collection_index, item) in collection.iterate().enumerate() {
                        //println!("Item in collection: {item:?}");
                        let instance: &Instance = item.to_ref().dispatch_ref().unwrap();
                        self.reference_values
                            .entry(instance.get_reference())
                            .or_insert(
                                self.asset_instance
                                    .as_ref()
                                    .unwrap()
                                    .get_reference(instance.get_reference())
                                    .convert(),
                            );

                        //TODO this has only to be set 1 time if the variance is right
                        rs.multimesh_set_mesh(
                            rid,
                            self.reference_values[&instance.get_reference()]
                                .to::<Gd<Resource>>()
                                .get_rid(),
                        );

                        let transform = instance.get_transform();

                        rs.multimesh_instance_set_transform(
                            rid,
                            collection_index as i32,
                            self.base().get_global_transform() * transform.convert(),
                        );
                    }
                    // println!("{:?}", rs.multimesh_get_buffer(rid));

                    // println!("MMIC: {:?}", rs.multimesh_get_instance_count(rid));
                }
                _ => unreachable!("This cant be reached because we just checked for processable"),
            };
            //println!("{:#?}", self);
        }

        println!(
            "Godot instance computation took {} nanoseconds",
            computation_instant.elapsed().as_nanos()
        );
    }

    fn initialize(&mut self) {
        let outputs = self.asset_instance.as_ref().unwrap().get_output_sockets();
        //let mut local_sockets = Box::<[DataTypeValue]>::new_uninit_slice(sockets.len());
        let cached_output = Box::<[Variant]>::new_uninit_slice(outputs.len());

        let mut rids = Box::new_uninit_slice(outputs.len());
        let mut rs = RenderingServer::singleton();
        for (index, output) in outputs.into_iter().enumerate() {
            if output.is_processable() {
                let rid = rs.instance_create();
                rs.instance_set_scenario(rid, self.base().get_world_3d().unwrap().get_scenario());
                rs.instance_set_transform(rid, self.base().get_global_transform());
                rids[index].write(rid);
            }
        }
        self.cached_outputs = cached_output;
        self.managed_instances = unsafe { rids.assume_init() };

        let input_sockets = self.asset_instance.as_ref().unwrap().get_input_sockets();
        //println!("The input sockets are: {:#?}", input_sockets);
        let mut godot_inputs = HashMap::with_capacity(input_sockets.len());

        for (index, input) in input_sockets.into_iter().enumerate() {
            println!("{input:?}");
            //TODO somehow get exposable info from type maybe is_exposed_object ot so
            //HUH: It is not possible to guard this somehow since every input has to be accessible
            // not sure yet how this is goin to work out for every data type
            //if input.is_processable() {
            if true {
                match input.get_type() {
                    DataTypeKind::CubicBezier => {
                        if self.base().has_node(&index.to_string()) {
                            let mut path = self.base().get_node_as::<Path3D>(&index.to_string());
                            let gd_curve = path.get_curve().unwrap();
                            let godot_variant = gd_curve.to_variant();
                            godot_inputs.insert(
                                Reference::Standard {
                                    node: 0.into(),
                                    socket: index as u8,
                                },
                                (gd_curve.to_variant(), true, godot_variant.convert()),
                            );
                            let callable = self.base_mut().callable("compute");
                            path.connect("curve_changed", &callable);
                            continue;
                        }
                        let mut gd_path = Path3D::new_alloc();
                        gd_path.set_name(&index.to_string());

                        let gd_curve = Curve3D::new_gd();
                        gd_path.set_curve(&gd_curve);

                        //godot_inputs[index].write(gd_curve.to_variant());
                        let godot_variant = gd_curve.to_variant();
                        godot_inputs.insert(
                            Reference::Standard {
                                node: 0.into(),
                                socket: index as u8,
                            },
                            (gd_curve.to_variant(), true, godot_variant.convert()),
                        );

                        self.base_mut().add_child(&gd_path);

                        let scene_root =
                            self.base_mut().get_tree().get_edited_scene_root().unwrap();
                        gd_path.set_owner(&scene_root);
                        godot_print!("{:?}", gd_path)
                    }
                    _ => {
                        godot_error!(
                            "This is theoretically unreachable but so far not everything is implemented so TODO"
                        );
                    }
                }
            }
        }
        self.input_sockets = godot_inputs;
    }
}

impl ContextProvider for OneiroiInstance {
    /* fn initialize(&self) -> Option<Box<[oneiroi::nodes::OutputMetadata]>> {
        todo!()
    } */

    //TODO maybe one can somehow recompute the requested ref again.
    //The problem is the immutable reference and with refCell we can
    //no longer return a normal reference
    //Maybe it is better to convert this in the compute function since
    //in theory we know which things have changed there
    fn get_reference(&self, index: Reference) -> TypeRef {
        println!("{:#?} , index: {:?}", self.input_sockets, index);
        (&self.input_sockets.get(&index).unwrap().2).into()
    }
}
