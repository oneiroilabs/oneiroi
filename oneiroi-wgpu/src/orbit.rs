use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct OrbitCamera {
    target: glam::Vec3,
    yaw: f32,
    pitch: f32,
    radius: f32,
    dragging: bool,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self::new(Vec3::new(0.0, 0.0, 0.0), 10.)
    }
}

impl OrbitCamera {
    pub fn new(target: Vec3, radius: f32) -> Self {
        Self {
            target,
            yaw: 0.0f32.to_radians(),
            pitch: 0.0f32.to_radians(),
            radius,
            dragging: false,
        }
    }

    fn view_matrix(&self) -> Mat4 {
        let cos_pitch = self.pitch.cos();
        let sin_pitch = self.pitch.sin();
        let cos_yaw = self.yaw.cos();
        let sin_yaw = self.yaw.sin();

        let camera_pos = self.target
            + glam::Vec3::new(
                self.radius * cos_pitch * sin_yaw,
                self.radius * sin_pitch,
                self.radius * cos_pitch * cos_yaw,
            );

        Mat4::look_at_lh(camera_pos, self.target, Vec3::Y)
    }

    pub fn view_projection(&self, aspect_ratio: f32) -> Mat4 {
        let view = self.view_matrix();
        let projection =
            Mat4::perspective_infinite_reverse_lh(45.0f32.to_radians(), aspect_ratio, 0.1);
        projection * view
    }

    pub fn set_dragging(&mut self, dragging: bool) {
        self.dragging = dragging
    }

    pub fn radius_delta(&mut self, delta: f32) {
        self.radius -= delta;
        //TODO maybe clamp?
    }
    pub fn yaw_pitch_delta(&mut self, yaw: f32, pitch: f32) {
        self.yaw += yaw;
        self.pitch -= pitch;
        // Prevent camera flipping over.
        self.pitch = self
            .pitch
            .clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
    }

    pub fn dragging(&self) -> bool {
        self.dragging
    }
}
