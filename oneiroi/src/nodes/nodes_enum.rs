use serde::{Deserialize, Serialize};

use crate::{
    asset::instance::AssetInstance,
    property::PropertyMetadata,
    type_system::{OwnedDataType, Reference, TypeRef, data_types::TypeDescriptor},
};

use crate::nodes::*;

use super::{
    ContextProvider, Node, PropertyInterface, PropertyNotFound, SetPropertyError, SocketInterface,
    StaticNodeMetadata,
};

//Since the enum is tagged in serde we can erase the versioning for the Nodes enum
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Nodes {
    //Internal
    Expose, //Only 1 of this should be in the graph root and edges to it indicate Exposed Variables

    //Producers
    BoxV1(Box<BoxV1>),
    CylinderV1(Box<CylinderV1>),
    DistributePointsV1(Box<DistributePointsV1>),
    MaterialV1(Box<MaterialV1>),
    PolygonV1(Box<PolygonV1>),

    //Modifiers
    BevelV1(Box<BevelV1>),
    ExtrudeV1(Box<ExtrudeV1>),
    SetMaterialV1(Box<SetMaterialV1>),

    //Transformers
    CreateInstanceV1(Box<CreateInstanceV1>),
    InstancesFromTransformsV1(Box<InstancesFromTransformsV1>),
    SampleCurveV1(Box<SampleCurveV1>),
    SweepV1(Box<SweepV1>),

    //Control Flow
    SocketInput(Box<SocketInputV1>),
    SocketOutput(Box<SocketOutputV1>),

    //Asset
    EmbeddedAsset(Box<AssetInstance>),
}

impl Nodes {
    pub fn get_node_types() {} //TODO

    pub fn is_output_node(&self) -> bool {
        matches!(self, Nodes::SocketOutput(_))
    }

    pub fn is_input_node(&self) -> bool {
        matches!(self, Nodes::SocketInput(_))
    }

    pub(crate) fn get_embedded_instance(&mut self) -> &mut AssetInstance {
        match self {
            Nodes::EmbeddedAsset(instance) => instance,
            _ => panic!(),
        }
    }

    /* pub(crate) fn set_property_external(
        &mut self,
        index: u8,
        reference: Reference,
    ) -> Result<(), SetPropertyError> {
        match self {
            Nodes::Expose => unreachable!(),
            Nodes::BoxV1(node) => node.,
            Nodes::CylinderV1(node) => todo!(),
            Nodes::DistributePointsV1(node) => todo!(),
            Nodes::MaterialV1(node) => todo!(),
            Nodes::ExtrudeV1(node) => todo!(),
            Nodes::SetMaterialV1(node) => todo!(),
            Nodes::CreateInstanceV1(node) => todo!(),
            Nodes::InstancesFromPointsV1(node) => todo!(),
            Nodes::SampleCurveV1(node) => todo!(),
            Nodes::SocketInput(node) => todo!(),
            Nodes::SocketOutput(node) => todo!(),
            Nodes::EmbeddedAsset(node) => todo!(),
        }
    } */

    //pub fn get_static_metadata(&self) -> StaticNodeMetadata {

    pub(crate) fn from_alias(alias: &str) -> Self {
        match alias {
            "Box" => Nodes::BoxV1(Box::default()),
            "Cylinder" => Nodes::CylinderV1(Box::default()),
            "Extrude" => Nodes::ExtrudeV1(Box::default()),
            "Bevel" => Nodes::BevelV1(Box::default()),
            "Input" => Nodes::SocketInput(Box::default()),
            "Output" => Nodes::SocketOutput(Box::default()),
            "CreateInstance" => Nodes::CreateInstanceV1(Box::default()),
            "DistributePoints" => Nodes::DistributePointsV1(Box::default()),
            "InstancesFromPoints" => Nodes::InstancesFromTransformsV1(Box::default()),
            "SampleCurve" => Nodes::SampleCurveV1(Box::default()),
            "Material" => Nodes::MaterialV1(Box::default()),
            "SetMaterial" => Nodes::SetMaterialV1(Box::default()),
            "Sweep" => Nodes::SweepV1(Box::default()),
            "Polygon" => Nodes::PolygonV1(Box::default()),
            _ => panic!("This should not panic but instead return an error"),
        }
    }
}

