use std::mem;

use godot::{
    builtin::{Color as GDColor, Variant, VariantArray, VariantType, Vector3},
    classes::{
        ArrayMesh, Curve3D, Material as GDMaterial, StandardMaterial3D,
        mesh::{ArrayType, PrimitiveType},
    },
    meta::ToGodot,
    prelude::*,
};
use oneiroi::type_system::{
    OwnedDataType, TypeRef,
    data_types::{
        Collection, Collider, Color, CubicBezier, Curve, DataTypeKind, Instance, Material, Mesh,
        Outline, Selection, Texture, Transform, Vec3, Vec3A,
    },
};

pub trait OneiroiToGodot {
    //TODO this method name most liely sucks
    fn variant_type(&self) -> VariantType;
}

impl OneiroiToGodot for DataTypeKind {
    fn variant_type(&self) -> VariantType {
        match self {
            DataTypeKind::Omni => unimplemented!(),
            DataTypeKind::Mesh => unimplemented!(),
            DataTypeKind::Collection => todo!(),
            DataTypeKind::Instance => unimplemented!(),
            DataTypeKind::Curve => todo!(),
            DataTypeKind::Vec3 => VariantType::VECTOR3,
            DataTypeKind::Int => VariantType::INT,
            DataTypeKind::Float => VariantType::FLOAT,
            DataTypeKind::Bool => VariantType::BOOL,
            DataTypeKind::Collider => todo!(),
            DataTypeKind::Selection => VariantType::STRING,
            DataTypeKind::Material => todo!(),
            DataTypeKind::CubicBezier => VariantType::OBJECT,
            DataTypeKind::Transform => VariantType::TRANSFORM3D,
            DataTypeKind::Color => VariantType::COLOR,
            DataTypeKind::Outline => todo!(),
            DataTypeKind::Texture => todo!(),
        }
    }
}

/// This trait handles the Conversion of the Data Types between Godot and Oneiroi.
pub(crate) trait TypeConvert {
    type Target;
    /// Executes the Type Conversion to or from Godot.
    fn convert(self) -> Self::Target;
}

impl TypeConvert for Vector3 {
    type Target = Vec3;

    fn convert(self) -> Self::Target {
        //TODO inspect assembly if we need to transmute here because Vec3 is quite essential
        Vec3::new(self.x, self.y, self.z)
    }
}

impl TypeConvert for Vec3 {
    type Target = Vector3;

    fn convert(self) -> Self::Target {
        //TODO inspect assembly if we need to transmute here because Vec3 is quite essential
        Vector3::new(self.x, self.y, self.z)
    }
}

/// Since sometimes we receive Vec3A from Oneiroi we also need this impl
impl TypeConvert for Vec3A {
    type Target = Vector3;

    fn convert(self) -> Self::Target {
        //TODO inspect assembly if we need to transmute here because Vec3 is quite essential
        Vector3::new(self.x, self.y, self.z)
    }
}

impl TypeConvert for Gd<ArrayMesh> {
    type Target = Mesh;

    fn convert(self) -> Self::Target {
        todo!()
    }
}

impl TypeConvert for Mesh {
    type Target = Gd<ArrayMesh>;

    fn convert(self) -> Self::Target {
        let buffers = self.get_index_mesh_buffers();

        let mut mesh = ArrayMesh::new_gd();

        let mut surface = VariantArray::new();
        surface.resize(ArrayType::MAX.ord().try_into().unwrap(), &Variant::nil());
        surface.set(
            ArrayType::VERTEX.ord().try_into().unwrap(),
            &PackedVector3Array::from(unsafe {
                mem::transmute::<Vec<Vec3>, Vec<Vector3>>(buffers.positions)
            })
            .to_variant(),
        );

        surface.set(
            ArrayType::NORMAL.ord().try_into().unwrap(),
            &PackedVector3Array::from(unsafe {
                mem::transmute::<Vec<Vec3>, Vec<Vector3>>(buffers.normals)
            })
            .to_variant(),
        );

        //Since Godot uses clockwise winding order we reverse the iterator
        surface.set(
            ArrayType::INDEX.ord().try_into().unwrap(),
            &PackedInt32Array::from_iter(buffers.indices.into_iter().map(|int| int as i32).rev())
                .to_variant(),
        );
        mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &surface);
        mesh
    }
}

