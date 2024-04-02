use shipyard::Component;
use steel_common::data::{ComponentData, Value};
use crate::edit::Edit;

#[derive(Component, Debug, Default)]
pub struct EntityInfo {
    pub name: String,
    // steel-editor can read EntityId from EntityData so that we don't need to store EntityId here
}

impl EntityInfo {
    pub fn new(name: impl Into<String>) -> Self {
        EntityInfo { name: name.into() }
    }
}

impl Edit for EntityInfo {
    fn name() -> &'static str { "EntityInfo" }

    fn get_data(&self) -> ComponentData {
        let mut data = ComponentData::new();
        data.values.insert("name".into(), Value::String(self.name.clone()));
        data
    }

    fn set_data(&mut self, data: &ComponentData) {
        if let Some(Value::String(s)) = data.values.get("name") { self.name = s.clone() }
    }
}
