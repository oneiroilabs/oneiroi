fn curve_frames(index: u32) -> EvaluatedFrame {
    var samples = 2u;
    if (in_samples_len() > 0u) {
        samples = max(in_samples(0u), 2u);
    }
    let u = f32(index % samples) / f32(samples - 1u);
    let frame = curve_frame(in_segments(index / samples), u);
    return EvaluatedFrame(frame.position, frame.tangent, frame.normal, frame.binormal);
}
