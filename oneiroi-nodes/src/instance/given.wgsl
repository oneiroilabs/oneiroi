// The transform of Transform. Needs `similarity.wgsl`.
fn transform_given() -> Similarity {
    var translate = vec3<f32>(0.0);
    if (in_translate_len() > 0u) {
        translate = in_translate(0u);
    }
    var rotate = vec3<f32>(0.0);
    if (in_rotate_len() > 0u) {
        rotate = in_rotate(0u);
    }
    var scale = 1.0;
    if (in_scale_len() > 0u) {
        scale = in_scale(0u);
    }
    return Similarity(quat_from_degrees(rotate), translate, scale);
}
