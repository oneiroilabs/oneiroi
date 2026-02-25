use std::mem::transmute;

use ffi::{IndexedMeshBuffers, Vec3};
use oneiroi::asset::Asset;

use oneiroi::nodes::{ContextProvider, Node};
use oneiroi::type_system::Reference as InternalReference;
use oneiroi::type_system::data_types::{
    Collection as InternalCollection, CubicBezier as InternalCubicBezier,
    DataTypeKind as InternalDataTypeType, IndexedMeshBuffers as InternalIndexedMeshBuffers,
    Instance as InternalInstance, Material as InternalMaterial, Mesh as InternalMesh,
    Vec3 as InternalVec3,
};
use oneiroi::type_system::{
    OwnedDataType as InternalDataTypeValue, TypeRef as InternalDataTypeRef,
};
use oneiroi::{
    asset::instance::AssetInstance, nodes::PropertyInterface,
    serialization::deserialize_asset_v1 as internal_deserialize_asset_v1,
};

mod cache;
use cache::AssetCache;

use crate::ffi::{Color, DataTypeType, Reference, Transform};

pub struct CubicBezier(InternalCubicBezier);

impl From<InternalReference> for Reference {
    fn from(value: InternalReference) -> Self {
        unsafe { transmute::<InternalReference, Reference>(value) }
    }
}

impl From<Reference> for InternalReference {
    fn from(value: Reference) -> Self {
        unsafe { transmute::<Reference, InternalReference>(value) }
    }
}

pub struct Mesh(InternalMesh);

impl Mesh {
    fn get_mesh_buffers(&self) -> IndexedMeshBuffers {
        unsafe {
            transmute::<InternalIndexedMeshBuffers, IndexedMeshBuffers>(
                self.0.get_index_mesh_buffers(),
            )
        }
    }

    fn has_material_ref(&self) -> bool {
        self.0.get_material_ref().is_some()
    }

    fn get_material_ref(&self) -> Reference {
        self.0.get_material_ref().unwrap().into()
    }
}

pub struct Material(InternalMaterial);
impl Material {
    fn get_albedo(&self) -> Color {
        let color = self.0.get_albedo();
        let [r, g, b, a] = color.to_rgba();
        Color { r, g, b, a }
    }
}

pub struct Collection(InternalCollection);
impl Collection {
    //TODO in an ideal wold
    /* fn get_type(&self) -> DataTypeType {
        DataTypeType::Bool
    } */

    fn get_item(&self, index: usize) -> &DataTypeValue {
        unsafe {
            transmute::<&InternalDataTypeValue, &DataTypeValue>(
                self.0.iterate().nth(index).unwrap(),
            )
        }
    }

    fn length(&self) -> usize {
        self.0.length()
    }
}

pub struct Instance(InternalInstance);
impl Instance {
    fn get_transform(&self) -> Transform {
        let t = self.0.get_transform().to_cols_array();
        Transform {
            x_x: t[0],
            x_y: t[1],
            x_z: t[2],
            y_x: t[3],
            y_y: t[4],
            y_z: t[5],
            z_x: t[6],
            z_y: t[7],
            z_z: t[8],
            w_x: t[9],
            w_y: t[10],
            w_z: t[11],
        }
    }
}

pub struct DataTypeRef<'a>(InternalDataTypeRef<'a>);
impl DataTypeRef<'_> {
    fn get_mesh(&self) -> &Mesh {
        match self.0 {
            InternalDataTypeRef::Mesh(mesh) => unsafe { transmute::<&InternalMesh, &Mesh>(mesh) },
            _ => panic!("The method should only be called when the output is a Collection"),
        }
    }

    fn get_material(&self) -> &Material {
        match self.0 {
            InternalDataTypeRef::Material(val) => unsafe {
                transmute::<&InternalMaterial, &Material>(val)
            },
            _ => panic!("The method should only be called when the output is a Collection"),
        }
    }

    fn get_collection(&self) -> &Collection {
        match self.0 {
            InternalDataTypeRef::Collection(val) => unsafe {
                transmute::<&InternalCollection, &Collection>(val)
            },
            _ => panic!("The method should only be called when the output is a Collection"),
        }
    }

    fn get_instance(&self) -> &Instance {
        match self.0 {
            InternalDataTypeRef::Instance(instance) => unsafe {
                transmute::<&InternalInstance, &Instance>(instance)
            },
            _ => panic!("The method should only be called when the output is a Collection"),
        }
    }

    fn get_type(&self) -> DataTypeType {
        unsafe { std::mem::transmute::<InternalDataTypeType, DataTypeType>(self.0.get_type()) }
    }
}

