use glam::{Vec2, Vec3};
use shipyard::Component;
use steel_common::{ComponentData, Limit, Value};
use crate::Edit;

#[derive(Component, Debug)]
pub struct Transform2D {
    pub position: Vec3,
    pub rotation: f32, // radian
    pub scale: Vec2,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self { position: Default::default(), rotation: Default::default(), scale: Vec2::ONE }
    }
}

impl Edit for Transform2D {
    fn name() -> &'static str { "Transform2D" }

    fn get_data(&self) -> ComponentData {
        let mut data = ComponentData::new();
        data.values.insert("position".into(), Value::Vec3(self.position));
        data.add("rotation", Value::Float32(self.rotation), Limit::Float32Rotation);
        data.values.insert("scale".into(), Value::Vec2(self.scale));
        data
    }

    fn set_data(&mut self, data: &ComponentData) {
        if let Some(Value::Vec3(v)) = data.values.get("position") { self.position = *v }
        if let Some(Value::Float32(f)) = data.values.get("rotation") { self.rotation = *f }
        if let Some(Value::Vec2(v)) = data.values.get("scale") { self.scale = *v }
    }
}