impl TypeConvert for Transform {
    type Target = Transform3D;

    fn convert(self) -> Self::Target {
        let basis = Basis::from_cols(
            self.matrix3.col(0).convert(),
            self.matrix3.col(1).convert(),
            self.matrix3.col(2).convert(),
        );
        Transform3D::new(basis, self.translation.convert())
    }
}

impl TypeConvert for Transform3D {
    type Target = Transform;

    fn convert(self) -> Self::Target {
        Transform::from_cols(
            self.basis.col_a().convert().into(),
            self.basis.col_a().convert().into(),
            self.basis.col_a().convert().into(),
            self.origin.convert().into(),
        )
    }
}

impl TypeConvert for CubicBezier {
    type Target = Gd<Curve3D>;

    fn convert(self) -> Self::Target {
        todo!()
    }
}

impl TypeConvert for Color {
    type Target = GDColor;

    fn convert(self) -> Self::Target {
        let [r, g, b, a] = self.to_rgba();
        GDColor::from_rgba(r, g, b, a)
    }
}

impl TypeConvert for GDColor {
    type Target = Color;

    fn convert(self) -> Self::Target {
        Color::from_srgb([self.r, self.g, self.b, self.a])
    }
}

///TODO
impl TypeConvert for Collider {
    type Target = ();

    fn convert(self) -> Self::Target {
        todo!()
    }
}

///TODO
impl TypeConvert for Texture {
    type Target = ();

    fn convert(self) -> Self::Target {
        todo!()
    }
}

impl TypeConvert for Material {
    type Target = Gd<GDMaterial>;

    fn convert(self) -> Self::Target {
        let albedo = self.get_albedo();
        let godot_albedo = albedo.convert();
        let mut godot_material = StandardMaterial3D::new_gd();
        godot_material.set_albedo(godot_albedo);

        godot_material.upcast::<GDMaterial>()
    }
}

///TODO
impl TypeConvert for Curve {
    type Target = ();

    fn convert(self) -> Self::Target {
        todo!()
    }
}

///TODO
impl TypeConvert for Selection {
    type Target = ();

    fn convert(self) -> Self::Target {
        todo!()
    }
}

///TODO
impl TypeConvert for Collection {
    type Target = ();

    fn convert(self) -> Self::Target {
        todo!()
    }
}

///TODO
impl TypeConvert for Outline {
    type Target = ();

    fn convert(self) -> Self::Target {
        todo!()
    }
}

///TODO
impl TypeConvert for Instance {
    type Target = ();

    fn convert(self) -> Self::Target {
        todo!()
    }
}

impl TypeConvert for Gd<Curve3D> {
    type Target = CubicBezier;

    fn convert(self) -> Self::Target {
        let curve = self;
        let gd_point_count = curve.get_point_count();
        // Cast from i32 MUST always succeed since a Curve cant have negative points
        let mut point_vec = Vec::with_capacity(gd_point_count as usize);
        for index in 0..gd_point_count {
            let main_position = curve.get_point_position(index).convert();

            if index != 0 {
                point_vec.push(curve.get_point_in(index).convert() + main_position);
            }

            point_vec.push(main_position);

            if index != gd_point_count - 1 {
                point_vec.push(curve.get_point_out(index).convert() + main_position);
            }
        }
        CubicBezier::with_points(point_vec)
    }
}

