use godot::{
    builtin::{GString, Variant, VariantType, Vector3},
    global::godot_print,
    meta::ToGodot,
};
use oneiroi::data_types::{DataTypeInstance, DataTypeType, Property, Selection, Vec3};

// This is technically not supposed to be here maybe add another conversion trait in the editor module and splite these two functions
//or just make 2 impls and conditional compile

pub trait OneiroiToGodot {
    //TODO this method name most liely sucks
    fn variant_type(&self) -> VariantType;
}

impl OneiroiToGodot for DataTypeType {
    fn variant_type(&self) -> VariantType {
        match self {
            DataTypeType::Omni => unimplemented!(),
            DataTypeType::Mesh => unimplemented!(),
            DataTypeType::Collection => todo!(),
            DataTypeType::Instance => todo!(),
            DataTypeType::Curve => todo!(),
            DataTypeType::Vec3 => VariantType::VECTOR3,
            DataTypeType::Int => VariantType::INT,
            DataTypeType::Float => VariantType::FLOAT,
            DataTypeType::Curve2d => todo!(),
            DataTypeType::Bool => VariantType::BOOL,
            DataTypeType::Collider => todo!(),
            DataTypeType::Selection => VariantType::STRING,
        }
    }
}

pub trait DataTypeConversion {
    fn to_godot(&self) -> Variant;
}

impl DataTypeConversion for DataTypeInstance {
    fn to_godot(&self) -> Variant {
        match self {
            DataTypeInstance::Vec3(vec3) => match vec3 {
                Property::Script(_) => {
                    let value = vec3.get_value();
                    Vector3::new(value.x, value.y, value.z).to_variant()
                }
                Property::Value(value) => Vector3::new(value.x, value.y, value.z).to_variant(),
            },
            DataTypeInstance::Float(float) => match float {
                Property::Script(_) => {
                    //OneiroiVector3::new(expression.clone()).to_variant()
                    //TODO
                    todo!()
                }
                Property::Value(value) => value.to_variant(),
            },
            DataTypeInstance::Mesh(_) => todo!(),
            DataTypeInstance::Selection(property) => {
                //godot_print!("{self:?}");
                property.get_value().get_literal().to_variant()
            }
            DataTypeInstance::Int(int) => match int {
                Property::Script(oneiroi_script) => todo!(),
                Property::Value(value) => value.to_variant(),
            },
            DataTypeInstance::Bool(bool) => match bool {
                Property::Script(oneiroi_script) => todo!(),
                Property::Value(value) => value.to_variant(),
            },
        }
    }
}
pub trait GodotDataTypeToOneiroiDataType {
    fn to_oneiroi(self) -> DataTypeInstance;
}

impl GodotDataTypeToOneiroiDataType for Variant {
    fn to_oneiroi(self) -> DataTypeInstance {
        //godot_print!("{self}");
        match self.get_type() {
            VariantType::VECTOR3 => {
                let vec3 = self.to::<Vector3>();
                DataTypeInstance::Vec3(Property::Value(Vec3::new(vec3.x, vec3.y, vec3.z)))
            }
            VariantType::FLOAT => {
                let float = self.to::<f32>();
                DataTypeInstance::Float(Property::Value(float))
            }
            VariantType::INT => {
                let int = self.to::<i64>();
                DataTypeInstance::Int(Property::Value(int))
            }
            VariantType::BOOL => {
                let bool = self.to::<bool>();
                DataTypeInstance::Bool(Property::Value(bool))
            }
            VariantType::STRING => {
                let string = String::from(self.to::<GString>());

                DataTypeInstance::Selection(Property::Value(Selection::new(&string)))
            }
            //TODO This is technically an error so probably should be handles as such
            _ => DataTypeInstance::Float(Property::Value(0.0)),
        }
    }
}
