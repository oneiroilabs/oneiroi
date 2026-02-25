use glam::{Affine3A, Vec3};
use petgraph::visit::Data;
use serde::{Deserialize, Serialize};

use crate::{
    nodes::ContextProvider,
    type_system::{
        OwnedDataType, Reference, TypeRef,
        data_types::{DataType, DataTypeConfiguration, DataTypeKind, Mesh, Selection},
    },
};

pub mod script;

/// The Type-Safe way to represent a Property inside a Node.
/// Can be configured to have restrictions and  holds
/// a Value which is either a Literal or an External Reference.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(from = "PropertyValue<T>", into = "PropertyValue<T>")]
pub(crate) struct Property<T: DataType> {
    #[serde(skip)]
    config: Option<T::ConfigurationOptions>,
    value: PropertyValue<T>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub(crate) enum PropertyValue<T: DataType> {
    External(Reference),
    Literal(T),
}

/// Implementation is for Serde Deserialization
impl<T: DataType> From<PropertyValue<T>> for Property<T> {
    fn from(value: PropertyValue<T>) -> Self {
        Self {
            config: None,
            value,
        }
    }
}

/// Implementation is for Serde Serialization
impl<T: DataType> From<Property<T>> for PropertyValue<T> {
    fn from(value: Property<T>) -> Self {
        value.value
    }
}

impl<T: DataType> Property<T> {
    pub fn new(value: T) -> Property<T> {
        Self {
            config: None,
            value: PropertyValue::Literal(value),
        }
    }

    pub fn with_config(value: T, config: T::ConfigurationOptions) -> Property<T> {
        Self {
            config: Some(config),
            value: PropertyValue::Literal(value),
        }
    }

    pub const fn get_type(&self) -> DataTypeKind {
        T::DATA_TYPE_TYPE
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
    /* pub const fn get_property_meta(&self) -> PropertyMetadata {
        match &self.value {
            /* Property::Script(expression) => {
                let default_value = expression /* .as_ref().borrow() */
                    .get_default_value();
                PropertyMetadata {
                    name: None,
                    r#type: T::DATA_TYPE_TYPE,
                    default: Some(default_value),
                    //TODO
                    configuration: DataTypeConfiguration::None,
                    documentation: String::new(),
                }
            } */
            //TODO
            //Property::External(reference) => context.get_reference(*reference),
            PropertyValue::External(..) => panic!(),
            PropertyValue::Literal(_) => PropertyMetadata {
                // Name and Default value of the Property come from a higher universe.
                // Initialization needs to be defered till then.
                name: None,
                default: None,
                r#type: T::DATA_TYPE_TYPE,
                configuration: DataTypeConfiguration::None,
                documentation: String::new(),
            },
        }
    } */

    pub(crate) const fn get_configuration(&self) -> Option<&T::ConfigurationOptions> {
        self.config.as_ref()
    }

    /*  pub fn get_instance(&self) -> DataTypeInstance {
        PropertyInstance::get_instance(self)
    } */

    /* pub fn get_script(&self) -> String {
        //TODO this is most likely wrong since we have to parse it before and get the value this way
        // this produces a value so to speak -> and caches the result
        // have to see how to completly bake it to a primitive without overhead
        match self {
            Property::Script(script) => script.get_string(),
            Property::Literal(value) => value.generate_script(),
        }
    } */

    /// Gets the Value the Property as long as its Literal.
    /// This function panics when the Property is set to external
    /// and it is up to the caller to ensure the Property is a literal.
    /// Most prominently used in nodes to set the default Value.
    /// TODO this function could be marked unsafe.
    pub(crate) fn get_literal_value(&self) -> &T {
        match &self.value {
            PropertyValue::Literal(t) => t,
            PropertyValue::External(_) => unreachable!(),
        }
    }

    pub(crate) fn set_external(&mut self, reference: Reference) {
        self.value = PropertyValue::External(reference);
    }

    pub(crate) fn get_value<'a>(&'a self, context: &'a impl ContextProvider) -> &'a T {
        //TODO this is most likely wrong since we have to parse it before and get the value this way
        // this produces a value so to speak -> and caches the result
        // have to see how to completly bake it to a primitive without overhead
        match &self.value {
            PropertyValue::External(reference) => {
                T::get_type_ref(context.get_reference(*reference))
            }
            PropertyValue::Literal(t) => t,
        }
    }

    /* pub fn consume(self) -> T {
        //TODO this is most likely wrong since we have to parse it before and get the value this way
        // this produces a value so to speak -> and caches the result
        // have to see how to completly bake it to a primitive without overhead
        match self {
            Property::Script(expression) => todo!(),
            Property::Literal(t) => t,
        }
    } */

    /* pub fn update(&mut self, new_value: Property<T>) {
        match self {
            Property::Script(current_expression) => match new_value {
                Property::Script(new_expression) => todo!(), //Should be valid expression just swap?
                Property::Literal(new_val) => current_expression.set_new_value(new_val),
            },
            Property::Literal(current_value) => match new_value {
                Property::Script(new_expression) => todo!(), //Should be valid expression just swap?
                Property::Literal(new_val) => *current_value = new_val, //TODO this should get inserted correctly
            },
        }
    } */

    pub fn set_value(&mut self, new_value: T) {
        match &mut self.value {
            PropertyValue::Literal(current_value) => *current_value = new_value,
            PropertyValue::External(..) => unreachable!(),
        }
    }

    //pub fn update(&mut self,value)
}

