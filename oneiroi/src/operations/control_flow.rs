pub mod socket_input {
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct SocketInputV1 {}
}

pub mod socket_output {
    use serde::Deserialize;
    use serde::Serialize;

    use crate::asset::instance::AssetInstance;
    use crate::operations::Operation;
    //use crate::operations::SocketType;
    use crate::data_types::DataTypeInstance;
    use crate::data_types::DataTypeType;
    use crate::operations::PropertyInterface;
    use crate::operations::StaticNodeMetadata;

    #[derive(Default, Debug, Serialize, Deserialize, Clone)]
    pub struct SocketOutputV1 {}

    impl PropertyInterface for SocketOutputV1 {
        fn try_set_property(&mut self, property: &str, value: DataTypeInstance) -> Result<(), ()> {
            //TODO
            Ok(())
        }

        fn try_get_property(
            &self,
            property: &str,
        ) -> Result<crate::data_types::DataTypeInstance, ()> {
            //TODO
            Err(())
        }

        fn get_properties(&self) -> Vec<crate::data_types::PropertyMetadata> {
            //TODO
            Vec::new()
        }
    }

    impl Operation for SocketOutputV1 {
        //type InputSockets = Omni;

        //type OutputSockets = ();

        /* fn compute(
            &self,
            input_sockets: /* impl TryInto< */ Self::InputSockets, /* > */
        ) -> Self::OutputSockets {
            todo!()
        } */

        fn compute(&self, input_sockets: Vec<&DataTypeInstance>) -> Vec<DataTypeInstance> {
            todo!()
        }

        /* fn get_sockets(
            &self,
        ) -> (
            Vec<crate::data_types::DataTypeType>,
            Vec<crate::data_types::DataTypeType>,
        ) {
            (vec![DataTypeType::Omni], Vec::new())
        } */
        fn static_metadata(&self) -> StaticNodeMetadata {
            StaticNodeMetadata { color: "#86198f" }
        }

        fn get_input_sockets(&self) -> Vec<DataTypeType> {
            vec![DataTypeType::Omni]
        }

        fn get_output_sockets(&self) -> Vec<DataTypeType> {
            vec![]
        }
    }
}