impl PropertyInterface for Nodes {
    fn try_set_property(
        &mut self,
        property: &str,
        value: OwnedDataType,
    ) -> Result<(), SetPropertyError> {
        match self {
            Nodes::Expose => unimplemented!(),
            Nodes::BoxV1(box_v1) => box_v1.try_set_property(property, value),
            Nodes::ExtrudeV1(extrude_v1) => extrude_v1.try_set_property(property, value),
            Nodes::SocketInput(socket_input_v1) => {
                socket_input_v1.try_set_property(property, value)
            }
            Nodes::SocketOutput(socket_output_v1) => {
                socket_output_v1.try_set_property(property, value)
            }
            Nodes::EmbeddedAsset(asset_instance) => {
                asset_instance.try_set_property(property, value)
            }
            Nodes::CylinderV1(cylinder_v1) => cylinder_v1.try_set_property(property, value),
            Nodes::CreateInstanceV1(create_instance_v1) => {
                create_instance_v1.try_set_property(property, value)
            }
            Nodes::DistributePointsV1(distribute_points_v1) => {
                distribute_points_v1.try_set_property(property, value)
            }
            Nodes::InstancesFromTransformsV1(instances_from_points_v1) => {
                instances_from_points_v1.try_set_property(property, value)
            }
            Nodes::SampleCurveV1(sample_curve_v1) => {
                sample_curve_v1.try_set_property(property, value)
            }
            Nodes::MaterialV1(material_v1) => material_v1.try_set_property(property, value),
            Nodes::SetMaterialV1(set_material_v1) => {
                set_material_v1.try_set_property(property, value)
            }
            Nodes::SweepV1(sweep_v1) => sweep_v1.try_set_property(property, value),
            Nodes::PolygonV1(polygon_v1) => polygon_v1.try_set_property(property, value),
            Nodes::BevelV1(bevel_v1) => bevel_v1.try_set_property(property, value),
        }
    }

    fn try_get_property(&self, property: &str) -> Result<TypeRef, PropertyNotFound> {
        match self {
            Nodes::Expose => unimplemented!(),
            Nodes::BoxV1(box_v1) => box_v1.try_get_property(property),
            Nodes::ExtrudeV1(extrude_v1) => extrude_v1.try_get_property(property),
            Nodes::SocketInput(socket_input_v1) => socket_input_v1.try_get_property(property),
            Nodes::SocketOutput(socket_output_v1) => socket_output_v1.try_get_property(property),
            Nodes::EmbeddedAsset(asset_instance) => asset_instance.try_get_property(property),
            Nodes::CylinderV1(cylinder_v1) => cylinder_v1.try_get_property(property),
            Nodes::CreateInstanceV1(create_instance_v1) => {
                create_instance_v1.try_get_property(property)
            }
            Nodes::DistributePointsV1(distribute_points_v1) => {
                distribute_points_v1.try_get_property(property)
            }
            Nodes::InstancesFromTransformsV1(instances_from_points_v1) => {
                instances_from_points_v1.try_get_property(property)
            }
            Nodes::SampleCurveV1(sample_curve_v1) => sample_curve_v1.try_get_property(property),
            Nodes::MaterialV1(material_v1) => material_v1.try_get_property(property),
            Nodes::SetMaterialV1(set_material_v1) => set_material_v1.try_get_property(property),
            Nodes::SweepV1(sweep_v1) => sweep_v1.try_get_property(property),
            Nodes::PolygonV1(polygon_v1) => polygon_v1.try_get_property(property),
            Nodes::BevelV1(bevel_v1) => bevel_v1.try_get_property(property),
        }
    }