/* pub trait DataTypeConversion {
    fn to_godot(&self) -> Variant;
}

impl DataTypeConversion for DataTypeValue {
    fn to_godot(&self) -> Variant {
        match self {
            DataTypeValue::Vec3(vec3) => Vector3::new(vec3.x, vec3.y, vec3.z).to_variant(),
            DataTypeValue::Float(float) => float.to_variant(),
            DataTypeValue::Mesh(mesh) => {
                let buffers = mesh.get_index_mesh_buffers();

                let mut mesh = ArrayMesh::new_gd();

                let mut surface = VariantArray::new();
                surface.resize(ArrayType::MAX.ord().try_into().unwrap(), &Variant::nil());
                surface.set(
                    ArrayType::VERTEX.ord().try_into().unwrap(),
                    &PackedVector3Array::from(unsafe {
                        mem::transmute::<Vec<Vec3>, Vec<Vector3>>(buffers.positions)
                    })
                    .to_variant(),
                );

                surface.set(
                    ArrayType::NORMAL.ord().try_into().unwrap(),
                    &PackedVector3Array::from(unsafe {
                        mem::transmute::<Vec<Vec3>, Vec<Vector3>>(buffers.normals)
                    })
                    .to_variant(),
                );

                //Since Godot uses clockwise winding order we reverse the iterator
                surface.set(
                    ArrayType::INDEX.ord().try_into().unwrap(),
                    &PackedInt32Array::from_iter(
                        buffers.indices.into_iter().map(|int| int as i32).rev(),
                    )
                    .to_variant(),
                );
                mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &surface);
                mesh.to_variant()
            }
            DataTypeValue::Selection(property) => {
                //godot_print!("{self:?}");
                property.get_literal().to_variant()
            }
            DataTypeValue::Int(int) => int.to_variant(),
            DataTypeValue::Bool(bool) => bool.to_variant(),
            DataTypeValue::Instance(_) => unimplemented!(),
            DataTypeValue::Curve(curve) => todo!(),
            DataTypeValue::Collection(collection) => todo!(),
            DataTypeValue::Material(material) => {
                let albedo = material.get_albedo();
                let [r, g, b, a] = albedo.to_rgba();
                let godot_albedo = GDColor::from_rgba(r, g, b, a);
                let mut godot_material = StandardMaterial3D::new_gd();
                println!("Albedo {albedo:?},GodotAlbedo: {godot_albedo:?}");
                godot_material.set_albedo(godot_albedo);

                godot_material.to_variant()
            }
            DataTypeValue::Collider(collider) => todo!(),
            DataTypeValue::CubicBezier(cubic_bezier) => todo!(),
            DataTypeValue::Transform(affine3_a) => {
                fn to_gd_vec3(vec: Vec3A) -> Vector3 {
                    Vector3 {
                        x: vec.x,
                        y: vec.y,
                        z: vec.z,
                    }
                }
                let basis = Basis::from_cols(
                    to_gd_vec3(affine3_a.matrix3.col(0)),
                    to_gd_vec3(affine3_a.matrix3.col(1)),
                    to_gd_vec3(affine3_a.matrix3.col(2)),
                );
                Transform3D::new(basis, to_gd_vec3(affine3_a.translation)).to_variant()
            }
            DataTypeValue::Color(color) => {
                let [r, g, b, a] = color.to_rgba();
                GDColor::from_rgba(r, g, b, a).to_variant()
            }
            DataTypeValue::Polygon2d(polygon2d) => todo!(),
            DataTypeValue::Texture(texture) => todo!(),
        }
    }
} */

