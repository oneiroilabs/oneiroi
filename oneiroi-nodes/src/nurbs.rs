use oneiroi_core::{arena::ExecutionContext, node::Node};
use oneiroi_core_macro::GraphNode;

//#[derive(GraphNode)]
struct Nurbs {
    //#[slot]
    whatever: u32,
}

impl<Ctx: ExecutionContext> Node<Ctx> for Nurbs {
    fn id(&self) -> &'static str {
        todo!()
    }

    fn display_name(&self) -> &str {
        todo!()
    }

    fn evaluate(&self, ctx: &mut Ctx, inputs: &[u32], outputs: &[u32]) {
        todo!()
    }
}
