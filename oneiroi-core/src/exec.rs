// The Traits and behaviour for executors.

use std::collections::HashMap;

struct UniqueGraph {
    graph: u32,
    instances: u32,
}

pub struct OneiroiData {
    compiled_graphs: HashMap<u32, UniqueGraph>,
}

// The Graph is compiled once.
// The Compiled Graph lives in core.
// Compiled Graph != the  global data structure.
// When instancing new graph we request new slots from the OneiroiData struct.
// This means we have a global buffer with compiled graphs

//@group(0) @binding(0)
//var<storage, read> segments: array<CompiledGraph>;

// Compiled Graph can dispatch on the gpu via multi indirect how many instances are rendered of it?
// -> Then exec.

// There is a global hetero arena/ slab/ offset

// Specification of the graph:
// How could we evaluate incrementally?
// -> Caching the values
// When would we want to evaluate incrementally?
// -> Very Heavy compute operations
// -> Something like caching the rmf when curve not changing.
// -> That would be statically known while compiling.
// -> We would have coalescing nodes in the graph.
// What would be the coalescing rules and how would they map to a graph?
// -> A node coal always be coalesced when:
//  -> Input and parameters do not change.
//  -> TODO
// -> Multiple exposed parameters can change simultaneously.
// How does the graph/instances know what nodes are dirty?
// -> We probably have a separate shader per compiled graph.
// -> That means we can bake certain things inside the graph.
// How would we get the graph uniforms or similar into the graph instance?
// Would it require a different struct and uniform buffer for each one because a graph can have multiple input params.
// -> You could always pass a fixed size array to each graph e.g. 128 u32 where you point to the index in the heterogeneous arena.
// -> The actual associated buffer with the index can then be codegened into the shader itself making the access correct.
// -> That would allow using a global param buffer for each graph because its constant size.

// A shader per node type approach could also be used. Those will result in too many passes though i guess...