/* impl Property<Vec3> {
    /* pub fn new(value: Vec3) -> Property<Vec3> {
        Property::Edit(Expression {
            exression: String::new(),
            //instance: value,
            parsed_tree: ParserCache { input_value: value },
            }) /* )) */
    } */

    pub fn get_instance(&self) -> PropertyInstance {
        PropertyInstance::Vec3(self.clone())
    }

    pub fn get_datatype_value(&self) -> DataTypeValue {
        DataTypeValue::Vec3(self.get_value().clone())
    }

    /* pub fn update_raw(&mut self, value: Vec3) {
    match self {
        Property::Edit(expression) => expression.set_value(value),
        Property::Instance(inst) => *inst = value,
        }
    } */

    /*#[cfg(feature = "editor")]
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

    pub fn get_instance(&self) -> PropertyInstance {
        PropertyInstance::Float(self.clone())
    }
    pub fn get_datatype_value(&self) -> DataTypeValue {
        DataTypeValue::Float(self.get_value().clone())
    }
}

impl Property<i64> {
    pub fn get_instance(&self) -> PropertyInstance {
        PropertyInstance::Int(self.clone())
    }
    pub fn get_datatype_value(&self) -> DataTypeValue {
        DataTypeValue::Int(self.get_value().clone())
    }
}

impl Property<Selection> {
    pub fn get_instance(&self) -> PropertyInstance {
        PropertyInstance::Selection(self.clone())
    }
    pub fn get_datatype_value(&self) -> DataTypeValue {
        DataTypeValue::Selection(Box::new(self.get_value().clone()))
    }
}

impl Property<Affine3A> {
    pub fn get_datatype_value(&self) -> DataTypeValue {
        DataTypeValue::Transform(Box::new(*self.get_value()))
    }
}

impl Property<bool> {
    pub fn get_instance(&self) -> PropertyInstance {
        PropertyInstance::Bool(self.clone())
    }
    pub fn get_datatype_value(&self) -> DataTypeValue {
        DataTypeValue::Bool(self.get_value().clone())
    }
} */

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PropertyInstance {
    Vec3(Property<Vec3>),
    Float(Property<f32>),
    Bool(Property<bool>),
    Int(Property<i64>),
    Mesh(Property<Mesh>),
    Selection(Property<Selection>),
}

impl PropertyInstance {
    pub fn get_type(&self) -> DataTypeKind {
        match self {
            PropertyInstance::Vec3(_) => DataTypeKind::Vec3,
            PropertyInstance::Float(_) => DataTypeKind::Float,
            PropertyInstance::Mesh(_) => DataTypeKind::Mesh,
            PropertyInstance::Selection(_) => DataTypeKind::Selection,
            PropertyInstance::Int(property) => DataTypeKind::Int,
            PropertyInstance::Bool(property) => DataTypeKind::Bool,
        }
    }

