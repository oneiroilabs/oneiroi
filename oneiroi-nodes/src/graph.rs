use std::marker::PhantomData;

use oneiroi_core::types::DataType;
use petgraph::stable_graph::StableDiGraph;

pub struct DebugGraph<Node> {
    nodes: Vec<Node>,
    node_graph: StableDiGraph<u32, ()>,
    property_graph: StableDiGraph<u32, ()>,
}

// wthelly is the local/global mapping?
// -> Compile the graph
// -> For the compilation itself we already require access to the caching layer to bake the constants and retrieve their ids.
// -> probbly a provision method should be provided by the caching layer.
// -> Each instance which gets created will then also provision those variables.
// -> So we need to cache the required params in the shared graph and bake them in the generated shader code.
// ->

// maybe the core property should be:
struct Property<T: DataType> {
    value: u32,
    _type: PhantomData<T>,
}
// maybe actually not but instead the variable because then we could loose all concrete information about the "node" variables
// and only could take a function hint for execution...?
// This does not work because type information cannot be retained with generics in a collection...
struct Variable<T: DataType> {
    value: u32,
    _type: PhantomData<T>,
}

// the type erasure idea still has some potential for core i believe:
// idk how that would work tho because i dont like dynamic dispatch...
struct ErasedNode {
    variables: Box<[u32]>,
    exec: Box<dyn Fn()>,
}

// The erased approach has the problem that it would be a variable length encoding if multiple graphs and their instances are live.
// But that is a ? either way because im actually still not sure bout the instance buffer in the first place...

// maybe i should invest some more energy thinking abuot that?

// -> we codegen a type and shader per graph type
// -> each separate graph should then be a separate instanced draw probably?
// -> if this is the case we have either way separate instance buffers would be codegened too and then handed off? :thinking:



// would come from core..
trait Node {
    fn eval(&self);
}

struct Variable2<T: Node> {
    value: u32,
    _type: PhantomData<T>,
}
