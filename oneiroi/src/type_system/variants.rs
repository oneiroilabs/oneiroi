use glam::{Affine3A, Vec3};
use serde::{Deserialize, Serialize};

use crate::{
    property::{Property, PropertyInstance},
    type_system::{
        TypeKind,
        data_types::{
            Collection, Collider, Color, CubicBezier, Curve, DataType, DataTypeKind, Instance,
            Material, Mesh, Outline, Selection, Texture,
        },
        trait_types::{MeshMut0D, SequentialSample, TraitType, TraitTypeKind},
    },
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum OwnedDataType {
    //Primitives
    Vec3(Vec3),
    Float(f32),
    Bool(bool),
    Int(i64),
    Transform(Box<Affine3A>),
    Color(Box<Color>),

    //Processable
    Mesh(Box<Mesh>),
    Collider(Box<Collider>),

    Texture(Box<Texture>),
    Material(Box<Material>),

    Curve(Box<Curve>),
    CubicBezier(Box<CubicBezier>),

    //Utility
    Selection(Box<Selection>),
    Collection(Box<Collection>),
    Instance(Box<Instance>),
    Outline(Box<Outline>),
}
impl OwnedDataType {
    pub fn is_processable(&self) -> bool {
        match self {
            OwnedDataType::Vec3(_) => false,
            OwnedDataType::Float(_) => false,
            OwnedDataType::Bool(_) => false,
            OwnedDataType::Int(_) => false,
            OwnedDataType::Mesh(_) => true,
            OwnedDataType::Curve(_) => true,
            OwnedDataType::Selection(_) => false,
            OwnedDataType::Instance(_) => true,
            OwnedDataType::Collection(collection) => collection.get_type().is_processable(),
            OwnedDataType::Material(_) => true,
            OwnedDataType::Collider(_) => true,
            OwnedDataType::CubicBezier(_) => true,
            OwnedDataType::Transform(affine3_a) => false,
            OwnedDataType::Color(color) => false,
            OwnedDataType::Outline(polygon2d) => false,
            OwnedDataType::Texture(texture) => false,
        }
    }

    #[inline]
    pub fn to_ref(&self) -> TypeRef {
        self.into()
    }

    pub(crate) fn dispatch<T: DataType>(self) -> Result<T, ()> {
        if T::DATA_TYPE_TYPE == self.get_data_type() {
            Ok(T::get_type(self))
        } else {
            Err(())
        }
    }

    #[inline]
    pub(crate) fn new<T: DataType>(value: T) -> Self {
        value.to_data_type_value()
    }

    pub fn get_data_type(&self) -> DataTypeKind {
        match self {
            OwnedDataType::Vec3(_) => DataTypeKind::Vec3,
            OwnedDataType::Float(_) => DataTypeKind::Float,
            OwnedDataType::Mesh(_) => DataTypeKind::Mesh,
            OwnedDataType::Selection(_) => DataTypeKind::Selection,
            OwnedDataType::Int(_) => DataTypeKind::Int,
            OwnedDataType::Bool(_) => DataTypeKind::Bool,
            OwnedDataType::Instance(_) => DataTypeKind::Instance,
            OwnedDataType::Curve(_) => DataTypeKind::Curve,
            OwnedDataType::Collection(_) => DataTypeKind::Collection,
            OwnedDataType::Material(_) => DataTypeKind::Material,
            OwnedDataType::Collider(_) => DataTypeKind::Collider,
            OwnedDataType::CubicBezier(_) => DataTypeKind::CubicBezier,
            OwnedDataType::Transform(_) => DataTypeKind::Transform,
            OwnedDataType::Color(_) => DataTypeKind::Color,
            OwnedDataType::Outline(_) => DataTypeKind::Outline,
            OwnedDataType::Texture(_) => DataTypeKind::Texture,
        }
    }

    //TODO this should most likely not be in here
    pub fn get_instance(self) -> PropertyInstance {
        match self {
            OwnedDataType::Vec3(vec3) => PropertyInstance::Vec3(Property::new(vec3)),
            OwnedDataType::Float(_) => todo!(),
            OwnedDataType::Bool(_) => todo!(),
            OwnedDataType::Int(_) => todo!(),
            OwnedDataType::Mesh(oneiroi_mesh) => todo!(),
            OwnedDataType::Selection(selection) => todo!(),
            OwnedDataType::Instance(instance) => todo!(),
            OwnedDataType::Curve(curve) => todo!(),
            OwnedDataType::Collection(collection) => todo!(),
            OwnedDataType::Material(material) => todo!(),
            OwnedDataType::Collider(collider) => todo!(),
            OwnedDataType::CubicBezier(cubic_bezier) => todo!(),
            OwnedDataType::Transform(affine3_a) => todo!(),
            OwnedDataType::Color(color) => todo!(),
            OwnedDataType::Outline(_) => todo!(),
            OwnedDataType::Texture(texture) => todo!(),
        }
    }
}

#[derive(Debug)]
pub enum TypeRef<'a> {
    //Primitives
    Vec3(&'a Vec3),
    Float(&'a f32),
    Bool(&'a bool),
    Int(&'a i64),
    Transform(&'a Affine3A),
    Color(&'a Color),

    //Processable
    Mesh(&'a Mesh),
    Collider(&'a Collider),

    Texture(&'a Texture),
    Material(&'a Material),

    Curve(&'a Curve),
    CubicBezier(&'a CubicBezier),

    //Utility
    Selection(&'a Selection),
    Collection(&'a Collection),
    Instance(&'a Instance),
    Outline(&'a Outline),

    SequentialSample(&'a dyn SequentialSample),
}

impl<'a> TypeRef<'a> {
    pub fn get_type(&self) -> DataTypeKind {
        match self {
            TypeRef::Vec3(_) => DataTypeKind::Vec3,
            TypeRef::Float(_) => DataTypeKind::Float,
            TypeRef::Mesh(_) => DataTypeKind::Mesh,
            TypeRef::Selection(_) => DataTypeKind::Selection,
            TypeRef::Int(_) => DataTypeKind::Int,
            TypeRef::Bool(_) => DataTypeKind::Bool,
            TypeRef::Instance(_) => DataTypeKind::Instance,
            TypeRef::Curve(_) => DataTypeKind::Curve,
            TypeRef::Collection(_) => DataTypeKind::Collection,
            TypeRef::Material(_) => DataTypeKind::Material,
            TypeRef::Collider(_) => DataTypeKind::Collider,
            TypeRef::CubicBezier(_) => DataTypeKind::CubicBezier,
            TypeRef::Transform(_) => DataTypeKind::Transform,
            TypeRef::Color(_) => DataTypeKind::Color,
            TypeRef::Outline(_) => DataTypeKind::Outline,
            TypeRef::Texture(_) => DataTypeKind::Texture,
            TypeRef::SequentialSample(sequential_sample) => todo!(),
        }
    }

    pub fn get_trait(&self) -> TraitTypeKind {
        match self {
            TypeRef::SequentialSample(_) => TraitTypeKind::SequentialSample,
            _ => unreachable!(),
        }
    }

    /* pub fn dispatch<T: DataType>(self) -> Result<T, ()> {
        if T::DATA_TYPE_TYPE == self.get_type() {
            Ok(T::get_type(self))
        } else {
            Err(())
        }
    } */

    pub fn dispatch_ref<T: DataType>(self) -> Result<&'a T, ()> {
        if T::DATA_TYPE_TYPE == self.get_type() {
            Ok(T::get_type_ref(self))
        } else {
            Err(())
        }
    }

    pub fn dispatch_trait<T: TraitType<'a>>(self) -> Result<T, ()> {
        if T::TRAIT_TYPE_KIND == self.get_trait() {
            Ok(T::get_type_ref(self))
        } else {
            Err(())
        }

        /*         if self
         */
    }
}

impl<'a> From<&'a OwnedDataType> for TypeRef<'a> {
    #[inline]
    fn from(value: &'a OwnedDataType) -> Self {
        match value {
            OwnedDataType::Vec3(vec3) => TypeRef::Vec3(vec3),
            OwnedDataType::Float(f) => TypeRef::Float(f),
            OwnedDataType::Bool(b) => TypeRef::Bool(b),
            OwnedDataType::Int(i) => TypeRef::Int(i),
            OwnedDataType::Transform(affine3_a) => TypeRef::Transform(affine3_a),
            OwnedDataType::Color(color) => TypeRef::Color(&color),
            OwnedDataType::Mesh(mesh) => TypeRef::Mesh(&mesh),
            OwnedDataType::Collider(collider) => TypeRef::Collider(&collider),
            OwnedDataType::Material(material) => TypeRef::Material(&material),
            OwnedDataType::Curve(curve) => TypeRef::Curve(&curve),
            OwnedDataType::CubicBezier(cubic_bezier) => TypeRef::CubicBezier(&cubic_bezier),
            OwnedDataType::Selection(selection) => TypeRef::Selection(&selection),
            OwnedDataType::Collection(collection) => TypeRef::Collection(&collection),
            OwnedDataType::Instance(instance) => TypeRef::Instance(&instance),
            OwnedDataType::Outline(outline) => TypeRef::Outline(&outline),
            OwnedDataType::Texture(texture) => TypeRef::Texture(&texture),
        }
    }
}

impl From<TypeRef<'_>> for OwnedDataType {
    fn from(value: TypeRef) -> Self {
        match value {
            TypeRef::Vec3(vec3) => OwnedDataType::Vec3(*vec3),
            TypeRef::Float(float) => OwnedDataType::Float(*float),
            TypeRef::Bool(bool) => OwnedDataType::Bool(*bool),
            TypeRef::Int(int) => OwnedDataType::Int(*int),
            TypeRef::Transform(affine3_a) => OwnedDataType::Transform(Box::new(*affine3_a)),
            TypeRef::Color(color) => OwnedDataType::Color(Box::new(*color)),
            TypeRef::Mesh(mesh) => OwnedDataType::Mesh(Box::new(mesh.clone())),
            TypeRef::Collider(collider) => OwnedDataType::Collider(Box::new(collider.clone())),
            TypeRef::Material(material) => OwnedDataType::Material(Box::new(material.clone())),
            TypeRef::Curve(curve) => OwnedDataType::Curve(Box::new(curve.clone())),
            TypeRef::CubicBezier(cubic_bezier) => {
                OwnedDataType::CubicBezier(Box::new(cubic_bezier.clone()))
            }
            TypeRef::Selection(selection) => OwnedDataType::Selection(Box::new(selection.clone())),
            TypeRef::Collection(collection) => {
                OwnedDataType::Collection(Box::new(collection.clone()))
            }
            TypeRef::Instance(instance) => OwnedDataType::Instance(Box::new(instance.clone())),
            TypeRef::Outline(outline) => OwnedDataType::Outline(Box::new(outline.clone())),
            TypeRef::Texture(texture) => OwnedDataType::Texture(Box::new(texture.clone())),

            // Traits cannot be converted to an owned type.
            TypeRef::SequentialSample(_) => unreachable!(),
        }
    }
}
