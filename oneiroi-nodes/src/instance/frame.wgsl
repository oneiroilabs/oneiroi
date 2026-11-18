// Where a shape put onto point `point` goes. Needs `similarity.wgsl`.
fn instance_frame(point: u32) -> Similarity {
    let frame = in_points(point);
    var scale = 1.0;
    if (in_scale_len() > 0u) {
        scale = in_scale(0u);
    }
    return Similarity(quat_from_basis(frame.normal, frame.binormal, frame.tangent), frame.position, scale);
}
