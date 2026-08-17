/// The main way to declare a DataType.
/// Can afterwards be used as a Property<T>.
pub trait DataType: Clone + Default {
    //fn get_value_string() -> String;
    //type ParsingType: DataType;
    /// Config and Restrictions of the Data Type for Properties.
    type ConfigurationOptions: Clone;

    // Which identifier it has in the Type System.
    /* const DATA_TYPE_TYPE: DataTypeKind;


    fn generate_script(&self) -> String {
        unimplemented!()
    }

    //This could be constant but rust trolls idk if temporary or technical limitation
    fn intrinsic_attributes() -> Option<Box<[ArributeMetadata]>>;

    fn get_type_ref(value: TypeRef) -> &Self;

    fn get_type(value: OwnedDataType) -> Self;

    fn to_data_type_value(&self) -> OwnedDataType;

    fn to_data_type_ref(&self) -> TypeRef; */

    // Retrieves all the References necessary to properly
    // compute the DataType.
    // If a DataType cant store any References this function returns None.
    /* fn get_references(&self) -> Option<Box<[Reference]>> {
        None
    } */

    //TODO need to implement defualt string representation
}
