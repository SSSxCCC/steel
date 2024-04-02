use glam::{Mat4, Quat, Vec3};
use shipyard::Component;
use steel_common::data::{ComponentData, Limit, Value};
use crate::edit::Edit;

#[derive(Component, Debug)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
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

    fn get_data(&self) -> ComponentData {
        let mut data = ComponentData::new();
        data.values.insert("position".into(), Value::Vec3(self.position));
        data.add("rotation", Value::Vec3(self.rotation.to_scaled_axis()), Limit::Float32Rotation);
        data.values.insert("scale".into(), Value::Vec3(self.scale));
        data
    }

    fn set_data(&mut self, data: &ComponentData) {
        if let Some(Value::Vec3(v)) = data.values.get("position") { self.position = *v }
        if let Some(Value::Vec3(v)) = data.values.get("rotation") { self.rotation = Quat::from_scaled_axis(*v) }
        if let Some(Value::Vec3(v)) = data.values.get("scale") { self.scale = *v }
    }
}
