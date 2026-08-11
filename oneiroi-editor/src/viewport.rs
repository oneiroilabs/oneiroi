use glam::Vec4;
use iced::widget::shader::{self, Pipeline, Primitive};
use oneiroi_core::curve::nurbs::CubicNurbs;
use oneiroi_wgpu::{State, orbit::OrbitCamera};

pub struct OneiroiScene {
    curve: CubicNurbs,
    camera: OrbitCamera,
}

impl OneiroiScene {
    pub fn new() -> Self {
        let curve = {
            let num_points = 15;
            let mut control_points = Vec::with_capacity(num_points);

            let step_distance = 0.25;

            // Scale factor for how quickly the spiral expands outwards (the 'a' coefficient)
            let expansion_rate = 0.15;

            for i in 0..num_points {
                let step = i as f64;

                let target_arc_length = step * step_distance;

                let theta = if target_arc_length > 0.0 {
                    (2.0 * target_arc_length / expansion_rate).sqrt()
                } else {
                    0.0
                };

                let radius = expansion_rate * theta;

                let x = radius * theta.cos();
                let y = radius * theta.sin();

                control_points.push(Vec4::new(x as f32, y as f32, 0.0, 1.0));
            }

            /* for i in 0..num_points {
                control_points.push(Vec4::new((i as i32 - 100) as f32, 0.0, 0.0, 1.0));
            } */

            let num_knots = num_points + 4;

            let mut knot_vec = vec![0.0; num_knots];
            for i in num_points..num_knots {
                knot_vec[i] = 1.0;
            }
            let num_interior_segments = num_points - 3;
            for i in 4..num_points {
                let interior_t = (i - 3) as f32 / num_interior_segments as f32;
                knot_vec[i] = interior_t;
            }

            oneiroi_core::curve::nurbs::CubicNurbs::new(control_points, knot_vec)
        };

        let camera = {
            let camera = OrbitCamera::new(glam::Vec3::new(0., 0., 0.0), 10.0);

            /* let aspect_ratio = size.width as f32 / size.height as f32;
            let view = camera.build_view_matrix();
            let projection = glam::Mat4::perspective_infinite_reverse_lh(
                45.0f32.to_radians(),
                aspect_ratio,
                0.1,
            );
            let view_projection_matrix = projection * view; */
            camera
        };

        Self { curve, camera }
    }
}

impl<Message> shader::Program<Message> for OneiroiScene {
    type State = ();

    type Primitive = Prim;

    fn draw(
        &self,
        state: &Self::State,
        cursor: iced_core::mouse::Cursor,
        bounds: iced::Rectangle,
    ) -> Self::Primitive {
        Prim {
            camera: self.camera,
            curve: self.curve.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Prim {
    camera: OrbitCamera,
    curve: CubicNurbs,
}

impl Primitive for Prim {
    type Pipeline = OneiroiPipe;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        bounds: &iced::Rectangle,
        viewport: &shader::Viewport,
    ) {
        todo!()
    }

    fn render(
        &self,
        _pipeline: &Self::Pipeline,
        _encoder: &mut iced::wgpu::CommandEncoder,
        _target: &iced::wgpu::TextureView,
        _clip_bounds: &iced::Rectangle<u32>,
    )
    {
        
    }
}

pub struct OneiroiPipe(PipelineState);

impl Pipeline for OneiroiPipe {
    fn new(
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        format: iced::wgpu::TextureFormat,
    ) -> Self
    where
        Self: Sized,
    {
        OneiroiPipe(PipelineState::new(device, format))
    }
}
