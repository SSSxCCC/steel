use glam::{Mat4, UVec2, Vec3};
use shipyard::{AddComponent, Component, Get, IntoIter, IntoWithId, Unique, UniqueViewMut, View, ViewMut};
use steel_common::{data::{Data, Limit, Value}, engine::EditorCamera};
use crate::{edit::Edit, transform::Transform};

/// The camera size specify. Since we can not fix the screen aspect ratio,
/// we must choose to either specify the width or height, or specify the minimum width and height
#[derive(Debug, Clone, Copy)]
pub enum CameraSpec {
    /// Specify a width and caculate height by width / aspect_ratio
    FixedWidth,
    /// Specify a height and caculate with by height * aspect_ratio
    FixedHeight,
    /// Specify a min width and a min height
    MinWidthHeight,
}

impl From<i32> for CameraSpec {
    fn from(value: i32) -> Self {
        match value {
            0 => CameraSpec::FixedWidth,
            1 => CameraSpec::FixedHeight,
            2 => CameraSpec::MinWidthHeight,
            _ => CameraSpec::FixedHeight,
        }
    }
}

/// The camera info to use for current frame.
/// CameraInfo is overriden by Camera component every frame if it exists,
/// and is overriden by EditorCamera every frame if we are in steel-editor.
#[derive(Unique)]
pub struct CameraInfo {
    pub position: Vec3,
    pub width: f32,
    pub height: f32,
    pub spec: CameraSpec,
}

impl CameraInfo {
    pub fn new() -> Self {
        CameraInfo { position: Vec3::ZERO, width: 20.0, height: 20.0, spec: CameraSpec::FixedHeight }
    }

    pub fn projection_view(&self, window_size: &UVec2) -> Mat4 {
        let view = Mat4::look_at_lh(self.position, self.position + Vec3::NEG_Z, Vec3::Y);
        let (half_width, half_height) = match self.spec {
            CameraSpec::FixedWidth => {
                self.fixed_width(window_size)
            },
            CameraSpec::FixedHeight => {
                self.fixed_height(window_size)
            },
            CameraSpec::MinWidthHeight => {
                if self.width / self.height > window_size.x as f32 / window_size.y as f32 {
                    self.fixed_width(window_size)
                } else {
                    self.fixed_height(window_size)
                }
            },
        };
        let projection = Mat4::orthographic_lh(half_width, -half_width, half_height, -half_height, -1000000.0, 1000000.0);
        projection * view
    }

    fn fixed_width(&self, window_size: &UVec2) -> (f32, f32) {
        let half_width = self.width / 2.0;
        let half_height = half_width * window_size.y as f32 / window_size.x as f32;
        (half_width, half_height)
    }

    fn fixed_height(&self, window_size: &UVec2) -> (f32, f32) {
        let half_height = self.height / 2.0;
        let half_width = half_height * window_size.x as f32 / window_size.y as f32;
        (half_width, half_height)
    }

    pub fn set(&mut self, editor_camera: &EditorCamera) {
        self.position = editor_camera.position;
        self.height = editor_camera.height;
        self.spec = CameraSpec::FixedHeight;
    }
}

/// The Camera component can be attached to an entity and move camera according to the Transform component.
#[derive(Component, Debug)]
pub struct Camera {
    pub width: f32,
    pub height: f32,
    pub spec: CameraSpec,
}

impl Default for Camera {
    fn default() -> Self {
        Self { width: 20.0, height: 20.0, spec: CameraSpec::FixedHeight }
    }
}

impl Edit for Camera {
    fn name() -> &'static str { "Camera" }

    fn get_data(&self) -> Data {
        let mut data = Data::new();
        data.add_value_with_limit("spec", Value::Int32(self.spec as i32),
            Limit::Int32Enum(vec![(0, "FixedWidth".into()), (1, "FixedHeight".into()), (2, "MinWidthHeight".into())]));
        match self.spec {
            CameraSpec::FixedWidth => data.add_value("width", Value::Float32(self.width)),
            CameraSpec::FixedHeight => data.add_value("height", Value::Float32(self.height)),
            CameraSpec::MinWidthHeight => {
                data.add_value("min_width", Value::Float32(self.width));
                data.add_value("min_height", Value::Float32(self.height));
            },
        }
        data
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Int32(v)) = data.get("spec") { self.spec = (*v).into() }
        match self.spec {
            CameraSpec::FixedWidth => if let Some(Value::Float32(v)) = data.get("width") { self.width = *v },
            CameraSpec::FixedHeight => if let Some(Value::Float32(v)) = data.get("height") { self.height = *v },
            CameraSpec::MinWidthHeight => {
                if let Some(Value::Float32(v)) = data.get("min_width") { self.width = *v };
                if let Some(Value::Float32(v)) = data.get("min_height") { self.height = *v };
            },
        }
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
        info.width = camera.width;
        info.height = camera.height;
        info.spec = camera.spec;
    } // TODO: handle situation without Camera
}
