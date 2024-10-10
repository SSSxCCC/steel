use crate::edit::Edit;
use shipyard::Component;
use steel_common::data::{Data, Value};

/// The component that defines the name of an entity.
/// Steel editor displays this name in entities list.
#[derive(Component, Edit, Default, Debug)]
pub struct Name(pub String);

impl Name {
    /// Create a Name component.
    pub fn new(name: impl Into<String>) -> Self {
        name.into().into()
    }
}

impl std::ops::Deref for Name {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Name {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<String> for Name {
    fn from(name: String) -> Self {
        Name(name)
    }
}