    pub(crate) fn new(meta: &PropertyMetadata) -> Self {
        match meta.r#type {
            DataTypeKind::Omni => todo!(),
            DataTypeKind::Selection => todo!(),
            DataTypeKind::Mesh => todo!(),
            DataTypeKind::Collider => todo!(),
            DataTypeKind::Curve => todo!(),
            DataTypeKind::CubicBezier => todo!(),
            DataTypeKind::Instance => todo!(),
            DataTypeKind::Material => todo!(),
            DataTypeKind::Collection => todo!(),
            DataTypeKind::Vec3 => {
                Self::Vec3(Property::new(meta.default.clone().dispatch().unwrap()))
            }
            DataTypeKind::Bool => todo!(),
            DataTypeKind::Int => todo!(),
            DataTypeKind::Float => {
                Self::Float(Property::new(meta.default.clone().dispatch().unwrap()))
            }
            DataTypeKind::Color => todo!(),
            DataTypeKind::Transform => todo!(),
            DataTypeKind::Outline => todo!(),
            DataTypeKind::Texture => todo!(),
        }
    }

    pub(crate) fn set_value(&mut self, value: OwnedDataType) {
        match self {
            PropertyInstance::Vec3(property) => property.set_value(value.dispatch().unwrap()),
            PropertyInstance::Float(property) => property.set_value(value.dispatch().unwrap()),
            PropertyInstance::Bool(property) => todo!(),
            PropertyInstance::Int(property) => todo!(),
            PropertyInstance::Mesh(property) => todo!(),
            PropertyInstance::Selection(property) => todo!(),
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
    /* pub fn inner_vec3(self) -> Result<Property<Vec3>, ()> {
        match self {
            PropertyInstance::Vec3(property) => Ok(property),
            _ => Err(()),
        }
    }
    //TODO maybe there is a better way since evaluation should be moved outside
    pub fn inner_f32(self) -> Result<Property<f32>, ()> {
        match self {
            PropertyInstance::Float(property) => Ok(property),
            _ => Err(()),
        }
    }
    //TODO maybe there is a better way since evaluation should be moved outside
    pub fn inner_int(self) -> Result<Property<i64>, ()> {
        match self {
            PropertyInstance::Int(property) => Ok(property),
            _ => Err(()),
        }
    }

    //TODO maybe there is a better way since evaluation should be moved outside
    pub fn inner_bool(self) -> Result<Property<bool>, ()> {
        match self {
            PropertyInstance::Bool(property) => Ok(property),
            _ => Err(()),
        }
    }

    pub fn inner_selection(self) -> Result<Property<Selection>, ()> {
        match self {
            PropertyInstance::Selection(property) => Ok(property),
            _ => Err(()),
        }
    }

    pub fn inner_mesh(&self) -> Result<&Property<Mesh>, ()> {
        match self {
            PropertyInstance::Mesh(property) => Ok(property),
            _ => Err(()),
        }
    } */

    /* pub fn get_expression(&self) -> String {
        match self {
            PropertyInstance::Vec3(property) => property.get_script(),
            PropertyInstance::Bool(property) => property.get_script(),
            PropertyInstance::Float(property) => property.get_script(),
            PropertyInstance::Int(property) => property.get_script(),
            PropertyInstance::Mesh(_) => unimplemented!(),
            PropertyInstance::Selection(_) => unimplemented!(),
        }
    } */
    /*  pub(crate) fn get_datatype_value(&self) -> DataTypeValue {
        match self {
            PropertyInstance::Vec3(property) => property.get_literal_value().to_data_type_ref(),
            PropertyInstance::Float(property) => property.get_literal_value().to_data_type_ref(),
            PropertyInstance::Bool(property) => property.get_literal_value().to_data_type_ref(),
            PropertyInstance::Int(property) => property.get_literal_value().to_data_type_ref(),
            PropertyInstance::Mesh(property) => todo!(),
            PropertyInstance::Selection(property) => property.get_literal_value().to_data_type_ref(),
        }
    } */

    /* pub fn get_value(&self) -> Self {
        match self {
            PropertyInstance::Vec3(property) => todo!(),
            PropertyInstance::Float(property) => todo!(),
        }
    } */

    /* pub fn get_property_meta(&self) -> PropertyMetadata {
        match self {
            PropertyInstance::Vec3(property) => property.get_property_meta(),
            PropertyInstance::Float(property) => property.get_property_meta(),
            PropertyInstance::Bool(property) => property.get_property_meta(),
            PropertyInstance::Mesh(oneiroi_mesh) => todo!(),
            PropertyInstance::Selection(_) => todo!(),
            PropertyInstance::Int(property) => property.get_property_meta(),
        }
    } */
}

impl<'a> From<&'a PropertyInstance> for TypeRef<'a> {
    fn from(value: &'a PropertyInstance) -> Self {
        match value {
            PropertyInstance::Vec3(property) => {
                TypeRef::Vec3(unsafe { property.get_literal_value() })
            }
            PropertyInstance::Float(property) => {
                TypeRef::Float(unsafe { property.get_literal_value() })
            }
            PropertyInstance::Bool(property) => todo!(),
            PropertyInstance::Int(property) => todo!(),
            PropertyInstance::Mesh(property) => todo!(),
            PropertyInstance::Selection(property) => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PropertyMetadata {
    pub(crate) name: String,
    pub(crate) default: OwnedDataType,
    pub(crate) r#type: DataTypeKind,

    pub(crate) configuration: Option<DataTypeConfiguration>,

    //TODO decide on type
    pub(crate) documentation: String,
}

impl PropertyMetadata {
    /// # Panics if name not set already
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn get_type(&self) -> DataTypeKind {
        self.r#type
    }

    pub fn get_default(&self) -> TypeRef {
        (&self.default).into()
    }
}
