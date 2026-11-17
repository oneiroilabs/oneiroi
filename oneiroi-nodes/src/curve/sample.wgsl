
// How many frames every curve gets. Spacing them is left to the CPU.
fn sample_curve_per_curve() -> u32 {
    if (in_t_len() > 0u) {
        return in_t_len();
    }
    return in_count(0u);
}

// How far along its curve a frame is, as a fraction of the length.
fn sample_curve_fraction(sample: u32, per: u32) -> f32 {
    if (in_t_len() > 0u) {
        return clamp(in_t(sample), 0.0, 1.0);
    }
    if (per < 2u) {
        return 0.0;
    }
    return f32(sample) / f32(per - 1u);
}

fn sample_curve_frames(index: u32) -> EvaluatedFrame {
    let per = sample_curve_per_curve();
    let curve = index / per;
    let start = in_segments_start(curve);
    let end = in_segments_end(curve);

    let first = in_segments(start);
    let begin = first.cumulative_length - first.length;
    let length = in_segments(end - 1u).cumulative_length - begin;
    let distance = sample_curve_fraction(index % per, per) * length + begin;

    // The first segment which ends behind the distance, or the last one.
    var low = start;
    var high = end;
    while (low < high) {
        let middle = (low + high) / 2u;
        if (distance >= in_segments(middle).cumulative_length) {
            low = middle + 1u;
        } else {
            high = middle;
        }
    }
    let segment = in_segments(min(low, end - 1u));
    let local = distance - (segment.cumulative_length - segment.length);
    let frame = curve_frame(segment, curve_u_at_distance(segment, local));
    return EvaluatedFrame(frame.position, frame.tangent, frame.normal, frame.binormal);
}

fn sample_curve_offsets(curve: u32) -> u32 {
    return curve * sample_curve_per_curve();
}
