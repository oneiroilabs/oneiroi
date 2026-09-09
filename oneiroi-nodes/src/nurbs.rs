use oneiroi_core::{arena::ExecutionContext, node::Node};
use oneiroi_core_macro::graph_node;

#[graph_node]
struct Nurbs {
    /// This is some nice docs.
    #[slot]
    whatever: u32,

    #[property]
    this_and_that: u32,
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

fn nurbs_thing<Ctx: ExecutionContext>(ctx: &mut Ctx, inputs: &[u32], outputs: &[u32]) {}