    fn get_properties(&self) -> Box<[PropertyMetadata]> {
        match self {
            Nodes::Expose => unimplemented!(),
            Nodes::BoxV1(box_v1) => box_v1.get_properties(),
            Nodes::ExtrudeV1(extrude_v1) => extrude_v1.get_properties(),
            Nodes::SocketInput(socket_input_v1) => socket_input_v1.get_properties(),
            Nodes::SocketOutput(socket_output_v1) => socket_output_v1.get_properties(),
            Nodes::EmbeddedAsset(asset_instance) => asset_instance.get_properties(),
            Nodes::CylinderV1(cylinder_v1) => cylinder_v1.get_properties(),
            Nodes::CreateInstanceV1(create_instance_v1) => create_instance_v1.get_properties(),
            Nodes::DistributePointsV1(distribute_points_v1) => {
                distribute_points_v1.get_properties()
            }
            Nodes::InstancesFromTransformsV1(instances_from_points_v1) => {
                instances_from_points_v1.get_properties()
            }
            Nodes::SampleCurveV1(sample_curve_v1) => sample_curve_v1.get_properties(),
            Nodes::MaterialV1(material_v1) => material_v1.get_properties(),
            Nodes::SetMaterialV1(set_material_v1) => set_material_v1.get_properties(),
            Nodes::SweepV1(sweep_v1) => sweep_v1.get_properties(),
            Nodes::PolygonV1(polygon_v1) => polygon_v1.get_properties(),
            Nodes::BevelV1(bevel_v1) => bevel_v1.get_properties(),
        }
    }

    fn try_set_property_index(
        &mut self,
        index: u8,
        value: OwnedDataType,
    ) -> Result<(), SetPropertyError> {
        todo!()
    }

    fn try_get_property_index(&self, index: u8) -> Result<TypeRef, PropertyNotFound> {
        todo!()
    }

    fn set_property_external(
        &mut self,
        index: u8,
        reference: Reference,
    ) -> Result<(), SetPropertyError> {
        match self {
            Nodes::Expose => unreachable!(),
            Nodes::BoxV1(node) => node.set_property_external(index, reference),
            Nodes::CylinderV1(node) => node.set_property_external(index, reference),
            Nodes::DistributePointsV1(node) => node.set_property_external(index, reference),
            Nodes::MaterialV1(node) => node.set_property_external(index, reference),
            Nodes::ExtrudeV1(node) => node.set_property_external(index, reference),
            Nodes::SetMaterialV1(node) => node.set_property_external(index, reference),
            Nodes::CreateInstanceV1(node) => node.set_property_external(index, reference),
            Nodes::InstancesFromTransformsV1(node) => node.set_property_external(index, reference),
            Nodes::SampleCurveV1(node) => node.set_property_external(index, reference),
            Nodes::SocketInput(node) => node.set_property_external(index, reference),
            Nodes::SocketOutput(node) => node.set_property_external(index, reference),
            Nodes::EmbeddedAsset(node) => node.set_property_external(index, reference),
            Nodes::SweepV1(node) => node.set_property_external(index, reference),
            Nodes::PolygonV1(node) => node.set_property_external(index, reference),
            Nodes::BevelV1(bevel_v1) => bevel_v1.set_property_external(index, reference),
        }
    }

    /* fn try_get_property_script(&self, property: &str) -> Result<String, PropertyNotFound> {
        match self {
            Nodes::Expose => unimplemented!(),
            Nodes::BoxV1(box_v1) => box_v1.try_get_property_script(property),
            Nodes::ExtrudeV1(extrude_v1) => extrude_v1.try_get_property_script(property),
            Nodes::SocketInput(socket_input_v1) => {
                socket_input_v1.try_get_property_script(property)
            }
            Nodes::SocketOutput(socket_output_v1) => {
                socket_output_v1.try_get_property_script(property)
            }
            Nodes::EmbeddedAsset(asset_instance) => {
                asset_instance.try_get_property_script(property)
            }
            Nodes::CylinderV1(cylinder_v1) => cylinder_v1.try_get_property_script(property),
            Nodes::CreateInstanceV1(create_instance_v1) => {
                create_instance_v1.try_get_property_script(property)
            }
            Nodes::DistributePointsV1(distribute_points_v1) => {
                distribute_points_v1.try_get_property_script(property)
            }
            Nodes::InstancesFromPointsV1(instances_from_points_v1) => {
                instances_from_points_v1.try_get_property_script(property)
            }
            Nodes::SampleCurveV1(sample_curve_v1) => {
                sample_curve_v1.try_get_property_script(property)
            }
        }
    } */

