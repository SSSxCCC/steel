pub use steel_proc::Edit;

use steel_common::data::Data;

// TODO: prevent data clone between ecs world and editor.

/// You can impl Edit for a Component or Unique so that they can be edited in steel-editor.
/// # Examples
/// ## Use Edit derive macro
/// ```rust
/// use steel::{edit::Edit, data::{Data, Value, Limit}};
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
/// ## Manually impl Edit
/// ```rust
/// use steel::{edit::Edit, data::{Data, Value, Limit}};
/// use shipyard::Component;
///
/// #[derive(Component, Default)]
/// pub struct TestComponent {
///     pub boo: bool,
///     pub int: i32,
///     pub float: f32,
///     pub string: String,
///     pub vec3: glam::Vec3,
///     pub other: Other, // not supported field is ignored
/// }
///
/// impl Edit for TestComponent {
///     fn name() -> &'static str { "TestComponent" }
///
///     fn get_data(&self) -> Data {
///         Data::new().insert("boo", Value::Bool(self.boo))
///             .insert_with_limit("int_renamed", Value::Int32(self.int), Limit::Int32Range(0..=3))
///             .insert_with_limit("f32_renamed", Value::Float32(self.float), Limit::ReadOnly)
///             .insert("string", Value::String(self.string.clone()))
///             .insert("vec3", Value::Vec3(self.vec3))
///     }
///
///     fn set_data(&mut self, data: &Data) {
///         if let Some(Value::Bool(v)) = data.get("boo") { self.boo = *v }
///         if let Some(Value::Int32(v)) = data.get("int_renamed") { self.int = *v }
///         if let Some(Value::String(v)) = data.get("string") { self.string = v.clone() }
///         if let Some(Value::Vec3(v)) = data.get("vec3") { self.vec3 = *v }
///     }
/// }
/// ```
/// Value names that starts with "unnamed-" will not display in the editor. So you can use
/// "unnamed-0" and "unnamed-1" as tuple struct field names when manually impl Edit.
/// Edit derive macro uses "unnamed-i" as value name for tuple struct fields.
pub trait Edit {
    /// The name of this component or unique.
    fn name() -> &'static str;

    /// Create a Data struct from self.
    fn get_data(&self) -> Data {
        Data::new()
    }

    /// Modify self according to a Data struct. This function is called every frame in editor
    /// to make user able to modify component or unique in editor. You should omit read-only
    /// values so that they are never modified.
    fn set_data(&mut self, data: &Data) {
        let _ = data; // disable unused variable warning
    }

    /// Modify self according to a Data struct. This function is usually called during scene loading.
    /// By default, this calls Edit::set_data so that you do not need to implement this for the
    /// component/unique which has the same behaviour between scene loading and editor modify.
    fn load_data(&mut self, data: &Data) {
        self.set_data(data);
    }

    /// Create Self from a Data struct. This function is usually called during scene loading.
    fn from_data(data: &Data) -> Self
    where
        Self: Default,
    {
        let mut e = Self::default();
        e.load_data(data);
        e
    }
}
