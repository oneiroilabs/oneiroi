use serde::{Deserialize, Serialize};

use crate::{
    nodes::{
        ContextProvider, Node, PropertyInterface, PropertyNotFound, SetPropertyError,
        SocketInterface, StaticNodeMetadata,
    },
    property::{Property, PropertyMetadata},
    type_system::{
        OwnedDataType, Reference, TypeRef,
        data_types::{Collection, CubicBezier, DataType, DataTypeKind, TypeDescriptor},
    },
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SampleCurveV1 {
    number: Property<i64>,
    distance: Property<f32>,
    equidistant: Property<bool>,
}
impl Default for SampleCurveV1 {
    fn default() -> Self {
        Self {
            number: Property::new(5),
            distance: Property::new(1.),
            equidistant: Property::new(true),
        }
    }
}

impl PropertyInterface for SampleCurveV1 {
    fn try_set_property(
        &mut self,
        property: &str,
        value: OwnedDataType,
    ) -> Result<(), SetPropertyError> {
        match property {
            "number" => {
                self.number
                    .set_value(value.dispatch().expect("TODO error handling"));
                Ok(())
            }
            "distance" => {
                self.distance
                    .set_value(value.dispatch().expect("TODO error handling"));
                Ok(())
            }
            "equidistant" => {
                self.equidistant
                    .set_value(value.dispatch().expect("TODO error handling"));
                Ok(())
            }
            _ => {
                println!("called set_prop with {:?}", property);
                Err(SetPropertyError::NotFound)
            }
        }
    }
    fn try_get_property(&self, property: &str) -> Result<TypeRef, PropertyNotFound> {
        match property {
            "number" => self.try_get_property_index(0),
            "distance" => self.try_get_property_index(1),
            "equidistant" => self.try_get_property_index(2),
            _ => {
                //println!("called get_prop with {:?}", property);
                Err(PropertyNotFound)
            }
        }
    }

    fn get_properties(&self) -> Box<[PropertyMetadata]> {
        println!("Get property called on Box");

        //TODO this can be most likely optimized
        let default = Self::default();

        let mut info = PropertyMetadata {
            name: "number".into(),
            r#type: default.number.get_type(),
            default: default.number.get_literal_value().to_data_type_value(),
            configuration: None,
            //configuration: default.size.get_configuration().to_owned(),
            documentation: "".into(),
        };
        //const WHAT: Box<[PropertyMetadata]> = Box::new([info]);

        let mut info2 = PropertyMetadata {
            name: "distance".into(),
            r#type: default.distance.get_type(),
            default: default.distance.get_literal_value().to_data_type_value(),
            configuration: None,
            //configuration: default.size.get_configuration().to_owned(),
            documentation: "".into(),
        };

        let mut info3 = PropertyMetadata {
            name: "equidistant".into(),
            r#type: default.equidistant.get_type(),
            default: default.equidistant.get_literal_value().to_data_type_value(),
            configuration: None,
            //configuration: default.size.get_configuration().to_owned(),
            documentation: "".into(),
        };

        vec![info, info2, info3].into_boxed_slice()
    }

    fn try_set_property_index(
        &mut self,
        index: u8,
        value: OwnedDataType,
    ) -> Result<(), SetPropertyError> {
        todo!()
    }

    fn try_get_property_index(&self, index: u8) -> Result<TypeRef, PropertyNotFound> {
        match index {
            0 => Ok(self.number.get_literal_value().to_data_type_ref()),
            1 => Ok(self.distance.get_literal_value().to_data_type_ref()),
            2 => Ok(self.equidistant.get_literal_value().to_data_type_ref()),
            _ => {
                //println!("called get_prop with {:?}", property);
                Err(PropertyNotFound)
            }
        }
    }

    fn try_get_property_metadata(&self, index: u8) -> PropertyMetadata {
        self.get_properties()[index as usize].clone()
    }

    fn set_property_external(
        &mut self,
        index: u8,
        reference: Reference,
    ) -> Result<(), SetPropertyError> {
        match index {
            0 => {
                self.number.set_external(reference);
                Ok(())
            }
            1 => {
                self.distance.set_external(reference);
                Ok(())
            }
            2 => Ok(self.equidistant.set_external(reference)),
            _ => Err(SetPropertyError::WrongIndex),
        }
    }

    /* fn try_get_property_script(&self, property: &str) -> Result<String, PropertyNotFound> {
        let string = match property {
            "number" => match &self.number {
                Property::Script(oneiroi_script) => oneiroi_script.get_string(),
                Property::Value(v) => v.generate_script(),
            },

            _ => return Err(PropertyNotFound),
        };
        Ok(string)
    } */

    /* fn try_get_property_metadata(&self, property: &str) -> Result<PropertyMetadata, ()> {
        todo!()
    } */
}

impl Node for SampleCurveV1 {
    fn compute(
        &self,
        input_sockets: Option<&[Reference]>,
        context: &impl ContextProvider,
    ) -> Box<[Option<OwnedDataType>]> {
        let curve = context.get_reference(input_sockets.unwrap()[0]);
        let curve: &CubicBezier = curve.dispatch_ref().unwrap();

        let mut collection = Collection::new(DataTypeKind::Transform);

        //TODO differentiate what sampling to use but no time to implement enums

        //let size = *self.number.get_value();

        //let points = curve.sample(size);

        /* for point in points {
            collection.push(DataTypeValue::Vec3(point));
        } */

        let distance = *self.distance.get_value(context);
        println!("Sample Curve distance is: {distance}");
        let transforms = curve.sample_at_fixed_distance(distance);

        for transform in transforms {
            collection.push(OwnedDataType::Transform(Box::new(transform)));
        }

        Box::new([Some(OwnedDataType::new(collection))])
    }

    /* fn get_sockets(
        &self,
    ) -> (
        Vec<crate::data_types::DataTypeType>,
        Vec<crate::data_types::DataTypeType>,
    ) {
        (Vec::new(), vec![DataTypeType::Mesh])
    } */
    fn node_metadata(&self) -> StaticNodeMetadata {
        StaticNodeMetadata { color: "#15803d" }
    }
}
impl SocketInterface for SampleCurveV1 {
    fn get_input_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::new([TypeDescriptor {
            r#type: DataTypeKind::CubicBezier,
            mutable: false,
        }])
    }

    fn get_output_sockets(&self) -> Box<[TypeDescriptor]> {
        Box::new([TypeDescriptor {
            r#type: DataTypeKind::Collection,
            mutable: true,
        }])
    }
}
