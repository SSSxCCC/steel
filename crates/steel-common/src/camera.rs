use crate::data::{Data, Limit, Value};
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Camera info for scene window.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SceneCamera {
    pub position: Vec3,
    pub rotation: Quat,
    pub settings: CameraSettings,
}

impl Default for SceneCamera {
    fn default() -> Self {
        SceneCamera {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            settings: CameraSettings::new_orthographic(),
        }
    }
}

impl SceneCamera {
    /// Reset the camera, keeping the enum of camera settings.
    pub fn reset(&mut self) {
        self.position = Vec3::ZERO;
        self.rotation = Quat::IDENTITY;
        self.settings = match self.settings {
            CameraSettings::Orthographic { .. } => CameraSettings::new_orthographic(),
            CameraSettings::Perspective { .. } => CameraSettings::new_perspective(),
        };
    }

    /// Insert all values in self to [Data].
    pub fn get_data(&self, data: &mut Data) {
        data.insert("position", Value::Vec3(self.position))
            .insert_with_limit(
                "rotation",
                Value::Vec3(self.rotation.to_scaled_axis()),
                Limit::Float32Rotation,
            );
        self.settings.get_data(data);
    }

    /// Set values in self according to a [Data].
    pub fn set_data(&mut self, data: &Data) {
        if let Some(Value::Vec3(v)) = data.get("position") {
            self.position = *v
        }
        if let Some(Value::Vec3(v)) = data.get("rotation") {
            self.rotation = Quat::from_scaled_axis(*v)
        }
        self.settings.set_data(data);
    }
}

/// The camera settings.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CameraSettings {
    Orthographic {
        width: f32,
        height: f32,
        size: OrthographicCameraSize,
        near: f32,
        far: f32,
    },
    Perspective {
        /// The y fov radians.
        fov: f32,
        /// Must be greater than zero.
        near: f32,
        /// Must be greater than zero.
        far: f32,
    },
}

impl CameraSettings {
    /// Create a new orthographic camera settings with default value.
    pub fn new_orthographic() -> Self {
        CameraSettings::Orthographic {
            width: 20.0,
            height: 20.0,
            size: OrthographicCameraSize::FixedHeight,
            near: -1000000.0,
            far: 1000000.0,
        }
    }

    /// Create a new perspective camera settings with default value.
    pub fn new_perspective() -> Self {
        CameraSettings::Perspective {
            fov: 90.0_f32.to_radians(),
            near: 0.0001,
            far: 1000.0,
        }
    }

    /// Use with [crate::data::Limit::Int32Enum].
    pub fn to_i32(&self) -> i32 {
        match self {
            CameraSettings::Orthographic { .. } => 0,
            CameraSettings::Perspective { .. } => 1,
        }
    }

    /// Use with [crate::data::Limit::Int32Enum].
    pub fn from_i32(i: i32) -> Self {
        match i {
            0 => CameraSettings::new_orthographic(),
            1 => CameraSettings::new_perspective(),
            _ => CameraSettings::new_orthographic(),
        }
    }

    /// Use with [crate::data::Limit::Int32Enum].
    pub fn enum_vector() -> Vec<(i32, String)> {
        vec![(0, "Orthographic".into()), (1, "Perspective".into())]
    }

    /// Fill all values in self into a [Data].
    pub fn get_data(&self, data: &mut Data) {
        data.insert_with_limit(
            "mode",
            Value::Int32(self.to_i32()),
            Limit::Int32Enum(CameraSettings::enum_vector()),
        );
        match *self {
            CameraSettings::Orthographic {
                width,
                height,
                size,
                near,
                far,
            } => {
                data.insert_with_limit(
                    "size",
                    Value::Int32(size as i32),
                    Limit::Int32Enum(OrthographicCameraSize::enum_vector()),
                );
                match size {
                    OrthographicCameraSize::FixedWidth => {
                        data.insert("width", Value::Float32(width));
                    }
                    OrthographicCameraSize::FixedHeight => {
                        data.insert("height", Value::Float32(height));
                    }
                    OrthographicCameraSize::MinWidthHeight => {
                        data.insert("min_width", Value::Float32(width))
                            .insert("min_height", Value::Float32(height));
                    }
                }
                data.insert("near", Value::Float32(near))
                    .insert("far", Value::Float32(far));
            }
            CameraSettings::Perspective { fov, near, far } => {
                data.insert_with_limit("fov", Value::Float32(fov), Limit::Float32Rotation)
                    .insert_with_limit(
                        "near",
                        Value::Float32(near),
                        Limit::Float32Range(0.00001..=f32::MAX),
                    )
                    .insert_with_limit(
                        "far",
                        Value::Float32(far),
                        Limit::Float32Range(0.00001..=f32::MAX),
                    );
            }
        }
    }

    /// Set values in self according to a [Data].
    pub fn set_data(&mut self, data: &Data) {
        if let Some(Value::Int32(v)) = data.get("mode") {
            if self.to_i32() != *v {
                *self = CameraSettings::from_i32(*v);
            }
        }
        match self {
            CameraSettings::Orthographic {
                width,
                height,
                size,
                near,
                far,
            } => {
                if let Some(Value::Int32(v)) = data.get("size") {
                    *size = (*v).into()
                }
                match size {
                    OrthographicCameraSize::FixedWidth => {
                        if let Some(Value::Float32(v)) = data.get("width") {
                            *width = *v
                        }
                    }
                    OrthographicCameraSize::FixedHeight => {
                        if let Some(Value::Float32(v)) = data.get("height") {
                            *height = *v
                        }
                    }
                    OrthographicCameraSize::MinWidthHeight => {
                        if let Some(Value::Float32(v)) = data.get("min_width") {
                            *width = *v
                        };
                        if let Some(Value::Float32(v)) = data.get("min_height") {
                            *height = *v
                        };
                    }
                }
                if let Some(Value::Float32(v)) = data.get("near") {
                    *near = *v
                }
                if let Some(Value::Float32(v)) = data.get("far") {
                    *far = *v
                }
            }
            CameraSettings::Perspective { fov, near, far } => {
                if let Some(Value::Float32(v)) = data.get("fov") {
                    *fov = *v
                }
                if let Some(Value::Float32(v)) = data.get("near") {
                    *near = *v
                }
                if let Some(Value::Float32(v)) = data.get("far") {
                    *far = *v
                }
            }
        }
    }
}

/// The orthographic camera size settings. Since we can not fix the screen aspect ratio,
/// we must choose to either set the width or height, or set the minimum width and height.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OrthographicCameraSize {
    /// Set a width and caculate height by width / aspect_ratio.
    FixedWidth = 0,
    /// Set a height and caculate with by height * aspect_ratio.
    FixedHeight = 1,
    /// Set a min width and a min height.
    MinWidthHeight = 2,
}

impl From<i32> for OrthographicCameraSize {
    fn from(value: i32) -> Self {
        match value {
            0 => OrthographicCameraSize::FixedWidth,
            1 => OrthographicCameraSize::FixedHeight,
            2 => OrthographicCameraSize::MinWidthHeight,
            _ => OrthographicCameraSize::FixedHeight,
        }
    }
}

impl OrthographicCameraSize {
    /// Use with [crate::data::Limit::Int32Enum].
    pub fn enum_vector() -> Vec<(i32, String)> {
        vec![
            (0, "FixedWidth".into()),
            (1, "FixedHeight".into()),
            (2, "MinWidthHeight".into()),
        ]
    }
}
