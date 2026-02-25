use glam::{FloatExt, Mat3A, Vec4};
use serde::{Deserialize, Serialize};

use crate::{
    ImVec,
    type_system::{
        data_types::{DataType, DataTypeKind, Transform, Vec3},
        variants::{TypeRef, OwnedDataType},
    },
};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Curve {
    control_points: ImVec<Vec4>,
    knot_vector: ImVec<f32>,
    degree: usize,
}

impl Curve {
    pub(crate) fn new(
        control_points: impl IntoIterator<Item = Vec4>,
        knot_vector: impl IntoIterator<Item = f32>,
        degree: usize,
    ) -> Self {
        Self {
            degree,
            control_points: ImVec::from_iter(control_points),
            knot_vector: ImVec::from_iter(knot_vector),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CubicBezier {
    ctrl_points: Vec<Vec3>,
}

impl CubicBezier {
    pub(crate) fn new() -> Self {
        Self {
            ctrl_points: Vec::new(),
        }
    }

    pub fn with_points(points: Vec<Vec3>) -> Self {
        Self {
            ctrl_points: points,
        }
    }

    pub(crate) fn sample(&self, progress: f32) -> Vec3 {
        let mut t = progress.fract();
        let mut inv_t = 1.0 - t;
        let index = if t == 0.0 && progress >= 1.0 {
            t = 1.0;
            inv_t = 0.0;
            (progress.floor() as usize - 1) * 3
        } else {
            progress.floor() as usize * 3
        };

        // println!("WRONG: {progress}");

        self.ctrl_points[index] * inv_t * inv_t * inv_t
            + self.ctrl_points[index + 1] * 3. * inv_t * inv_t * t
            + self.ctrl_points[index + 2] * 3. * inv_t * t * t
            + self.ctrl_points[index + 3] * t * t * t
    }

    fn num_arcs(&self) -> usize {
        (self.ctrl_points.len() - 1) / 3
    }

    pub(crate) fn multi_sample(&self, count: usize) -> Box<[Vec3]> {
        let mut sequence = Box::new_uninit_slice(count);
        let segments = self.num_arcs() as f32;
        let div = segments / count as f32;
        for iteration in 0..count {
            let sample_point = div * iteration as f32;
            sequence[iteration].write(self.sample(sample_point));
        }

        //println!("{sequence:?}");

        unsafe { sequence.assume_init() }
    }

    pub(crate) fn sample_at_fixed_distance(&self, distance: f32) -> Box<[Transform]> {
        const STEPS: usize = 15;
        let curve_length = self.approx_curve_length(STEPS);
        let num_samples = curve_length / distance;
        let num_samples = num_samples.floor() as usize;
        let mut transforms = Box::new_uninit_slice(num_samples);

        let length_info = self.get_flattenend_segment_lengths(STEPS);
        // println!("{:#?}", length_info);

        let mut desired_distance = 0.;

        for uninit_transform in transforms.iter_mut() {
            let mut t: f32 = 0.0;
            for (index, (cur_t, distance)) in length_info.iter().enumerate() {
                if *distance > desired_distance {
                    let prev_entry = length_info[index - 1];
                    let prev_dist = prev_entry.1;
                    let dist_percentage = (desired_distance - prev_dist) / (distance - prev_dist);
                    let prev_t = prev_entry.0;
                    t = prev_t.lerp(*cur_t, dist_percentage);

                    /*  println!(
                        "T: {t}, PERC: {dist_percentage}, DIST: {desired_distance}, PREV_T: {prev_t}, CUR_T: {cur_t}"
                    ); */

                    break;
                }
            }

            let position = self.sample(t);
            let tangent = self.tangent(t);

            //println!("tangent: {tangent}, at: {t}");
            //let angle = Vec3::X.angle_between(tangent);
            //let rotation = Quat::from_rotation_y(angle);
            //let translation = Affine3A::from_translation(position);
            //let rotation =
            let transform = if tangent == Vec3::ZERO {
                Transform::from_translation(position)
            } else {
                frame_to_affine(position, tangent, Vec3::Y)
            };

            uninit_transform.write(transform);

            desired_distance += distance;
        }
        unsafe { transforms.assume_init() }
    }

    pub(crate) fn get_flattenend_segment_lengths(&self, steps: usize) -> Box<[(f32, f32)]> {
        let arcs = self.num_arcs();
        let samples = self.multi_sample(arcs * steps);
        let mut line_lengths = Box::new_uninit_slice(samples.len());

        let mut current_t = 1. / steps as f32;

        for sample in 0..samples.len() {
            if sample == 0 {
                line_lengths[sample].write((0., 0.));
                continue;
            }
            line_lengths[sample]
                .write((current_t, (samples[sample] - samples[sample - 1]).length()));

            unsafe {
                line_lengths[sample].assume_init_mut().1 +=
                    line_lengths[sample - 1].assume_init_mut().1;
            }

            current_t += 1. / steps as f32;
        }
        unsafe { line_lengths.assume_init() }
    }

    pub(crate) fn approx_curve_length(&self, steps: usize) -> f32 {
        let arcs = self.num_arcs();
        let mut length = 0.;
        let samples = self.multi_sample(arcs * steps);

        for sample in 1..samples.len() {
            length += (samples[sample] - samples[sample - 1]).length();
        }
        length
    }

    pub(crate) fn tangent(&self, progress: f32) -> Vec3 {
        let mut t = progress.fract();
        let mut inv_t = 1.0 - t;
        let index = if t == 0.0 && progress >= 1.0 {
            t = 1.0;
            inv_t = 0.0;
            (progress.floor() as usize - 1) * 3
        } else {
            progress.floor() as usize * 3
        };

        let raw_tangent =
            (self.ctrl_points[index + 1] - self.ctrl_points[index]) * inv_t * inv_t * 3.
                + (self.ctrl_points[index + 2] - self.ctrl_points[index + 1]) * 2. * inv_t * t * 3.
                + (self.ctrl_points[index + 3] - self.ctrl_points[index + 2]) * t * t * 3.;
        raw_tangent //.normalize_or_zero()
    }

    /* pub(crate) fn second_derivative(&self, sample_point: f32) -> Vec2 {
        let t = sample_point.fract();
        let t_inv = 1. - t;
        let index = (sample_point - 0.001).floor() as usize * 3;
        let tangent = self.tangent(sample_point);

        let raw_derivative = Vec2::new(
            (tangent.y - tangent.x) /* * t_inv * t_inv */ * 2.,
            (tangent.z - tangent.y) * 2.,
        );
        raw_derivative
    }

    pub(crate) fn normal_frenet(&self) -> Vec3 {
        let tan = self.tangent(0.0);
        let what=self.second_derivative(0.0);
        let derive2 = (tan.x+= + ).normalize();

    } */
}

impl DataType for CubicBezier {
    const DATA_TYPE_TYPE: DataTypeKind = DataTypeKind::CubicBezier;

    fn intrinsic_attributes() -> Option<Box<[super::ArributeMetadata]>> {
        None
    }

    type ConfigurationOptions = ();
    fn get_type_ref(value: TypeRef) -> &Self {
        match value {
            TypeRef::CubicBezier(val) => &val,
            _ => unreachable!(),
        }
    }

    fn get_type(value: OwnedDataType) -> Self {
        match value {
            OwnedDataType::CubicBezier(val) => *val,
            _ => unreachable!(),
        }
    }

    fn to_data_type_value(&self) -> OwnedDataType {
        OwnedDataType::CubicBezier(Box::new(self.clone()))
    }

    fn to_data_type_ref(&self) -> TypeRef {
        TypeRef::CubicBezier(self)
    }
}

/// This is essentially a look_to method in glam but for some reason glam does some weird stuff.
/// I dont know if this is a glam bug or its not suited for this usecase maybe investigate at some point.
fn frame_to_affine(pos: Vec3, forward: Vec3, up: Vec3) -> Transform {
    let mat = Mat3A::from_cols(
        forward.cross(up).normalize().into(),
        up.into(),
        (-forward.normalize()).into(),
    );
    Transform {
        matrix3: mat,
        translation: pos.into(),
    }
}
