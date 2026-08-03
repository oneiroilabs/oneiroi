pub struct OrbitCamera {
    pub target: glam::Vec3,
    pub yaw: f32,    // Rotation um die Y-Achse (in Radiant)
    pub pitch: f32,  // Rotation hoch/runter (in Radiant)
    pub radius: f32, // Abstand zum Target
    pub is_dragging: bool,
}

impl OrbitCamera {
    pub fn new(target: glam::Vec3, radius: f32) -> Self {
        Self {
            target,
            yaw: 0.0f32.to_radians(),
            pitch: 0.0f32.to_radians(),
            radius,
            is_dragging: false,
        }
    }

    // Berechnet die aktuelle View-Matrix basierend auf den Orbit-Koordinaten
    pub fn build_view_matrix(&self) -> glam::Mat4 {
        // Sphärische Koordinaten in kartesische umrechnen
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

        glam::Mat4::look_at_lh(camera_pos, self.target, glam::Vec3::Y)
    }
}
