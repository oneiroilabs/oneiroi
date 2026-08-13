use std::time::Instant;

use glam::{Vec2, Vec3, Vec4, Vec4Swizzles};
use iced::{
    Event,
    widget::shader::{self, Pipeline, Primitive},
};
use oneiroi_core::curve::nurbs::CubicNurbs;
use oneiroi_wgpu::{
    PipelineState, RmfVisualizerUniforms, SdfUniforms, State, TubeUniforms, orbit::OrbitCamera,
};

#[derive(Debug)]
pub struct OneiroiScene {
    curve: CubicNurbs,
    camera: OrbitCamera,
    vis_uniforms: RmfVisualizerUniforms,
    tube_uniforms: TubeUniforms,
    sdf_uniforms: SdfUniforms,
}

impl OneiroiScene {
    pub fn new() -> Self {
        let curve = {
            let num_points = 15;
            let mut control_points = Vec::with_capacity(num_points);

            let step_distance = 0.25;

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

        let radial_segments = 16;

        let tube_radius = 0.2;

        let view = camera.build_view_matrix();
        let projection =
            glam::Mat4::perspective_infinite_reverse_lh(45.0f32.to_radians(), 4. / 3., 0.1);
        let view_projection = projection * view;

        let tube_uniforms = TubeUniforms::new(view_projection, tube_radius, radial_segments);

        let vis_uniforms = RmfVisualizerUniforms::new(view_projection, 0.25);
        let sdf_uniforms = SdfUniforms {
            color: Vec4::new(1.0, 0.0, 0.0, 1.0),
            origin: Vec3::new(0., 0., 0.),
            radius: 0.2,
            view_projection: view_projection.inverse(),
        };

        Self {
            curve,
            camera,
            vis_uniforms,
            tube_uniforms,
            sdf_uniforms,
        }
    }

    fn test_ray(&self, ndc_x: f32, ndc_y: f32) -> bool {
        let view = self.camera.build_view_matrix();
        let projection =
            glam::Mat4::perspective_infinite_reverse_lh(45.0f32.to_radians(), 4. / 3., 0.1);
        let inv_view_proj = (projection * view).inverse_or_zero();

        let near_target = inv_view_proj * glam::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);
        let world_near = near_target.xyz() / near_target.w;

        let dir_target = inv_view_proj * glam::Vec4::new(ndc_x, ndc_y, 0.9, 1.0);
        let world_dir_p = dir_target.xyz() / dir_target.w;

        let ray_origin = world_near;
        let ray_dir = (world_dir_p - world_near).normalize();

        let oc = ray_origin - self.sdf_uniforms.origin;

        let b = oc.dot(ray_dir);
        let c = oc.dot(oc) - (self.sdf_uniforms.radius * self.sdf_uniforms.radius);

        let discriminant = b * b - c;

        if discriminant < 0.0 {
            return false;
        }

        let t = -b - discriminant.sqrt();

        t > 0.0
    }
}

impl<Message> shader::Program<Message> for OneiroiScene {
    type State = ();

    type Primitive = Prim;

    fn update(
        &self,
        _state: &mut Self::State,
        event: &iced_core::Event,
        bounds: iced::Rectangle,
        cursor: iced_core::mouse::Cursor,
    ) -> Option<shader::Action<Message>> {
        if let Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) = event {
            if let Some(cursor_position) = cursor.position_in(bounds) {
                // 1. Pixel-Koordinaten in NDC-Raum (-1.0 bis 1.0) umrechnen
                // Wichtig: WebGPU NDC Y geht von unten (-1) nach oben (1)
                let ndc_x = (cursor_position.x / bounds.width) * 2.0 - 1.0;
                let ndc_y = 1.0 - (cursor_position.y / bounds.height) * 2.0;

                let instant = Instant::now();
                if self.test_ray(ndc_x, ndc_y) {
                    println!("SDF Kugel im GUI-Widget angeklickt!");
                    // Hier müsstest du die Message zurückgeben.
                    // Da OneiroiScene aktuell generisch über <Message> ist, kannst du eine
                    // Callback-Struktur nutzen oder OneiroiScene fest an dein Custom Message-Enum binden.
                    //return Some(shader::Action::publish(Message::));
                }
                println!("{:?}", instant.elapsed());
            }
        }
        None
    }

    fn draw(
        &self,
        state: &Self::State,
        cursor: iced_core::mouse::Cursor,
        bounds: iced::Rectangle,
    ) -> Self::Primitive {
        Prim {
            camera: self.camera,
            curve: self.curve.clone(),
            vis_uniforms: self.vis_uniforms,
            tube_uniforms: self.tube_uniforms,
            sdf_uniforms: self.sdf_uniforms,
        }
    }
}

#[derive(Debug)]
pub struct Prim {
    camera: OrbitCamera,
    curve: CubicNurbs,
    vis_uniforms: RmfVisualizerUniforms,
    tube_uniforms: TubeUniforms,
    sdf_uniforms: SdfUniforms,
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
        let size = (
            viewport.physical_size().width,
            viewport.physical_size().height,
        );

        pipeline.0.update(
            device,
            queue,
            size,
            &self.tube_uniforms,
            &self.vis_uniforms,
            &self.sdf_uniforms,
            self.curve.segments(),
        );
    }

    fn render(
        &self,
        pipeline: &Self::Pipeline,
        encoder: &mut iced::wgpu::CommandEncoder,
        target: &iced::wgpu::TextureView,
        clip_bounds: &iced::Rectangle<u32>,
    ) {
        pipeline.0.render(
            target,
            encoder,
            self.curve.segments().len() as u32,
            Some((
                clip_bounds.x as f32,
                clip_bounds.y as f32,
                clip_bounds.width as f32,
                clip_bounds.height as f32,
            )),
        );
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
        println!("{format:?}");
        OneiroiPipe(PipelineState::new(device, format))
    }
}
