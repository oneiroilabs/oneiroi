use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::core::{Interface, PCSTR};

pub unsafe fn setup_work_graph(
    device: &ID3D12Device5,
) -> Result<(ID3D12StateObject, ID3D12Resource), windows::core::Error> {
    // -------------------------------------------------------------
    // 1. Query the device for Work Graph API Support (Shader Model 6.8)
    // -------------------------------------------------------------
    let mut feature_data = D3D12_FEATURE_DATA_D3D12_OPTIONS21::default();
    device.CheckFeatureSupport(
        D3D12_FEATURE_D3D12_OPTIONS21,
        &mut feature_data as *mut _ as *mut _,
        std::mem::size_of::<D3D12_FEATURE_DATA_D3D12_OPTIONS21>() as u32,
    )?;

    if feature_data.WorkGraphsTier == D3D12_WORK_GRAPHS_TIER_NOT_SUPPORTED {
        panic!("Work Graphs are not supported on this GPU/Driver combination.");
    }

    // -------------------------------------------------------------
    // 2. Load and Compile Shader Library (Pre-compiled or via DXC)
    // -------------------------------------------------------------
    // For production, load your compiled DXIL bytecode (.bin) file containing SM 6.8
    let dxil_bytecode: Vec<u8> = std::fs::read("work_graph.dxil").expect("Failed to load DXIL");

    // -------------------------------------------------------------
    // 3. Define the State Object Subobjects to assemble the Work Graph
    // -------------------------------------------------------------
    let mut subobjects = Vec::new();

    // A. Define the DXIL Library container
    let dxil_lib_desc = D3D12_DXIL_LIBRARY_DESC {
        DXILLibrary: D3D12_SHADER_BYTECODE {
            pShaderBytecode: dxil_bytecode.as_ptr() as *const _,
            BytecodeLength: dxil_bytecode.len(),
        },
        NumExports: 0, // 0 exports means implicitly export all nodes in the file
        pExports: std::ptr::null(),
    };

    subobjects.push(D3D12_STATE_SUBOBJECT {
        Type: D3D12_STATE_SUBOBJECT_TYPE_DXIL_LIBRARY,
        pDesc: &dxil_lib_desc as *const _ as *const _,
    });

    // B. Explicitly define the Work Graph Config
    let graph_name = windows::core::w!("MyWorkGraph");
    let work_graph_desc = D3D12_WORK_GRAPH_DESC {
        ProgramName: graph_name,
        Flags: D3D12_WORK_GRAPH_FLAG_NONE,
        NumEntrypoints: 0,
        pEntrypoints: std::ptr::null(),
        NumExplicitlyDefinedNodes: 0,
        pExplicitlyDefinedNodes: std::ptr::null(),
    };

    subobjects.push(D3D12_STATE_SUBOBJECT {
        Type: D3D12_STATE_SUBOBJECT_TYPE_WORK_GRAPH,
        pDesc: &work_graph_desc as *const _ as *const _,
    });

    // -------------------------------------------------------------
    // 4. Create the final Executable State Object
    // -------------------------------------------------------------
    let state_object_desc = D3D12_STATE_OBJECT_DESC {
        Type: D3D12_STATE_OBJECT_TYPE_COLLECTION,
        NumSubobjects: subobjects.len() as u32,
        pSubobjects: subobjects.as_ptr(),
    };

    let state_object: ID3D12StateObject = unsafe { device.CreateStateObject(&state_object_desc)? };

    // -------------------------------------------------------------
    // 5. Query Graph Properties and Allocate Backing Memory
    // -------------------------------------------------------------
    // Work Graphs require scratch memory allocated by the CPU for internal GPU scheduling data
    let work_graph_properties: ID3D12WorkGraphProperties = state_object.cast()?;
    let graph_index = work_graph_properties.GetWorkGraphIndex(graph_name);

    let mut memory_requirements = D3D12_WORK_GRAPH_MEMORY_REQUIREMENTS::default();
    work_graph_properties.GetWorkGraphMemoryRequirements(graph_index, &mut memory_requirements);

    // Allocate a raw GPU buffer matched exactly to 'memory_requirements.MaxSizeInBytes'
    let backing_memory_buffer = create_gpu_buffer(device, memory_requirements.MaxSizeInBytes)?;

    Ok((state_object, backing_memory_buffer))
}

// Utility function to instantiate raw default-heap buffers natively
unsafe fn create_gpu_buffer(
    device: &ID3D12Device,
    size: u64,
) -> Result<ID3D12Resource, windows::core::Error> {
    let mut resource: Option<ID3D12Resource> = None;
    let heap_properties = D3D12_HEAP_PROPERTIES {
        Type: D3D12_HEAP_TYPE_DEFAULT,
        ..Default::default()
    };
    let resource_desc = D3D12_RESOURCE_DESC {
        Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
        Width: size,
        Height: 1,
        DepthOrArraySize: 1,
        MipLevels: 1,
        Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
        Flags: D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS,
        ..Default::default()
    };
    device.CreateCommittedResource(
        &heap_properties,
        D3D12_HEAP_FLAG_NONE,
        &resource_desc,
        D3D12_RESOURCE_STATE_COMMON,
        None,
        &mut resource,
    )?;
    Ok(resource.unwrap())
}

pub unsafe fn dispatch_graph(
    command_list: &ID3D12GraphicsCommandList,
    state_object: &ID3D12StateObject,
    backing_memory: &ID3D12Resource,
) -> Result<(), windows::core::Error> {
    // 1. Cast command list up to Interface version 10 to expose DispatchGraph
    let cmd_list_10: ID3D12GraphicsCommandList10 = command_list.cast()?;

    // 2. Provide the GPU memory pointer where the graph schedules nodes
    let program_identifier = state_object
        .cast::<ID3D12StateObjectProperties>()?
        .GetShaderIdentifier(windows::core::w!("MyWorkGraph"));

    let dispatch_desc = D3D12_DISPATCH_GRAPH_DESC {
        Mode: D3D12_DISPATCH_MODE_NODE_CPU_INPUT,
        Anonymous: D3D12_DISPATCH_GRAPH_DESC_0 {
            // Point the internal scheduler to our allocated backing memory pool
            NodeCPUInput: D3D12_NODE_CPU_INPUT {
                EntrypointIndex: 0,
                NumRecords: 1,
                pRecords: std::ptr::null(), // Populate with entry point data structs if needed
                RecordStrideInBytes: std::mem::size_of::<u32>() as u64,
            },
        },
    };

    // 3. Dispatch the workload autonomously outside of any render pass or draw scopes
    cmd_list_10.DispatchGraph(&dispatch_desc);

    Ok(())
}
