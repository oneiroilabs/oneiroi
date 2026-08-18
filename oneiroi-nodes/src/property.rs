use oneiroi_core::types::DataType;
use serde::{Deserialize, Serialize};

/* use crate::{
    type_system::{
        OwnedDataType, Reference, TypeRef,
        data_types::{DataType, DataTypeConfiguration, DataTypeKind, Mesh, Selection},
    },
}; */

mod script;

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
enum PropertyValue<T: DataType> {
    Value(T),
    Script(String),
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
            value: PropertyValue::Value(value),
        }
    }

    pub fn with_config(value: T, config: T::ConfigurationOptions) -> Property<T> {
        Self {
            config: Some(config),
            value: PropertyValue::Value(value),
        }
    }

    pub(crate) const fn get_configuration(&self) -> Option<&T::ConfigurationOptions> {
        self.config.as_ref()
    }
}
