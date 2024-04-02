use shipyard::Component;
use steel_common::data::ComponentData;

pub trait Edit: Component + Default {
    fn name() -> &'static str;

    fn get_data(&self) -> ComponentData {
        ComponentData::new()
    }

    fn set_data(&mut self, #[allow(unused)] data: &ComponentData) { }

    fn from(data: &ComponentData) -> Self {
        let mut e = Self::default();
        e.set_data(data);
        e
    }
}
