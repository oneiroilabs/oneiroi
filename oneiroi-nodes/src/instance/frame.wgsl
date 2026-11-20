// Where a shape put onto point `point` goes, turned as `align` says. The
// same as `aligned` on the CPU. Needs `similarity.wgsl`.
fn instance_frame(point: u32) -> Similarity {
    let frame = in_points(point);
    var scale = 1.0;
    if (in_scale_len() > 0u) {
        scale = in_scale(0u);
    }
    var align = 0u;
    if (in_align_len() > 0u) {
        align = min(in_align(0u), 2u);
    }
    var rotation = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    if (align == 0u) {
        rotation = quat_from_basis(frame.normal, frame.binormal, frame.tangent);
    } else if (align == 1u) {
        // Where it heads, flat; straight up or down it heads nowhere.
        var heading = vec3<f32>(frame.tangent.x, 0.0, frame.tangent.z);
        let len = length(heading);
        if (len > 1e-30) {
            heading = heading / len;
        } else {
            heading = vec3<f32>(1.0, 0.0, 0.0);
        }
        let up = vec3<f32>(0.0, 1.0, 0.0);
        rotation = quat_from_basis(heading, up, cross(heading, up));
    }
    return Similarity(rotation, frame.position, scale);
}