    /* fn try_get_property_metadata(&self, property: &str) -> Result<PropertyMetadata, ()> {
        match self {
            Nodes::Expose => unimplemented!(),
            Nodes::BoxV1(box_v1) => box_v1.try_get_property_metadata(property),
            Nodes::CylinderV1(cylinder_v1) => cylinder_v1.try_get_property_metadata(property),
            Nodes::ExtrudeV1(extrude_v1) => extrude_v1.try_get_property_metadata(property),
            Nodes::SocketInput(socket_input_v1) => todo!(), /* {
            socket_input_v1.try_get_property_metadata(property)
            } */
            Nodes::SocketOutput(socket_output_v1) => {
                socket_output_v1.try_get_property_metadata(property)
            }
            Nodes::EmbeddedAsset(asset_instance) => {
                asset_instance.try_get_property_metadata(property)
            }
        }
    } */
}

impl Node for Nodes {
    fn compute(
        &self,
        input_sockets: Option<&[Reference]>,
        context: &impl ContextProvider,
    ) -> Box<[Option<OwnedDataType>]> {
        match self {
            Nodes::Expose => unreachable!(),
            Nodes::SocketInput(_) => unreachable!(),
            Nodes::SocketOutput(_) => unreachable!(),
            Nodes::BoxV1(box_v1) => box_v1.compute(input_sockets, context),
            Nodes::ExtrudeV1(extrude_v1) => extrude_v1.compute(input_sockets, context),
            Nodes::EmbeddedAsset(asset_template) => asset_template.compute(input_sockets, context),
            Nodes::CylinderV1(cylinder_v1) => cylinder_v1.compute(input_sockets, context),
            Nodes::CreateInstanceV1(create_instance_v1) => {
                create_instance_v1.compute(input_sockets, context)
            }
            Nodes::DistributePointsV1(distribute_points_v1) => {
                distribute_points_v1.compute(input_sockets, context)
            }
            Nodes::InstancesFromTransformsV1(instances_from_points_v1) => {
                instances_from_points_v1.compute(input_sockets, context)
            }
            Nodes::SampleCurveV1(sample_curve_v1) => {
                sample_curve_v1.compute(input_sockets, context)
            }
            Nodes::MaterialV1(material_v1) => material_v1.compute(input_sockets, context),
            Nodes::SetMaterialV1(set_material_v1) => {
                set_material_v1.compute(input_sockets, context)
            }
            Nodes::SweepV1(sweep_v1) => sweep_v1.compute(input_sockets, context),
            Nodes::PolygonV1(node) => node.compute(input_sockets, context),
            Nodes::BevelV1(bevel_v1) => bevel_v1.compute(input_sockets, context),
        }
    }

    fn node_metadata(&self) -> StaticNodeMetadata {
        match self {
            Nodes::Expose => unimplemented!(),
            Nodes::BoxV1(var) => var.node_metadata(),
            Nodes::CylinderV1(var) => var.node_metadata(),
            Nodes::ExtrudeV1(var) => var.node_metadata(),
            Nodes::SocketInput(var) => var.node_metadata(),
            Nodes::SocketOutput(var) => var.node_metadata(),
            Nodes::EmbeddedAsset(var) => var.node_metadata(),
            Nodes::CreateInstanceV1(create_instance_v1) => create_instance_v1.node_metadata(),
            Nodes::DistributePointsV1(distribute_points_v1) => distribute_points_v1.node_metadata(),
            Nodes::InstancesFromTransformsV1(instances_from_points_v1) => {
                instances_from_points_v1.node_metadata()
            }
            Nodes::SampleCurveV1(sample_curve_v1) => sample_curve_v1.node_metadata(),
            Nodes::MaterialV1(material_v1) => material_v1.node_metadata(),
            Nodes::SetMaterialV1(set_material_v1) => set_material_v1.node_metadata(),
            Nodes::SweepV1(sweep_v1) => sweep_v1.node_metadata(),
            Nodes::PolygonV1(polygon_v1) => polygon_v1.node_metadata(),
            Nodes::BevelV1(bevel_v1) => bevel_v1.node_metadata(),
        }
    }
    /* fn parse_sockets(input_sockets: Vec<&DataTypeInstance>) -> Result<Self::InputSockets, ()> {
        todo!()
    } */

