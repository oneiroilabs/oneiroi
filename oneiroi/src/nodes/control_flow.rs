mod socket_input {
    use serde::Deserialize;
    use serde::Serialize;

    use crate::nodes::ContextProvider;
    use crate::nodes::Node;
    use crate::nodes::PropertyInterface;
    use crate::nodes::PropertyNotFound;
    use crate::nodes::SetPropertyError;
    use crate::nodes::SocketInterface;
    use crate::nodes::StaticNodeMetadata;
    use crate::property::Property;
    use crate::property::PropertyMetadata;
    use crate::type_system::OwnedDataType;
    use crate::type_system::Reference;
    use crate::type_system::TypeRef;
    use crate::type_system::data_types::DataTypeKind;
    use crate::type_system::data_types::TypeDescriptor;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct SocketInputV1 {
        runtime: Property<bool>,
    }

    impl Default for SocketInputV1 {
        fn default() -> Self {
            Self {
                runtime: Property::new(false),
            }
        }
    }

    impl PropertyInterface for SocketInputV1 {
        fn try_set_property(
            &mut self,
            property_name: &str,
            value: OwnedDataType,
        ) -> Result<(), SetPropertyError> {
            //TODO
            Ok(())
        }

        fn try_set_property_index(
            &mut self,
            index: u8,
            value: OwnedDataType,
        ) -> Result<(), SetPropertyError> {
            todo!()
        }
        fn try_get_property_index(&self, index: u8) -> Result<TypeRef, PropertyNotFound> {
            todo!()
        }

        fn try_get_property(&self, property: &str) -> Result<TypeRef, PropertyNotFound> {
            //TODO
            Err(PropertyNotFound)
        }

        fn get_properties(&self) -> Box<[PropertyMetadata]> {
            //TODO
            Box::new([])
        }

        fn set_property_external(
            &mut self,
            index: u8,
            reference: Reference,
        ) -> Result<(), SetPropertyError> {
            todo!()
        }

        /* fn try_get_property_script(&self, property: &str) -> Result<String, PropertyNotFound> {
            todo!()
        } */

        /* fn try_get_property_metadata(
            &self,
            property: &str,
        ) -> Result<crate::data_types::PropertyMetadata, ()> {
            todo!()
        } */
    }

    impl Node for SocketInputV1 {
        //type InputSockets = Omni;

        //type OutputSockets = ();

        /* fn compute(
            &self,
            input_sockets: /* impl TryInto< */ Self::InputSockets, /* > */
        ) -> Self::OutputSockets {
            todo!()
        } */

        fn compute(
            &self,
            _: Option<&[Reference]>,
            context: &impl ContextProvider,
        ) -> Box<[Option<OwnedDataType>]> {
            unreachable!()
        }

        /* fn get_sockets(
            &self,
        ) -> (
            Vec<crate::data_types::DataTypeType>,
            Vec<crate::data_types::DataTypeType>,
        ) {
            (vec![DataTypeType::Omni], Vec::new())
        } */
        fn node_metadata(&self) -> StaticNodeMetadata {
            StaticNodeMetadata { color: "#86198f" }
        }
    }
    impl SocketInterface for SocketInputV1 {
        fn get_input_sockets(&self) -> Box<[TypeDescriptor]> {
            Box::default()
        }

        fn get_output_sockets(&self) -> Box<[TypeDescriptor]> {
            Box::new([TypeDescriptor {
                r#type: DataTypeKind::Omni,
                mutable: false,
            }])
        }
    }
}

mod socket_output {
    use serde::Deserialize;
    use serde::Serialize;

    use crate::nodes::ContextProvider;
    use crate::nodes::Node;

    use crate::nodes::PropertyInterface;
    use crate::nodes::PropertyNotFound;
    use crate::nodes::SetPropertyError;
    use crate::nodes::SocketInterface;
    use crate::nodes::StaticNodeMetadata;
    use crate::property::PropertyMetadata;
    use crate::type_system::OwnedDataType;
    use crate::type_system::Reference;
    use crate::type_system::TypeRef;
    use crate::type_system::data_types::DataTypeKind;
    use crate::type_system::data_types::TypeDescriptor;

    #[derive(Default, Debug, Serialize, Deserialize, Clone)]
    pub struct SocketOutputV1 {}

    impl PropertyInterface for SocketOutputV1 {
        fn try_set_property(
            &mut self,
            property_name: &str,
            value: OwnedDataType,
        ) -> Result<(), SetPropertyError> {
            //TODO
            Ok(())
        }

        fn try_get_property(&self, property: &str) -> Result<TypeRef, PropertyNotFound> {
            //TODO
            Err(PropertyNotFound)
        }

        fn get_properties(&self) -> Box<[PropertyMetadata]> {
            //TODO
            Box::new([])
        }

        fn try_set_property_index(
            &mut self,
            index: u8,
            value: OwnedDataType,
        ) -> Result<(), SetPropertyError> {
            todo!()
        }

        fn try_get_property_index(&self, index: u8) -> Result<TypeRef, PropertyNotFound> {
            todo!()
        }

        fn set_property_external(
            &mut self,
            index: u8,
            reference: Reference,
        ) -> Result<(), SetPropertyError> {
            todo!()
        }

        /* fn try_get_property_script(&self, property: &str) -> Result<String, PropertyNotFound> {
            todo!()
        } */

        /* fn try_get_property_metadata(
            &self,
            property: &str,
        ) -> Result<crate::data_types::PropertyMetadata, ()> {
            todo!()
        } */
    }

    impl Node for SocketOutputV1 {
        //type InputSockets = Omni;

        //type OutputSockets = ();

        /* fn compute(
            &self,
            input_sockets: /* impl TryInto< */ Self::InputSockets, /* > */
        ) -> Self::OutputSockets {
            todo!()
        } */

        fn compute(
            &self,
            input_sockets: Option<&[Reference]>,
            context: &impl ContextProvider,
        ) -> Box<[Option<OwnedDataType>]> {
            unreachable!()
        }

        /* fn get_sockets(
            &self,
        ) -> (
            Vec<crate::data_types::DataTypeType>,
            Vec<crate::data_types::DataTypeType>,
        ) {
            (vec![DataTypeType::Omni], Vec::new())
        } */
        fn node_metadata(&self) -> StaticNodeMetadata {
            StaticNodeMetadata { color: "#86198f" }
        }
    }
    impl SocketInterface for SocketOutputV1 {
        fn get_input_sockets(&self) -> Box<[TypeDescriptor]> {
            Box::new([TypeDescriptor {
                r#type: DataTypeKind::Omni,
                mutable: false,
            }])
        }

        fn get_output_sockets(&self) -> Box<[TypeDescriptor]> {
            Box::default()
        }
    }
}

pub use socket_input::SocketInputV1;
pub use socket_output::SocketOutputV1;
