use shipyard::Component;
use steel_common::data::{Data, Value};
use crate::edit::Edit;

#[derive(Component, Edit, Default, Debug)]
pub struct EntityInfo {
    pub name: String,
    // steel-editor can read EntityId from EntityData so that we don't need to store EntityId here
}

impl EntityInfo {
    pub fn new(name: impl Into<String>) -> Self {
        EntityInfo { name: name.into() }
    }
}
