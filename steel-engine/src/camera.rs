use glam::{Mat4, Vec2, Vec3};
use shipyard::Unique;

#[derive(Unique)]
pub struct CameraInfo {
    position: Vec3,
    half_height: f32,
}

impl CameraInfo {
    pub fn new() -> Self {
        CameraInfo { position: Vec3::ZERO, half_height: 10.0 }
    }

    pub fn projection_view(&self, window_size: &Vec2) -> Mat4 {
        let view = Mat4::look_at_lh(self.position, self.position + Vec3::NEG_Z, Vec3::Y);
        let half_width = self.half_height * window_size.x / window_size.y as f32;
        let projection = Mat4::orthographic_lh(half_width, -half_width, self.half_height, -self.half_height, -1000.0, 1000.0);
        projection * view
    }
}
