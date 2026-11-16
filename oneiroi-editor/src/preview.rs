//! The mesh of the displayed node, to orbit around.

use std::sync::Arc;

use glam::Vec2;
use iced::{
    Event, Rectangle, mouse,
    widget::shader::{self, Action, Pipeline, Primitive},
};
use oneiroi_wgpu::{MeshData, MeshPreview, orbit::OrbitCamera};

use crate::document::PreviewMesh;

/// Draws a mesh. Drag with the middle mouse button to orbit, scroll to zoom.
#[derive(Debug, Clone, Default)]
pub struct Preview {
    pub mesh: Option<Arc<PreviewMesh>>,
    /// The camera is aimed at the mesh again whenever this changes.
    pub frame: u64,
}

pub struct CameraState {
    camera: OrbitCamera,
    last_cursor: Vec2,
    /// The [`Preview::frame`] the camera was last aimed for.
    framed: Option<u64>,
}

impl Default for CameraState {
    fn default() -> Self {
        let mut camera = OrbitCamera::default();
        camera.yaw_pitch_delta(0.6, -0.35);
        Self {
            camera,
            last_cursor: Vec2::ZERO,
            framed: None,
        }
    }
}

impl Preview {
    /// The camera to draw with: the one the user moved, or one aimed at the
    /// mesh if that was asked for since.
    fn camera(&self, state: &CameraState) -> (OrbitCamera, bool) {
        let mut camera = state.camera;
        match &self.mesh {
            Some(mesh) if state.framed != Some(self.frame) => {
                camera.frame(mesh.center, (mesh.radius * 3.0).max(0.1));
                (camera, true)
            }
            _ => (camera, false),
        }
    }
}

impl<Message> shader::Program<Message> for Preview {
    type State = CameraState;
    type Primitive = MeshPrimitive;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let (camera, aimed) = self.camera(state);
        if aimed {
            state.camera = camera;
            state.framed = Some(self.frame);
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                let position = cursor.position_in(bounds)?;
                state.camera.set_dragging(true);
                state.last_cursor = Vec2::new(position.x, position.y);
                None
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)) => {
                state.camera.set_dragging(false);
                None
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if state.camera.dragging() => {
                let position = cursor.position_in(bounds)?;
                let position = Vec2::new(position.x, position.y);
                let delta = (position - state.last_cursor) * 0.005;
                state.last_cursor = position;
                state.camera.yaw_pitch_delta(delta.x, delta.y);
                Some(Action::request_redraw())
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                cursor.position_in(bounds)?;
                let amount = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => y * 0.02,
                };
                // Steps which get smaller the closer the camera is.
                let distance = (state.camera.position() - state.camera.target()).length();
                state.camera.radius_delta(amount * distance * 0.1);
                Some(Action::request_redraw().and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        MeshPrimitive {
            mesh: self.mesh.clone(),
            camera: self.camera(state).0,
        }
    }
}

#[derive(Debug)]
pub struct MeshPrimitive {
    mesh: Option<Arc<PreviewMesh>>,
    camera: OrbitCamera,
}

pub struct PreviewPipeline {
    preview: MeshPreview,
    /// The generation of the mesh which is uploaded.
    shown: Option<u64>,
    /// Where the widget is, in pixels of the target.
    bounds: (f32, f32, f32, f32),
}

impl Pipeline for PreviewPipeline {
    fn new(
        device: &iced::wgpu::Device,
        _queue: &iced::wgpu::Queue,
        format: iced::wgpu::TextureFormat,
    ) -> Self {
        Self {
            preview: MeshPreview::new(device, format),
            shown: None,
            bounds: (0.0, 0.0, 1.0, 1.0),
        }
    }
}

impl Primitive for MeshPrimitive {
    type Pipeline = PreviewPipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        let generation = self.mesh.as_ref().map(|mesh| mesh.generation);
        if pipeline.shown != generation {
            pipeline.shown = generation;
            let mesh = self.mesh.as_ref().map(|mesh| MeshData {
                positions: &mesh.positions,
                normals: mesh.normals.as_deref(),
                indices: &mesh.indices,
            });
            pipeline.preview.set_mesh(device, mesh);
        }

        let scale = viewport.scale_factor();
        pipeline.bounds = (
            bounds.x * scale,
            bounds.y * scale,
            bounds.width * scale,
            bounds.height * scale,
        );

        let size = viewport.physical_size();
        let aspect = bounds.width / bounds.height.max(1.0);
        pipeline.preview.prepare(
            device,
            queue,
            (size.width, size.height),
            self.camera.view_projection(aspect),
            self.camera.position() - self.camera.target(),
        );
    }

    fn render(
        &self,
        pipeline: &Self::Pipeline,
        encoder: &mut iced::wgpu::CommandEncoder,
        target: &iced::wgpu::TextureView,
        _clip_bounds: &Rectangle<u32>,
    ) {
        pipeline
            .preview
            .render(encoder, target, Some(pipeline.bounds));
    }
}
