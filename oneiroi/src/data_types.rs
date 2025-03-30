use std::vec;

pub use glam::Vec3;

use serde::{Deserialize, Serialize};

use crate::mesh::{FaceHandle, OneiroiMesh};
use crate::script::OneiroiScript;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataTypeType {
    Omni,

    Selection,

    //Renderable  //Processable
    Mesh,
    Collection, //?
    Instance,
    Curve,
    Collider,

    //Primitives
    Vec3,
    Bool,
    Int,
    Float,
    Curve2d,
}

impl DataTypeType {
    pub fn get_color(&self) -> Vec3 {
        match self {
            DataTypeType::Omni => Vec3::new(134., 25., 143.).map(|f| f / 256.),
            DataTypeType::Mesh => Vec3::new(59.0, 230.0, 121.0).map(|f| f / 256.),
            DataTypeType::Collection => todo!(),
            DataTypeType::Instance => todo!(),
            DataTypeType::Curve => todo!(),
            DataTypeType::Collider => todo!(),
            DataTypeType::Vec3 => todo!(),
            DataTypeType::Int => todo!(),
            DataTypeType::Float => todo!(),
            DataTypeType::Curve2d => todo!(),
            DataTypeType::Bool => todo!(),
            DataTypeType::Selection => todo!(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DataTypeInstance {
    Vec3(Property<Vec3>),
    Float(Property<f32>),
    Bool(Property<bool>),
    Int(Property<i64>),
    Mesh(Property<OneiroiMesh>),
    Selection(Property<Selection>),
}

impl DataTypeInstance {
    pub fn get_type(&self) -> DataTypeType {
        match self {
            DataTypeInstance::Vec3(_) => DataTypeType::Vec3,
            DataTypeInstance::Float(_) => DataTypeType::Float,
            DataTypeInstance::Mesh(_) => DataTypeType::Mesh,
            DataTypeInstance::Selection(_) => DataTypeType::Selection,
            DataTypeInstance::Int(property) => DataTypeType::Int,
            DataTypeInstance::Bool(property) => DataTypeType::Bool,
        }
    }

    /* pub fn new<T: DataType>(value: Property<T>) -> Self {
        value.get_instance()
    } */

    /* fn get_instance<T: DataType>(value: Property<T>) -> Self {
        match T::get_type() {
            PropertyType::Mesh => todo!(),
            PropertyType::Collection => todo!(),
            PropertyType::Instance => todo!(),
            PropertyType::Curve => todo!(),
            PropertyType::Vec3 => Self::Vec3(value.ensure()),
            PropertyType::Int => todo!(),
            PropertyType::Float => todo!(),
            PropertyType::Curve2d => todo!(),
        }
    } */
    //TODO maybe there is a better way since evaluation should be moved outside
    pub fn inner_vec3(self) -> Result<Property<Vec3>, ()> {
        match self {
            DataTypeInstance::Vec3(property) => Ok(property),
            _ => Err(()),
        }
    }
    //TODO maybe there is a better way since evaluation should be moved outside
    pub fn inner_f32(self) -> Result<Property<f32>, ()> {
        match self {
            DataTypeInstance::Float(property) => Ok(property),
            _ => Err(()),
        }
    }
    //TODO maybe there is a better way since evaluation should be moved outside
    pub fn inner_int(self) -> Result<Property<i64>, ()> {
        match self {
            DataTypeInstance::Int(property) => Ok(property),
            _ => Err(()),
        }
    }

    //TODO maybe there is a better way since evaluation should be moved outside
    pub fn inner_bool(self) -> Result<Property<bool>, ()> {
        match self {
            DataTypeInstance::Bool(property) => Ok(property),
            _ => Err(()),
        }
    }

    pub fn inner_selection(self) -> Result<Property<Selection>, ()> {
        match self {
            DataTypeInstance::Selection(property) => Ok(property),
            _ => Err(()),
        }
    }

    pub fn inner_mesh(&self) -> Result<&Property<OneiroiMesh>, ()> {
        match self {
            DataTypeInstance::Mesh(property) => Ok(property),
            _ => Err(()),
        }
    }

    pub fn get_expression(&self) -> String {
        match self {
            DataTypeInstance::Vec3(property) => property.get_script(),
            DataTypeInstance::Bool(property) => property.get_script(),
            DataTypeInstance::Float(property) => property.get_script(),
            DataTypeInstance::Int(property) => property.get_script(),
            DataTypeInstance::Mesh(_) => unimplemented!(),
            DataTypeInstance::Selection(_) => unimplemented!(),
        }
    }

    /* pub fn get_value(&self) -> Self {
        match self {
            PropertyInstance::Vec3(property) => todo!(),
            PropertyInstance::Float(property) => todo!(),
        }
    } */

    pub fn get_property_meta(&self) -> PropertyMetadata {
        match self {
            DataTypeInstance::Vec3(property) => property.get_property_meta(),
            DataTypeInstance::Float(property) => property.get_property_meta(),
            DataTypeInstance::Bool(property) => property.get_property_meta(),
            DataTypeInstance::Mesh(oneiroi_mesh) => todo!(),
            DataTypeInstance::Selection(_) => todo!(),
            DataTypeInstance::Int(property) => property.get_property_meta(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Property<T: DataType> {
    Script(OneiroiScript<T>),
    Value(T),
}

impl<T: DataType> Property<T> {
    pub fn new(value: T) -> Property<T> {
        Property::Value(value)
    }

    /* pub fn get_instance(&self) -> DataTypeInstance {
        match T::DATA_TYPE_TYPE {
            DataTypeType::Omni => todo!(),
            DataTypeType::Mesh => todo!(),
            DataTypeType::Collection => todo!(),
            DataTypeType::Instance => todo!(),
            DataTypeType::Curve => todo!(),
            DataTypeType::Collider => todo!(),
            DataTypeType::Vec3 => DataTypeInstance::Vec3(self.clone()),
            DataTypeType::Int => todo!(),
            DataTypeType::Float => todo!(),
            DataTypeType::Curve2d => todo!(),
            DataTypeType::Selection => todo!(),
            DataTypeType::Bool => todo!(),
        }
    } */

    /* pub fn get_type(&self) -> PropertyType {
        T::get_type()
    } */
    //this function can be reworkes since two of 3 props get set one level higher which is not desired
    pub fn get_property_meta(&self) -> PropertyMetadata {
        match self {
            Property::Script(expression) => {
                let default_value = expression /* .as_ref().borrow() */
                    .get_default_value();
                PropertyMetadata {
                    name: None,
                    property_type: T::DATA_TYPE_TYPE,
                    default: Some(default_value),
                }
            }
            Property::Value(_) => PropertyMetadata {
                name: None,
                property_type: T::DATA_TYPE_TYPE,
                //We cant know what the default value of a built in property is
                // has to be set one level higher from the callee
                default: None,
            },
        }
    }

    /*  pub fn get_instance(&self) -> DataTypeInstance {
        PropertyInstance::get_instance(self)
    } */

    pub fn get_script(&self) -> String {
        //TODO this is most likely wrong since we have to parse it before and get the value this way
        // this produces a value so to speak -> and caches the result
        // have to see how to completly bake it to a primitive without overhead
        match self {
            Property::Script(script) => script.get_string(),
            Property::Value(value) => value.generate_script(),
        }
    }

    pub fn get_value(&self) -> &T {
        //TODO this is most likely wrong since we have to parse it before and get the value this way
        // this produces a value so to speak -> and caches the result
        // have to see how to completly bake it to a primitive without overhead
        match self {
            Property::Script(expression) => expression.get_value(),
            Property::Value(t) => t,
        }
    }

    pub fn consume(self) -> T {
        //TODO this is most likely wrong since we have to parse it before and get the value this way
        // this produces a value so to speak -> and caches the result
        // have to see how to completly bake it to a primitive without overhead
        match self {
            Property::Script(expression) => todo!(),
            Property::Value(t) => t,
        }
    }

    pub fn update(&mut self, new_value: Property<T>) {
        match self {
            Property::Script(current_expression) => match new_value {
                Property::Script(new_expression) => todo!(), //Should be valid expression just swap?
                Property::Value(new_val) => current_expression.set_new_value(new_val),
            },
            Property::Value(current_value) => match new_value {
                Property::Script(new_expression) => todo!(), //Should be valid expression just swap?
                Property::Value(new_val) => *current_value = new_val, //TODO this should get inserted correctly
            },
        }
    }

    //pub fn update(&mut self,value)
}

impl Property<Vec3> {
    /* pub fn new(value: Vec3) -> Property<Vec3> {
        Property::Edit(Expression {
            exression: String::new(),
            //instance: value,
            parsed_tree: ParserCache { input_value: value },
            }) /* )) */
    } */

    pub fn get_instance(&self) -> DataTypeInstance {
        DataTypeInstance::Vec3(self.clone())
    }

    /* pub fn update_raw(&mut self, value: Vec3) {
    match self {
        Property::Edit(expression) => expression.set_value(value),
        Property::Instance(inst) => *inst = value,
        }
    } */

    /* #[cfg(not(feature = "only_runtime"))]
    pub fn update_expression(&mut self, asset: &mut Asset, value: String) {
        /* match self {
            Property::Edit(expression) => expression.set_value(value),
            Property::Instance(inst) => *inst = value,
            } */
    } */

    /* pub fn update(&mut self, value: Property<Vec3>) {
    match self {
        Property::Edit(expression) => match value {
            Property::Edit(new_expression) => todo!(), //Should be valid expression just swap?
            Property::Instance(new_val) => expression.set_value(new_val), //TODO this should get inserted correctly
            },
            Property::Instance(_) => todo!(),
            }
        } */
}

impl Property<f32> {
    /* pub fn new(value: f32) -> Property<f32> {
    Property::Edit(Expression {
        exression: String::new(),
        //instance: value,
        parsed_tree: ParserCache { input_value: value },
        }) /* )) */
        } */

    pub fn get_instance(&self) -> DataTypeInstance {
        DataTypeInstance::Float(self.clone())
    }
}

impl Property<i64> {
    pub fn get_instance(&self) -> DataTypeInstance {
        DataTypeInstance::Int(self.clone())
    }
}

impl Property<Selection> {
    pub fn get_instance(&self) -> DataTypeInstance {
        DataTypeInstance::Selection(self.clone())
    }
}

impl Property<bool> {
    pub fn get_instance(&self) -> DataTypeInstance {
        DataTypeInstance::Bool(self.clone())
    }
}

pub trait DataType: Clone + Default {
    //fn get_value_string() -> String;
    //type ParsingType: DataType;

    const DATA_TYPE_TYPE: DataTypeType;

    fn generate_script(&self) -> String {
        unimplemented!()
    }

    //fn get_type() -> PropertyType;

    /* fn ensure(self) -> Property<Self::CastingType> {
    self
    } */

    //TODO need to implement defualt string representation
}

pub struct Omni;

impl DataType for Vec3 {
    /* fn get_type() -> PropertyType {
    PropertyType::Vec3
    } */

    fn generate_script(&self) -> String {
        "Vec3(".to_owned()
            + &self.x.to_string()
            + ","
            + &self.y.to_string()
            + ","
            + &self.z.to_string()
            + ")"
    }

    //type ParsingType = Vec3;

    const DATA_TYPE_TYPE: DataTypeType = DataTypeType::Vec3;
}

impl DataType for f32 {
    /*  fn get_type() -> PropertyType {
        PropertyType::Float
    } */

    // type ParsingType = f32;
    fn generate_script(&self) -> String {
        self.to_string()
    }

    const DATA_TYPE_TYPE: DataTypeType = DataTypeType::Float;
}

impl DataType for bool {
    /*  fn get_type() -> PropertyType {
    PropertyType::Float
    } */

    // type ParsingType = f32;
    fn generate_script(&self) -> String {
        self.to_string()
    }

    const DATA_TYPE_TYPE: DataTypeType = DataTypeType::Bool;
}

impl DataType for OneiroiMesh {
    const DATA_TYPE_TYPE: DataTypeType = DataTypeType::Mesh;
}

impl DataType for i64 {
    fn generate_script(&self) -> String {
        self.to_string()
    }
    const DATA_TYPE_TYPE: DataTypeType = DataTypeType::Int;
}

#[derive(Debug, Clone)]
pub struct PropertyMetadata {
    name: Option<String>,

    property_type: DataTypeType,
    // This needs to be in Value mode
    default: Option<DataTypeInstance>,
}

impl PropertyMetadata {
    /// # Panics if name not set already
    pub fn name(&self) -> &str {
        self.name.as_ref().unwrap()
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = Some(name.to_owned())
    }

    pub fn set_default(&mut self, default_val: DataTypeInstance) {
        self.default = Some(default_val)
    }

    pub fn get_type(&self) -> &DataTypeType {
        &self.property_type
    }

    pub fn get_default(&self) -> &DataTypeInstance {
        self.default
            .as_ref()
            .expect("Default has to be set somewhere before accessing it")
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Selection {
    literal: String,
}

impl DataType for Selection {
    const DATA_TYPE_TYPE: DataTypeType = DataTypeType::Selection;
}

impl Selection {
    pub fn new(selection: &str) -> Self {
        Selection {
            literal: selection.into(),
        }
    }

    pub fn get_literal(&self) -> &str {
        &self.literal
    }

    //TODO this should be a slice
    pub fn try_get(&self) -> Result<Vec<FaceHandle>, ()> {
        let parsed = self.literal.parse::<usize>();
        println!("{parsed:?}");
        Ok(vec![FaceHandle::new(parsed.unwrap())])
    }
}
