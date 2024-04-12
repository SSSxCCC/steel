use glam::{Mat4, UVec2, Vec3};
use shipyard::{AddComponent, Component, Get, IntoIter, IntoWithId, Unique, UniqueViewMut, View, ViewMut};
use steel_common::{data::{Data, Value}, engine::EditorCamera};
use crate::{edit::Edit, transform::Transform};

#[derive(Unique)]
pub struct CameraInfo {
    pub position: Vec3,
    pub height: f32,
}

impl CameraInfo {
    pub fn new() -> Self {
        CameraInfo { position: Vec3::ZERO, height: 20.0 }
    }

    pub fn projection_view(&self, window_size: &UVec2) -> Mat4 {
        let view = Mat4::look_at_lh(self.position, self.position + Vec3::NEG_Z, Vec3::Y);
        let half_height = self.height / 2.0;
        let half_width = half_height * window_size.x as f32 / window_size.y as f32;
        let projection = Mat4::orthographic_lh(half_width, -half_width, half_height, -half_height, -1000000.0, 1000000.0);
        projection * view
    }

    pub fn set(&mut self, editor_camera: &EditorCamera) {
        self.position = editor_camera.position;
        self.height = editor_camera.height;
    }
}

#[derive(Component, Default, Debug)]
pub struct Camera {
    pub height: f32,
}

impl Edit for Camera {
    fn name() -> &'static str { "Camera" }

    fn get_data(&self) -> Data {
        let mut data = Data::new();
        data.values.insert("height".into(), Value::Float32(self.height));
        data
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Float32(f)) = data.values.get("height") { self.height = *f }
    }
}

pub fn camera_maintain_system(mut transform: ViewMut<Transform>, camera: View<Camera>, mut info: UniqueViewMut<CameraInfo>) {
    if let Some((e, camera)) = camera.iter().with_id().next() {
        if !transform.contains(e) {
            transform.add_component_unchecked(e, Transform::default());
        }
        let transform = transform.get(e).unwrap();
        info.position = transform.position;
        // TODO: transform.rotation
        info.height = camera.height;
    } // TODO: handle situation without Camera
}
