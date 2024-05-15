use glam::{Mat4, Quat, Vec3};
use shipyard::Component;
use steel_common::data::{Data, Limit, Value};
use crate::edit::Edit;

/// The Transform component defines position, rotation, and scale of an entity.
#[derive(Component, Debug)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    /// Get the model matrix of this transform.
    pub fn model(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self { position: Default::default(), rotation: Default::default(), scale: Vec3::ONE }
    }
}

impl Edit for Transform {
    fn name() -> &'static str { "Transform" }

    fn get_data(&self) -> Data {
        Data::new().insert("position", Value::Vec3(self.position))
            .insert_with_limit("rotation", Value::Vec3(self.rotation.to_scaled_axis()), Limit::Float32Rotation)
            .insert("scale", Value::Vec3(self.scale))
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Vec3(v)) = data.get("position") { self.position = *v }
        if let Some(Value::Vec3(v)) = data.get("rotation") { self.rotation = Quat::from_scaled_axis(*v) }
        if let Some(Value::Vec3(v)) = data.get("scale") { self.scale = *v }
    }
}
