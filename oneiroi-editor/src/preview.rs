//! The displayed node, to orbit around.
//!
//! This is where the graph actually runs: the user interface hands out its
//! graphics device only while it draws, so what the application planned is
//! executed here, on that device, right before it is shown.

use std::sync::{Arc, Mutex};

use glam::{Vec2, Vec3};
use iced::{
    Event, Rectangle, mouse,
    widget::shader::{self, Action, Pipeline, Primitive},
};
use oneiroi_wgpu::{DrawStage, DrawView, MeshData, MeshPreview, orbit::OrbitCamera};

use crate::document::{Session, Shown};

/// Shows what a [`Session`] shows. Drag with the middle mouse button to
/// orbit, scroll to zoom.
#[derive(Clone)]
pub struct Preview {
    pub session: Arc<Mutex<Session>>,
    /// The camera is aimed at what is shown again whenever this changes.
    pub frame: u64,
    /// Steps along and across every patch of a surface.
    pub detail: (u32, u32),
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
    /// The camera which was aimed for the current request, once drawing
    /// has found out where to aim it.
    fn aimed(&self) -> Option<OrbitCamera> {
        let (request, camera) = self.session.lock().unwrap().framed?;
        (request == self.frame).then_some(camera)
    }
}

impl<Message> shader::Program<Message> for Preview {
    type State = CameraState;
    type Primitive = Shot;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        if state.framed != Some(self.frame)
            && let Some(camera) = self.aimed()
        {
            let dragging = state.camera.dragging();
            state.camera = camera;
            state.camera.set_dragging(dragging);
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
        // Where to aim is only known once things ran, which is while
        // drawing. Until then the request is passed along.
        let (camera, frame) = if state.framed == Some(self.frame) {
            (state.camera, None)
        } else {
            match self.aimed() {
                Some(camera) => (camera, None),
                None => (state.camera, Some(self.frame)),
            }
        };
        Shot {
            session: self.session.clone(),
            camera,
            frame,
            detail: self.detail,
        }
    }
}

/// One frame of the preview.
pub struct Shot {
    session: Arc<Mutex<Session>>,
    camera: OrbitCamera,
    /// A request to aim the camera which nobody answered yet.
    frame: Option<u64>,
    detail: (u32, u32),
}

impl std::fmt::Debug for Shot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shot").field("camera", &self.camera).finish()
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Drawing {
    Nothing,
    /// Straight from the store, by the [`DrawStage`].
    Value,
    Mesh,
}

pub struct PreviewPipeline {
    stage: DrawStage,
    mesh: MeshPreview,
    drawing: Drawing,
    /// The generation of the session the mesh was uploaded for.
    uploaded: Option<u64>,
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
            stage: DrawStage::new(device, format),
            mesh: MeshPreview::new(device, format),
            drawing: Drawing::Nothing,
            uploaded: None,
            bounds: (0.0, 0.0, 1.0, 1.0),
        }
    }
}

impl Primitive for Shot {
    type Pipeline = PreviewPipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        let mut session = self.session.lock().unwrap();
        let session = &mut *session;

        // From here on the graph runs on the device the interface draws with.
        if session.engine_mut().executor().gpu().is_none() {
            session
                .engine_mut()
                .executor_mut()
                .attach(device.clone(), queue.clone());
        }
        session.run();

        let mut camera = self.camera;
        if let Some(request) = self.frame
            && let Some((center, radius)) = session.bounds()
        {
            camera.frame(center, (radius * 3.0).max(0.1));
            session.framed = Some((request, camera));
        }

        let scale = viewport.scale_factor();
        pipeline.bounds = (
            bounds.x * scale,
            bounds.y * scale,
            bounds.width * scale,
            bounds.height * scale,
        );
        let size = viewport.physical_size();
        let size = (size.width, size.height);
        let view = DrawView {
            view_projection: camera.view_projection(bounds.width / bounds.height.max(1.0)),
            light: camera.position() - camera.target(),
            color: Vec3::new(0.8, 0.7, 0.6),
            rings: self.detail.0,
            sides: self.detail.1,
        };

        pipeline.drawing = Drawing::Nothing;
        match session.shown() {
            Shown::Nothing => {}
            Shown::Drawn { key, drawable } => {
                let (store, executor) = session.engine_mut().parts_mut();
                let Some(gpu) = executor.gpu_mut() else {
                    return;
                };
                gpu.sync(store);
                if let Some(handle) = store.get(key)
                    && let Some(page) = gpu.page(handle.page)
                {
                    pipeline
                        .stage
                        .prepare(device, queue, size, &drawable, page, handle, &view);
                    pipeline.drawing = Drawing::Value;
                }
            }
            Shown::Mesh { .. } => {
                if pipeline.uploaded != Some(session.generation()) {
                    pipeline.uploaded = Some(session.generation());
                    let mesh = session.mesh();
                    let mesh = mesh.as_ref().map(|mesh| MeshData {
                        positions: &mesh.positions,
                        normals: mesh.normals.as_deref(),
                        indices: &mesh.indices,
                    });
                    pipeline.mesh.set_mesh(device, mesh);
                }
                pipeline
                    .mesh
                    .prepare(device, queue, size, view.view_projection, view.light);
                pipeline.drawing = Drawing::Mesh;
            }
        }
    }

    fn render(
        &self,
        pipeline: &Self::Pipeline,
        encoder: &mut iced::wgpu::CommandEncoder,
        target: &iced::wgpu::TextureView,
        _clip_bounds: &Rectangle<u32>,
    ) {
        match pipeline.drawing {
            Drawing::Nothing => {}
            Drawing::Value => pipeline.stage.render(encoder, target, Some(pipeline.bounds)),
            Drawing::Mesh => pipeline.mesh.render(encoder, target, Some(pipeline.bounds)),
        }
    }
}
