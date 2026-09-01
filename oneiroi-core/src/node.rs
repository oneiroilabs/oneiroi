use std::{borrow::Cow, sync::Arc};

use crate::arena::ExecutionContext;

pub trait Node<Ctx: ExecutionContext>: Send + Sync {
    fn id(&self) -> &'static str;

    fn display_name(&self) -> &str; //Cow<'_, str>;

    fn evaluate(&self, ctx: &mut Ctx, inputs: &[u32], outputs: &[u32]) -> String;
}

pub struct ErasedNode<Ctx: ExecutionContext> {
    node: Arc<dyn Node<Ctx>>,

    input_ids: Vec<u32>,
    output_ids: Vec<u32>,
}

/* pub struct VariableMetadata {
    pub buffer_id: u32,
    pub byte_offset: u32,
    pub data_type: DataType,
} */
