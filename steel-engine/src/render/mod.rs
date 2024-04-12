pub mod canvas;
pub mod renderer2d;

use glam::Vec4;
use shipyard::Unique;
use steel_common::data::{Data, Limit, Value};
use crate::edit::Edit;

#[derive(Unique)]
pub struct RenderManager {
    pub clear_color: Vec4,
}

impl Default for RenderManager {
    fn default() -> Self {
        Self { clear_color: Vec4::ZERO }
    }
}

impl Edit for RenderManager {
    fn name() -> &'static str { "RenderManager" }

    fn get_data(&self) -> Data {
        let mut data = Data::new();
        data.add("clear_color", Value::Vec4(self.clear_color), Limit::Vec4Color);
        data
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Vec4(v)) = data.values.get("clear_color") { self.clear_color = *v }
    }
}
