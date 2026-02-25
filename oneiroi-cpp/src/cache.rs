use std::collections::HashMap;

use super::DataTypeValue;
use oneiroi::{
    asset::instance::AssetInstance,
    nodes::{ContextProvider, SocketInterface},
    type_system::Reference,
    type_system::{OwnedDataType as InternalDataTypeValue, TypeRef as InternalDataTypeRef},
};

#[derive(Debug)]
pub struct AssetCache {
    converted_inputs: HashMap<Reference, InternalDataTypeValue>,
    input_sockets: Box<[Reference]>,
}

impl AssetCache {
    pub fn init(instance: &AssetInstance) -> Self {
        let mut cached_refs = Box::new_uninit_slice(instance.get_input_sockets().len());
        for dings in 0..instance.get_input_sockets().len() {
            cached_refs[dings].write(Reference::Standard {
                node: 0.into(),
                socket: dings as u8,
            });
        }
        Self {
            converted_inputs: HashMap::default(),
            input_sockets: unsafe { cached_refs.assume_init() },
        }
    }

    pub fn get_input_refs(&self) -> &[Reference] {
        &self.input_sockets
    }

    pub fn set_input_index(&mut self, idx: u8, value: &DataTypeValue) {
        self.converted_inputs.insert(
            Reference::Standard {
                node: 0.into(),
                socket: idx,
            },
            value.0.clone(),
        );
    }
}

impl ContextProvider for AssetCache {
    fn get_reference(&self, index: Reference) -> InternalDataTypeRef {
        (&self.converted_inputs[&index]).into()
    }
}
