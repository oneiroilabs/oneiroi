use std::marker::PhantomData;

use glam::Vec3;
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
mod parser;

use crate::{
    asset::Asset,
    data_types::{DataType, DataTypeInstance, DataTypeType, Property},
};

//TODO remove default
#[derive(Debug, Default, Clone)]
struct ParserCache<T: DataType> {
    origin: NodeIndex<u16>,
    target: NodeIndex<u16>,
    //info: Option<PropertyInfo>,
    value: T,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UninitializedOneiroiScript<T: DataType> {
    literal: String,
    _phantom: PhantomData<T>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OneiroiScript<T: DataType> {
    literal: String,
    #[serde(skip)]
    parsed_tree: ParserCache<T>,
}
impl<T: DataType> OneiroiScript<T> {
    pub fn get_default_value(&self) -> DataTypeInstance {
        //TODO actually get the thing
        match T::DATA_TYPE_TYPE {
            DataTypeType::Omni => todo!(),
            DataTypeType::Mesh => todo!(),
            DataTypeType::Collection => todo!(),
            DataTypeType::Instance => todo!(),
            DataTypeType::Curve => todo!(),
            DataTypeType::Collider => todo!(),
            DataTypeType::Vec3 => DataTypeInstance::Vec3(Property::Value(Vec3::new(1.0, 1.0, 1.0))),
            DataTypeType::Int => todo!(),
            DataTypeType::Float => DataTypeInstance::Float(Property::Value(2.0)),
            DataTypeType::Curve2d => todo!(),
            DataTypeType::Bool => todo!(),
            DataTypeType::Selection => todo!(),
        }
    }

    pub fn set_new_value(&mut self, value: T) {
        self.parsed_tree.value = value
    }

    pub fn get_value(&self) -> &T {
        &self.parsed_tree.value
    }

    pub fn get_string(&self) -> String {
        self.literal.clone()
    }

    //Vec3(1,@expose("wow",3),1)
    pub fn try_parse(
        input: &str,
        asset: &Asset,
        index: NodeIndex<u16>,
    ) -> Result<OneiroiScript<T>, OneiroiScriptParserError> {
        println!("{:?}", index);
        //TODO this needs to be validated through the asset
        Ok(OneiroiScript {
            literal: input.to_string(),
            parsed_tree: ParserCache {
                origin: index,
                target: NodeIndex::from(0),
                value: T::default(),
            },
        })
    }
}

#[derive(Debug)]
pub struct OneiroiScriptParserError {}

/* fn parse_input(input: &str) -> IResult<&str, &str> {
    let (rem, nah) = tag("abc")(input)?;
    delimited(tag("("), count(take(2), 3), tag(")"))(rem)?;
} */
