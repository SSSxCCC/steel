pub use steel_proc::Edit;

use steel_common::data::Data;

/// You can impl Edit for a Component or Unique so that they can be edited in steel-editor.
/// Example:
/// ```rust
/// use steel::{edit::Edit, data::{Value, Limit}};
/// use shipyard::Component;
///
/// #[derive(Component, Edit, Default)]
/// pub struct TestComponent {
///     pub boo: bool,
///     #[edit(name = "int_renamed", limit = "Limit::Int32Range(0..=3)")]
///     pub int: i32,
///     #[edit(limit = "Limit::ReadOnly", name = "f32_renamed")]
///     pub float: f32,
///     pub string: String,
///     pub vec3: glam::Vec3,
///     pub other: Other, // not supported field is ignored
/// }
/// ```
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