pub struct DataTypeValue(InternalDataTypeValue);
impl DataTypeValue {
    //TODO
    /* fn is_processable(&self) -> bool {
    true
    //self.0.is_processable()
    //TODO
    } */

    fn get_mesh(&self) -> &Mesh {
        match &self.0 {
            InternalDataTypeValue::Mesh(mesh) => unsafe { transmute::<&InternalMesh, &Mesh>(mesh) },
            _ => panic!("The method should only be called when the output is a Collection"),
        }
    }

    fn get_material(&self) -> &Material {
        match &self.0 {
            InternalDataTypeValue::Material(val) => unsafe {
                transmute::<&InternalMaterial, &Material>(val)
            },
            _ => panic!("The method should only be called when the output is a Collection"),
        }
    }

    fn get_collection(&self) -> &Collection {
        match &self.0 {
            InternalDataTypeValue::Collection(val) => unsafe {
                transmute::<&InternalCollection, &Collection>(val)
            },
            _ => panic!("The method should only be called when the output is a Collection"),
        }
    }

    fn get_instance(&self) -> &Instance {
        match &self.0 {
            InternalDataTypeValue::Instance(instance) => unsafe {
                transmute::<&InternalInstance, &Instance>(instance)
            },
            _ => panic!("The method should only be called when the output is a Collection"),
        }
    }

    fn get_type(&self) -> DataTypeType {
        unsafe { std::mem::transmute::<InternalDataTypeType, DataTypeType>(self.0.get_data_type()) }
    }
}

fn new_value_cubic_bezier(points: Vec<Vec3>) -> Box<DataTypeValue> {
    let points = unsafe { transmute::<Vec<Vec3>, Vec<InternalVec3>>(points) };
    Box::new(DataTypeValue(InternalDataTypeValue::CubicBezier(Box::new(
        InternalCubicBezier::with_points(points),
    ))))
}

pub struct OneiroiInstance(AssetInstance);
impl OneiroiInstance {
    fn set_property_vec3(&mut self, name: &str, value: Vec3) {
        _ = self.0.try_set_property(
            name,
            InternalDataTypeValue::Vec3(unsafe { transmute::<Vec3, InternalVec3>(value) }),
        );
    }

    fn set_property_float(&mut self, name: &str, value: f32) {
        _ = self
            .0
            .try_set_property(name, InternalDataTypeValue::Float(value));
    }

    fn compute(&self, cache: &AssetCache) -> Vec<DataTypeValue> {
        self.0
            .compute(Some(cache.get_input_refs()), cache)
            .into_iter()
            .map(|d| unsafe { transmute::<InternalDataTypeValue, DataTypeValue>(d.unwrap()) })
            .collect::<Vec<_>>()
    }

    fn get_reference(&self, reference: Reference) -> Box<DataTypeRef> {
        Box::new(unsafe {
            transmute::<InternalDataTypeRef, DataTypeRef>(self.0.get_reference(reference.into()))
        })
    }
}
//mod asset;
pub struct OneiroiAsset(Asset);

impl OneiroiAsset {
    fn get_instance(&self) -> Box<OneiroiInstance> {
        Box::new(OneiroiInstance(self.0.get_instance()))
    }
}

fn deserialize_asset_v1(file_as_string: &str) -> Box<OneiroiAsset> {
    Box::new(OneiroiAsset(internal_deserialize_asset_v1(file_as_string)))
}

/* fn export_asset_v1(asset: &OneiroiAsset) -> String {
    oneiroi_export_asset_v1(&asset.0)
} */

//struct OneiroiMesh(InternalOneiroiMesh);