/* impl DataTypeConversion for DataTypeRef<'_> {
    fn to_godot(&self) -> Variant {
        match self {
            DataTypeRef::Vec3(vec3) => Vector3::new(vec3.x, vec3.y, vec3.z).to_variant(),
            DataTypeRef::Float(float) => float.to_variant(),
            DataTypeRef::Mesh(mesh) => {
                let buffers = mesh.get_index_mesh_buffers();

                let mut mesh = ArrayMesh::new_gd();

                let mut surface = VariantArray::new();
                surface.resize(ArrayType::MAX.ord().try_into().unwrap(), &Variant::nil());
                surface.set(
                    ArrayType::VERTEX.ord().try_into().unwrap(),
                    &PackedVector3Array::from(unsafe {
                        mem::transmute::<Vec<Vec3>, Vec<Vector3>>(buffers.positions)
                    })
                    .to_variant(),
                );

                surface.set(
                    ArrayType::NORMAL.ord().try_into().unwrap(),
                    &PackedVector3Array::from(unsafe {
                        mem::transmute::<Vec<Vec3>, Vec<Vector3>>(buffers.normals)
                    })
                    .to_variant(),
                );

                //Since Godot uses clockwise winding order we reverse the iterator
                surface.set(
                    ArrayType::INDEX.ord().try_into().unwrap(),
                    &PackedInt32Array::from_iter(
                        buffers.indices.into_iter().map(|int| int as i32).rev(),
                    )
                    .to_variant(),
                );
                mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &surface);
                mesh.to_variant()
            }
            DataTypeRef::Selection(property) => {
                //godot_print!("{self:?}");
                property.get_literal().to_variant()
            }
            DataTypeRef::Int(int) => int.to_variant(),
            DataTypeRef::Bool(bool) => bool.to_variant(),
            DataTypeRef::Instance(_) => unimplemented!(),
            DataTypeRef::Curve(curve) => todo!(),
            DataTypeRef::Collection(collection) => todo!(),
            DataTypeRef::Material(material) => {
                let albedo = material.get_albedo();
                let [r, g, b, a] = albedo.to_rgba();
                let godot_albedo = GDColor::from_rgba(r, g, b, a);
                let mut godot_material = StandardMaterial3D::new_gd();
                println!("Albedo {albedo:?},GodotAlbedo: {godot_albedo:?}");
                godot_material.set_albedo(godot_albedo);

                godot_material.to_variant()
            }
            DataTypeRef::Collider(collider) => todo!(),
            DataTypeRef::CubicBezier(cubic_bezier) => todo!(),
            DataTypeRef::Transform(affine3_a) => {
                fn to_gd_vec3(vec: Vec3A) -> Vector3 {
                    Vector3 {
                        x: vec.x,
                        y: vec.y,
                        z: vec.z,
                    }
                }
                let basis = Basis::from_cols(
                    to_gd_vec3(affine3_a.matrix3.col(0)),
                    to_gd_vec3(affine3_a.matrix3.col(1)),
                    to_gd_vec3(affine3_a.matrix3.col(2)),
                );
                Transform3D::new(basis, to_gd_vec3(affine3_a.translation)).to_variant()
            }
            DataTypeRef::Color(color) => {
                let [r, g, b, a] = color.to_rgba();
                GDColor::from_rgba(r, g, b, a).to_variant()
            }
            DataTypeRef::Polygon2d(polygon2d) => todo!(),
            DataTypeRef::Texture(texture) => todo!(),
        }
    }
} */

/* pub trait GodotDataTypeToOneiroiDataType {
    fn to_oneiroi(self) -> DataTypeValue;
}

impl GodotDataTypeToOneiroiDataType for Variant {
    fn to_oneiroi(self) -> DataTypeValue {
        //godot_print!("{self}");
        match self.get_type() {
            VariantType::VECTOR3 => {
                let vec3 = self.to::<Vector3>();
                DataTypeValue::Vec3(Vec3::new(vec3.x, vec3.y, vec3.z))
            }
            VariantType::FLOAT => {
                let float = self.to::<f32>();
                DataTypeValue::Float(float)
            }
            VariantType::INT => {
                let int = self.to::<i64>();
                DataTypeValue::Int(int)
            }
            VariantType::BOOL => {
                let bool = self.to::<bool>();
                DataTypeValue::Bool(bool)
            }

            VariantType::COLOR => {
                let color = self.to::<GDColor>();
                DataTypeValue::Color(Box::new(Color::from_srgb([
                    color.r, color.g, color.b, color.a,
                ])))
            }

            VariantType::STRING => {
                let string = String::from(self.to::<GString>());

                DataTypeValue::Selection(Box::new(Selection::new(&string)))
            }
            VariantType::OBJECT => {
                let object = self.to::<Gd<Object>>();
                //Godots Curve3D uses Cubic Bezier curves internally
                if object.get_class() == "Curve3D".into() {
                    let curve = object.cast::<Curve3D>();
                    let gd_point_count = curve.get_point_count();
                    // Cast from i32 MUST always succeed since a Curve cant have negative points
                    let mut point_vec = Vec::with_capacity(gd_point_count as usize);
                    for index in 0..gd_point_count {
                        //SAFETY: Both are the glam::Vec3 internally
                        let main_position = unsafe {
                            std::mem::transmute::<Vector3, Vec3>(curve.get_point_position(index))
                        };

                        if index != 0 {
                            point_vec.push(
                                //SAFETY: Both are the glam::Vec3 internally
                                unsafe {
                                    std::mem::transmute::<Vector3, Vec3>(curve.get_point_in(index))
                                } + main_position,
                            );
                        }

                        point_vec.push(main_position);

                        if index != gd_point_count - 1 {
                            point_vec.push(
                                //SAFETY: Both are the glam::Vec3 internally
                                unsafe {
                                    std::mem::transmute::<Vector3, Vec3>(curve.get_point_out(index))
                                } + main_position,
                            );
                        }
                    }
                    DataTypeValue::CubicBezier(Box::new(CubicBezier::with_points(point_vec)))
                } else {
                    //TODO This is technically an error so probably should be handles as such
                    DataTypeValue::Float(0.0)
                }
            }

            //TODO This is technically an error so probably should be handles as such
            _ => DataTypeValue::Float(0.0),
        }
    }
}
 */

