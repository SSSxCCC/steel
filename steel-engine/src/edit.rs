pub use steel_proc::Edit;

use steel_common::data::Data;

/// You can impl Edit for a Component or Unique so that they can be edited in steel-editor
pub trait Edit {
    fn name() -> &'static str;

    fn get_data(&self) -> Data {
        Data::new()
    }

    fn set_data(&mut self, data: &Data) {
        let _ = data; // disable unused variable warning
    }

    fn from(data: &Data) -> Self where Self: Default {
        let mut e = Self::default();
        e.set_data(data);
        e
    }
}
