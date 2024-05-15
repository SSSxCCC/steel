use shipyard::Component;
use steel_common::data::{Data, Value};
use crate::edit::Edit;

/// The entity extra info, like entity name, mainly used in editor to display the name of this entity.
#[derive(Component, Edit, Default, Debug)]
pub struct EntityInfo {
    pub name: String,
    // steel-editor can read EntityId from EntityData so that we don't need to store EntityId here
}

impl EntityInfo {
    /// Create an EntityInfo with name.
    pub fn new(name: impl Into<String>) -> Self {
        EntityInfo { name: name.into() }
    }
}
