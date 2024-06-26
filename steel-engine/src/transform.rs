use glam::{Mat4, Quat, Vec2, Vec3, Vec3Swizzles};
use shipyard::{Component, EntityId, Get};
use steel_common::data::{Data, Limit, Value};
use crate::{edit::Edit, hierarchy::Child};

/// The Transform component defines position, rotation, and scale of an entity.
#[derive(Component, Debug)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    /// Get the model matrix of this transform.
    pub fn model(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    /// Get the final model matrix of this transform, which is parent_final_model * self.model().
    /// # Example
    /// ```rust
    /// fn my_system(transforms: View<Transform>, children: View<Child>) {
    ///     for (transform, child) in (&transforms, &children).iter() {
    ///         let model = transform.final_model(child, &children, &transforms);
    ///     }
    /// }
    /// ```
    pub fn final_model<'a>(&self, child: &Child, children: impl Get<Out = &'a Child> + Copy, // &View<Child> or &ViewMut<Child>
            transforms: impl Get<Out = &'a Transform> + Copy) -> Mat4 { // &View<Transform> or &ViewMut<Transform>
        if let Some(parent_final_model) = Self::entity_final_model(child.parent(), children, transforms) {
            parent_final_model * self.model()
        } else {
            self.model()
        }
    }

    /// Get the final model matrix of entity eid, which is parent_final_model * eid_model.
    /// Returns None if eid and ancestors do not have any Transform component.
    /// # Example
    /// ```rust
    /// fn my_system(entities: EntitiesView, transforms: View<Transform>, children: View<Child>) {
    ///     for eid in entities.iter() {
    ///         // Mat4::default() returns Mat4::IDENTITY
    ///         let model = Transform::entity_final_model(eid, &children, &transforms).unwrap_or_default();
    ///     }
    /// }
    /// ```
    pub fn entity_final_model<'a>(eid: EntityId, children: impl Get<Out = &'a Child> + Copy, // &View<Child> or &ViewMut<Child>
            transforms: impl Get<Out = &'a Transform> + Copy) -> Option<Mat4> { // &View<Transform> or &ViewMut<Transform>
        if eid == EntityId::dead() {
            None
        } else {
            let child = children.get(eid).expect(format!("Missing Child component in entity: {eid:?}").as_str());
            if let Some(parent_final_model) = Self::entity_final_model(child.parent(), children, transforms) {
                if let Ok(transform) = transforms.get(eid) {
                    Some(parent_final_model * transform.model())
                } else {
                    Some(parent_final_model)
                }
            } else {
                if let Ok(transform) = transforms.get(eid) {
                    Some(transform.model())
                } else {
                    None
                }
            }
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self { position: Default::default(), rotation: Default::default(), scale: Vec3::ONE }
    }
}

impl Edit for Transform {
    fn name() -> &'static str { "Transform" }

    fn get_data(&self) -> Data {
        Data::new().insert("position", Value::Vec3(self.position))
            .insert_with_limit("rotation", Value::Vec3(self.rotation.to_scaled_axis()), Limit::Float32Rotation)
            .insert("scale", Value::Vec3(self.scale))
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Vec3(v)) = data.get("position") { self.position = *v }
        if let Some(Value::Vec3(v)) = data.get("rotation") { self.rotation = Quat::from_scaled_axis(*v) }
        if let Some(Value::Vec3(v)) = data.get("scale") { self.scale = *v }
    }
}

/// Helper functions to get some 2d data with Mat4.
pub trait Mat4Transform2D {
    /// Extracts 2d scale, 2d rotation and 2d translation from Mat4.
    fn to_2d_scale_rotation_translation(&self) -> (Vec2, f32, Vec2);
}

impl Mat4Transform2D for Mat4 {
    fn to_2d_scale_rotation_translation(&self) -> (Vec2, f32, Vec2) {
        let (scale, rotation, translation) = self.to_scale_rotation_translation();
        (scale.xy(), rotation.to_scaled_axis().z, translation.xy())
    }
}
