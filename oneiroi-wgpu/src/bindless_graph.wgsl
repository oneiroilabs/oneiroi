// == HEADER START
@group(0) @binding(0) 
var<storage, read> global_buffers: binding_array<array<u32>,2048>;

// Heterogeneous Arena
@group(0) @binding(1)
var<storage, read> nurbs_segements: array<NurbsSegments>
//...
// == HEADER END

// == UNIFORM STRUCT CODEGEN
struct GraphUniforms {
    thing: f32,
}

@group(2) @binding(0) var<uniform> exec_info: GraphUniforms;

// == UNIFORM STRUCT CODEGEN END


// == FUNCTIONS REQUIRED FOR EXECUTION
//...
// == FUNCTIONS END

// == ACTUAL SHADER
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    nurbs_segment[global_buffers[graph_id][argument_id]]
}