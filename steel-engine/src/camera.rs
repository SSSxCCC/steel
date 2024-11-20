pub use steel_common::camera::*;

use crate::{edit::Edit, transform::Transform};
use glam::{Mat4, Quat, UVec2, Vec3};
use shipyard::{
    AddComponent, Component, Get, IntoIter, IntoWithId, Unique, UniqueViewMut, View, ViewMut,
};
use steel_common::data::Data;

/// The camera info to use for current frame.
/// CameraInfo is overriden by [Camera] component every frame if it exists,
/// and is overriden by [SceneCamera] every frame if we are in steel-editor.
#[derive(Unique)]
pub struct CameraInfo {
    pub position: Vec3,
    pub rotation: Quat,
    pub settings: CameraSettings,
}

impl CameraInfo {
    /// Create a new CameraInfo.
    pub fn new() -> Self {
        CameraInfo {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            settings: CameraSettings::new_orthographic(),
        }
    }

    /// Caculate the (projection * view) matrix of camera.
    pub fn projection_view(&self, window_size: &UVec2) -> Mat4 {
        let direction = self.rotation * Vec3::NEG_Z;
        let up = self.rotation * Vec3::Y;
        let view = Mat4::look_at_rh(self.position, self.position + direction, up);
        let mut projection = match self.settings {
            CameraSettings::Orthographic {
                width,
                height,
                size,
                near,
                far,
            } => {
                let (half_width, half_height) = match size {
                    OrthographicCameraSize::FixedWidth => Self::fixed_width(width, window_size),
                    OrthographicCameraSize::FixedHeight => Self::fixed_height(height, window_size),
                    OrthographicCameraSize::MinWidthHeight => {
                        if width / height > window_size.x as f32 / window_size.y as f32 {
                            Self::fixed_width(width, window_size)
                        } else {
                            Self::fixed_height(height, window_size)
                        }
                    }
                };
                Mat4::orthographic_rh(
                    -half_width,
                    half_width,
                    -half_height,
                    half_height,
                    near,
                    far,
                )
            }
            CameraSettings::Perspective { fov, near, far } => {
                Mat4::perspective_rh(fov, window_size.x as f32 / window_size.y as f32, near, far)
            }
        };
        projection.y_axis.y *= -1.0;
        projection * view
    }

    fn fixed_width(width: f32, window_size: &UVec2) -> (f32, f32) {
        let half_width = width / 2.0;
        let half_height = half_width * window_size.y as f32 / window_size.x as f32;
        (half_width, half_height)
    }

    fn fixed_height(height: f32, window_size: &UVec2) -> (f32, f32) {
        let half_height = height / 2.0;
        let half_width = half_height * window_size.x as f32 / window_size.y as f32;
        (half_width, half_height)
    }

    pub fn set(&mut self, scene_camera: &SceneCamera) {
        self.position = scene_camera.position;
        self.rotation = scene_camera.rotation;
        self.settings = scene_camera.settings;
    }
}

/// The Camera component can be attached to an entity and move camera according to the [Transform] component.
#[derive(Component, Debug)]
pub struct Camera(pub CameraSettings);

impl std::ops::Deref for Camera {
    type Target = CameraSettings;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Camera {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for Camera {
    fn default() -> Self {
        Camera(CameraSettings::new_orthographic())
    }
}

impl Edit for Camera {
    fn name() -> &'static str {
        "Camera"
    }

    fn get_data(&self) -> Data {
        let mut data = Data::new();
        self.0.get_data(&mut data);
        data
    }

    fn set_data(&mut self, data: &Data) {
        self.0.set_data(data);
    }
}

/// Modify [CameraInfo] unique according to the [Camera] component.
pub fn camera_maintain_system(
    mut transform: ViewMut<Transform>,
    camera: View<Camera>,
    mut info: UniqueViewMut<CameraInfo>,
) {
    if let Some((e, camera)) = camera.iter().with_id().next() {
        if !transform.contains(e) {
            transform.add_component_unchecked(e, Transform::default());
        }
        let transform = transform.get(e).unwrap();
        info.position = transform.position;
        info.rotation = transform.rotation;
        info.settings = **camera;
    } // TODO: handle situation without Camera
}