fn init_asset_cache(instance: &OneiroiInstance) -> Box<AssetCache> {
    Box::new(AssetCache::init(&instance.0))
}

#[cxx::bridge(namespace = "oneiroi")]
mod ffi {

    struct Reference {
        repr_u32: u32,
    }

    struct Vec3 {
        x: f32,
        y: f32,
        z: f32,
    }

    struct Color {
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    }

    //Needs to be kept in sync with the Internal counterpart
    enum DataTypeType {
        Omni,

        Selection,

        //Processable
        Mesh,
        Collider,

        Curve,
        CubicBezier,

        Instance,

        Texture,
        Material,

        //Maybe Processable
        Collection,

        //Primitives
        Vec3,
        Bool,
        Int,
        Float,
        Color,
        Transform,

        Polygon2d,
    }

    /// This struct contains all buffers required to render the mesh
    /// vertices and normals are Vecs of same length
    /// indices is a Vec divisible by 3. Every 3 vertices form a triangle
    struct IndexedMeshBuffers {
        positions: Vec<Vec3>,
        normals: Vec<Vec3>,

        indices: Vec<u32>,
    }

    /// 4x3 Colum Major Affine Transformation Matrix
    struct Transform {
        x_x: f32,
        x_y: f32,
        x_z: f32,
        y_x: f32,
        y_y: f32,
        y_z: f32,
        z_x: f32,
        z_y: f32,
        z_z: f32,
        w_x: f32,
        w_y: f32,
        w_z: f32,
    }

    //Asset and its functions
    extern "Rust" {
        type OneiroiAsset;

        fn deserialize_asset_v1(file_as_string: &str) -> Box<OneiroiAsset>;

        fn get_instance(self: &OneiroiAsset) -> Box<OneiroiInstance>;
    }

    //AssetInstance and its functions
    extern "Rust" {
        type AssetCache;
        fn init_asset_cache(instance: &OneiroiInstance) -> Box<AssetCache>;
        fn set_input_index(self: &mut AssetCache, idx: u8, value: &DataTypeValue);

        type OneiroiInstance;
        fn compute(self: &OneiroiInstance, cache: &AssetCache) -> Vec<DataTypeValue>;
        unsafe fn get_reference<'a>(
            self: &'a OneiroiInstance,
            reference: Reference,
        ) -> Box<DataTypeRef<'a>>;
        fn set_property_vec3(self: &mut OneiroiInstance, name: &str, value: Vec3);
        fn set_property_float(self: &mut OneiroiInstance, name: &str, value: f32);
    }

    extern "Rust" {
        type DataTypeValue;
        fn get_type(self: &DataTypeValue) -> DataTypeType;
        fn new_value_cubic_bezier(points: Vec<Vec3>) -> Box<DataTypeValue>;

        //Retrieve DataTypeValue

        fn get_collection(self: &DataTypeValue) -> &Collection;
        fn get_instance(self: &DataTypeValue) -> &Instance;
        fn get_mesh(self: &DataTypeValue) -> &Mesh;
        fn get_material(self: &DataTypeValue) -> &Material;

        type DataTypeRef<'a>;
        unsafe fn get_collection<'a>(self: &'a DataTypeRef<'a>) -> &'a Collection;
        unsafe fn get_instance<'a>(self: &'a DataTypeRef) -> &'a Instance;
        unsafe fn get_mesh<'a>(self: &'a DataTypeRef) -> &'a Mesh;
        unsafe fn get_material<'a>(self: &'a DataTypeRef) -> &'a Material;

        type CubicBezier;

        type Collection;
        fn length(self: &Collection) -> usize;
        fn get_item(self: &Collection, index: usize) -> &DataTypeValue;

        type Instance;
        fn get_transform(self: &Instance) -> Transform;

        type Mesh;
        fn get_mesh_buffers(self: &Mesh) -> IndexedMeshBuffers;
        fn has_material_ref(self: &Mesh) -> bool;
        fn get_material_ref(self: &Mesh) -> Reference;

        type Material;
        fn get_albedo(self: &Material) -> Color;

    }
}
