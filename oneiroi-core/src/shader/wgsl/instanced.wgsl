struct InstanceArgs {
    // The word the first row starts at, in `instance_rows`.
    rows_offset: u32,
    rows: u32,
    // The word the offsets of the groups of the shape start at, and how
    // many there are. None at all are a single group of everything.
    groups_offset: u32,
    groups: u32,
    // How many elements the shape has, and its longest group.
    elements: u32,
    slots: u32,
}

@group(0) @binding(3) var<storage, read> instance_rows: array<u32>;
@group(0) @binding(4) var<storage, read> instance_groups: array<u32>;
@group(0) @binding(5) var<uniform> instance_args: InstanceArgs;
// How many instances are visible, and then which rows they are: see
// `cull`, which made it before.
@group(0) @binding(6) var<storage, read> instance_visible: array<u32>;

fn instance_rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

fn instance_place(t: Similarity, p: vec3<f32>) -> vec3<f32> {
    return instance_rotate(t.rotation, p) * t.scale + t.offset;
}

fn instance_group_start(group: u32) -> u32 {
    if (instance_args.groups == 0u) {
        return 0u;
    }
    return min(instance_groups[instance_args.groups_offset + group], instance_args.elements);
}

fn instance_group_end(group: u32) -> u32 {
    if (instance_args.groups == 0u) {
        return instance_args.elements;
    }
    return min(instance_groups[instance_args.groups_offset + group + 1u], instance_args.elements);
}