impl TypeConvert for Variant {
    type Target = OwnedDataType;

    fn convert(self) -> Self::Target {
        match self.get_type() {
            VariantType::FLOAT => {
                let float = self.to::<f32>();
                OwnedDataType::Float(float)
            }
            VariantType::INT => {
                let int = self.to::<i64>();
                OwnedDataType::Int(int)
            }
            VariantType::BOOL => {
                let bool = self.to::<bool>();
                OwnedDataType::Bool(bool)
            }

            VariantType::VECTOR3 => {
                let vec3 = self.to::<Vector3>();
                OwnedDataType::Vec3(vec3.convert())
            }

            VariantType::COLOR => {
                let color = self.to::<GDColor>();
                OwnedDataType::Color(Box::new(color.convert()))
            }

            VariantType::OBJECT => {
                let object = self.to::<Gd<Object>>();
                //Godots Curve3D uses Cubic Bezier curves internally
                if object.get_class() == "Curve3D".into() {
                    let curve = object.cast::<Curve3D>();
                    OwnedDataType::CubicBezier(Box::new(curve.convert()))
                } else {
                    //TODO This is technically an error so probably should be handles as such
                    OwnedDataType::Float(0.0)
                }
            }

            //TODO This is technically an error so probably should be handles as such
            _ => OwnedDataType::Float(0.0),
        }
    }
}

//FIXME: These Clones are not necessary at all. Optimize this
impl TypeConvert for TypeRef<'_> {
    type Target = Variant;

    fn convert(self) -> Self::Target {
        match self {
            TypeRef::Vec3(data) => data.convert().to_variant(),
            TypeRef::Float(data) => data.to_variant(),
            TypeRef::Bool(data) => data.to_variant(),
            TypeRef::Int(data) => data.to_variant(),
            TypeRef::Transform(data) => data.convert().to_variant(),
            TypeRef::Color(data) => data.convert().to_variant(),
            TypeRef::Mesh(data) => data.clone().convert().to_variant(),
            TypeRef::Collider(data) => data.clone().convert().to_variant(),
            TypeRef::Texture(data) => data.clone().convert().to_variant(),
            TypeRef::Material(data) => data.clone().convert().to_variant(),
            TypeRef::Curve(data) => data.clone().convert().to_variant(),
            TypeRef::CubicBezier(data) => data.clone().convert().to_variant(),
            TypeRef::Selection(data) => data.clone().convert().to_variant(),
            TypeRef::Collection(data) => data.clone().convert().to_variant(),
            TypeRef::Instance(data) => data.clone().convert().to_variant(),
            TypeRef::Outline(data) => data.clone().convert().to_variant(),
            _ => todo!(),
        }
    }
}

impl TypeConvert for OwnedDataType {
    type Target = Variant;

    fn convert(self) -> Self::Target {
        match self {
            OwnedDataType::Vec3(data) => data.convert().to_variant(),
            OwnedDataType::Float(data) => data.to_variant(),
            OwnedDataType::Bool(data) => data.to_variant(),
            OwnedDataType::Int(data) => data.to_variant(),
            OwnedDataType::Transform(data) => data.convert().to_variant(),
            OwnedDataType::Color(data) => data.convert().to_variant(),
            OwnedDataType::Mesh(data) => data.convert().to_variant(),
            OwnedDataType::Collider(data) => data.convert().to_variant(),
            OwnedDataType::Texture(data) => data.convert().to_variant(),
            OwnedDataType::Material(data) => data.convert().to_variant(),
            OwnedDataType::Curve(data) => data.convert().to_variant(),
            OwnedDataType::CubicBezier(data) => data.convert().to_variant(),
            OwnedDataType::Selection(data) => data.convert().to_variant(),
            OwnedDataType::Collection(data) => data.convert().to_variant(),
            OwnedDataType::Instance(data) => data.convert().to_variant(),
            OwnedDataType::Outline(data) => data.convert().to_variant(),
        }
    }
}
