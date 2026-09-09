use std::{borrow::Cow, marker::PhantomData, sync::Arc};

use crate::{
    arena::{ArenaAccess, ExecutionContext},
    types::DataType,
};

pub trait Node<Ctx: ExecutionContext>: Send + Sync {
    fn id(&self) -> &'static str;

    fn display_name(&self) -> &str; //Cow<'_, str>;

    fn evaluate(&self, ctx: &mut Ctx, inputs: &[u32], outputs: &[u32]);
}

pub struct ErasedNode<Ctx: ExecutionContext> {
    node: Arc<dyn Node<Ctx>>,

    schmode: Box<dyn Fn(&mut Ctx, &[u32], &[u32])>,

    inputs: Box<[u32]>,
    outputs: Box<[u32]>,
}

trait IntoErased<Ctx: ExecutionContext> {
    fn into_erased(&self) -> ErasedNode<Ctx>;
}

impl<Ctx: ExecutionContext, Func: Fn(&mut Ctx, &[u32], &[u32])> IntoErased<Ctx> for Func {
    fn into_erased(&self) -> ErasedNode<Ctx> {
        todo!()
    }
}

trait DebugNode {}

/* pub struct VariableMetadata {
    pub buffer_id: u32,
    pub byte_offset: u32,
    pub data_type: DataType,
} */

struct Hmmge<T: DataType> {
    access: u32,
    _phantom: PhantomData<T>,
}

impl<T: DataType> Hmmge<T> {
    fn get<Ctx: ExecutionContext + ArenaAccess<T>>(&self, ctx: Ctx) -> T {
        ctx.get_unchecked::<T>(self.access)
    }
}

struct NodeThing {
    first: Hmmge<u32>,
}

impl<Ctx: ExecutionContext> Node<Ctx> for NodeThing {
    fn id(&self) -> &'static str {
        todo!()
    }

    fn display_name(&self) -> &str {
        todo!()
    }

    fn evaluate(&self, ctx: &mut Ctx, inputs: &[u32], outputs: &[u32]) {
        self.first.get(ctx);
    }
}