    /* fn get_sockets(&self) -> (Vec<DataTypeType>, Vec<DataTypeType>) {
        match self {
            Nodes::Expose => todo!(),
            Nodes::BoxV1(box_v1) => box_v1.get_sockets(),
            Nodes::ExtrudeV1(extrude_v1) => extrude_v1.get_sockets(),
            Nodes::SocketInput(socket_input_v1) => todo!(),
            Nodes::SocketOutput(socket_output_v1) => socket_output_v1.get_sockets(),
            Nodes::EmbeddedAsset(asset_template) => asset_template.get_sockets(),
            Nodes::CylinderV1(cylinder_v1) => cylinder_v1.get_sockets(),
        }
    } */
}
impl SocketInterface for Nodes {
    fn get_output_sockets(&self) -> Box<[TypeDescriptor]> {
        match self {
            Nodes::Expose => unimplemented!(),
            Nodes::BoxV1(box_v1) => box_v1.get_output_sockets(),
            Nodes::ExtrudeV1(extrude_v1) => extrude_v1.get_output_sockets(),
            Nodes::SocketInput(socket_input_v1) => socket_input_v1.get_output_sockets(),
            Nodes::SocketOutput(socket_output_v1) => socket_output_v1.get_output_sockets(),
            Nodes::EmbeddedAsset(asset_template) => asset_template.get_output_sockets(),
            Nodes::CylinderV1(cylinder_v1) => cylinder_v1.get_output_sockets(),
            Nodes::CreateInstanceV1(create_instance_v1) => create_instance_v1.get_output_sockets(),
            Nodes::DistributePointsV1(distribute_points_v1) => {
                distribute_points_v1.get_output_sockets()
            }
            Nodes::InstancesFromTransformsV1(instances_from_points_v1) => {
                instances_from_points_v1.get_output_sockets()
            }
            Nodes::SampleCurveV1(sample_curve_v1) => sample_curve_v1.get_output_sockets(),
            Nodes::MaterialV1(material_v1) => material_v1.get_output_sockets(),
            Nodes::SetMaterialV1(set_material_v1) => set_material_v1.get_output_sockets(),
            Nodes::SweepV1(node) => node.get_output_sockets(),
            Nodes::PolygonV1(polygon_v1) => polygon_v1.get_output_sockets(),
            Nodes::BevelV1(bevel_v1) => bevel_v1.get_output_sockets(),
        }
    }

    fn get_input_sockets(&self) -> Box<[TypeDescriptor]> {
        match self {
            Nodes::Expose => unimplemented!(),
            Nodes::BoxV1(box_v1) => box_v1.get_input_sockets(),
            Nodes::ExtrudeV1(extrude_v1) => extrude_v1.get_input_sockets(),
            Nodes::SocketInput(socket_input_v1) => socket_input_v1.get_input_sockets(),
            Nodes::SocketOutput(socket_output_v1) => socket_output_v1.get_input_sockets(),
            Nodes::EmbeddedAsset(asset_template) => asset_template.get_input_sockets(),
            Nodes::CylinderV1(cylinder_v1) => cylinder_v1.get_input_sockets(),
            Nodes::CreateInstanceV1(create_instance_v1) => create_instance_v1.get_input_sockets(),
            Nodes::DistributePointsV1(distribute_points_v1) => {
                distribute_points_v1.get_input_sockets()
            }
            Nodes::InstancesFromTransformsV1(instances_from_points_v1) => {
                instances_from_points_v1.get_input_sockets()
            }
            Nodes::SampleCurveV1(sample_curve_v1) => sample_curve_v1.get_input_sockets(),
            Nodes::MaterialV1(material_v1) => material_v1.get_input_sockets(),
            Nodes::SetMaterialV1(set_material_v1) => set_material_v1.get_input_sockets(),
            Nodes::SweepV1(node) => node.get_input_sockets(),
            Nodes::PolygonV1(polygon_v1) => polygon_v1.get_input_sockets(),
            Nodes::BevelV1(bevel_v1) => bevel_v1.get_input_sockets(),
        }
    }
}
