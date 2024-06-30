use glam::{Affine2, Affine3A, Quat, Vec2, Vec3, Vec3Swizzles};
use shipyard::{Component, EntityId, Get};
use steel_common::data::{Data, Limit, Value};
use crate::{edit::Edit, hierarchy::Child};

/// The Transform component defines position, rotation, and scale of an entity.
/// If this entity has any ancestor which also has Transform component,
/// the transform values are relative to this entity's parent.
#[derive(Component, Debug)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    /// Get the model matrix of this transform.
    pub fn model(&self) -> Affine3A {
        Affine3A::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    /// Get the final model matrix of this transform.
    /// # Example
    /// ```rust
    /// fn my_system(transforms: View<Transform>, children: View<Child>) {
    ///     for (transform, child) in (&transforms, &children).iter() {
    ///         let model = transform.final_model(child, &children, &transforms);
    ///     }
    /// }
    /// ```
    pub fn final_model<'a>(&self, child: &Child,
            children: impl Get<Out = &'a Child> + Copy, // &View<Child> or &ViewMut<Child>
            transforms: impl Get<Out = &'a Transform> + Copy) // &View<Transform> or &ViewMut<Transform>
            -> Affine3A {
        let (final_position, final_rotation, final_scale) =
            self.final_position_rotation_scale(child, children, transforms);
        Affine3A::from_scale_rotation_translation(final_scale, final_rotation, final_position)
    }

    /// Get the final model matrix of entity eid.
    /// Returns None if eid and ancestors do not have any Transform component.
    /// # Example
    /// ```rust
    /// fn my_system(entities: EntitiesView, transforms: View<Transform>, children: View<Child>) {
    ///     for eid in entities.iter() {
    ///         // Affine3A::default() returns Affine3A::IDENTITY
    ///         let model = Transform::entity_final_model(eid, &children, &transforms).unwrap_or_default();
    ///     }
    /// }
    /// ```
    pub fn entity_final_model<'a>(eid: EntityId,
            children: impl Get<Out = &'a Child> + Copy, // &View<Child> or &ViewMut<Child>
            transforms: impl Get<Out = &'a Transform> + Copy) // &View<Transform> or &ViewMut<Transform>
            -> Option<Affine3A> {
        if let Some((final_position, final_rotation, final_scale)) =
                Self::entity_final_position_rotation_scale(eid, children, transforms) {
            Some(Affine3A::from_scale_rotation_translation(final_scale, final_rotation, final_position))
        } else {
            None
        }
    }

    /// Get the final position, rotation, scale of this transform.
    /// * final_position = parent_final_position + self.position
    /// * final_rotation = parent_final_rotation.mul_quat(self.rotation)
    /// * final_scale = parent_final_scale * self.scale
    pub fn final_position_rotation_scale<'a>(&self, child: &Child,
            children: impl Get<Out = &'a Child> + Copy, // &View<Child> or &ViewMut<Child>
            transforms: impl Get<Out = &'a Transform> + Copy) // &View<Transform> or &ViewMut<Transform>
            -> (Vec3, Quat, Vec3) {
        if let Some((parent_final_position, parent_final_rotation, parent_final_scale)) =
                Self::entity_final_position_rotation_scale(child.parent(), children, transforms) {
            (
                parent_final_position + self.position,
                parent_final_rotation.mul_quat(self.rotation),
                parent_final_scale * self.scale,
            )
        } else {
            (self.position, self.rotation, self.scale)
        }
    }

    /// Get the final position, rotation, scale of entity eid.
    /// * final_position = parent_final_position + self.position
    /// * final_rotation = parent_final_rotation.mul_quat(self.rotation)
    /// * final_scale = parent_final_scale * self.scale
    pub fn entity_final_position_rotation_scale<'a>(eid: EntityId,
            children: impl Get<Out = &'a Child> + Copy, // &View<Child> or &ViewMut<Child>
            transforms: impl Get<Out = &'a Transform> + Copy) // &View<Transform> or &ViewMut<Transform>
            -> Option<(Vec3, Quat, Vec3)> {
        if eid == EntityId::dead() {
            None
        } else {
            let child = children.get(eid).expect(format!("Missing Child component in entity: {eid:?}").as_str());
            if let Some((parent_final_position, parent_final_rotation, parent_final_scale)) =
                    Self::entity_final_position_rotation_scale(child.parent(), children, transforms) {
                if let Ok(transform) = transforms.get(eid) {
                    Some((
                        parent_final_position + transform.position,
                        parent_final_rotation.mul_quat(transform.rotation),
                        parent_final_scale * transform.scale,
                    ))
                } else {
                    Some((parent_final_position, parent_final_rotation, parent_final_scale))
                }
            } else {
                if let Ok(transform) = transforms.get(eid) {
                    Some((transform.position, transform.rotation, transform.scale))
                } else {
                    None
                }
            }
        }
    }

    /// Get the 2d model matrix of this transform.
    pub fn model_2d(&self) -> Affine2 {
        Affine2::from_scale_angle_translation(self.scale.xy(), self.rotation.to_scaled_axis().z, self.position.xy())
    }

    /// Get the 2d final model matrix of this transform.
    /// # Example
    /// ```rust
    /// fn my_system(transforms: View<Transform>, children: View<Child>) {
    ///     for (transform, child) in (&transforms, &children).iter() {
    ///         let model = transform.final_model_2d(child, &children, &transforms);
    ///     }
    /// }
    /// ```
    pub fn final_model_2d<'a>(&self, child: &Child,
            children: impl Get<Out = &'a Child> + Copy, // &View<Child> or &ViewMut<Child>
            transforms: impl Get<Out = &'a Transform> + Copy) // &View<Transform> or &ViewMut<Transform>
            -> Affine2 {
        let (final_position, final_rotation, final_scale) =
            self.final_position_rotation_scale_2d(child, children, transforms);
        Affine2::from_scale_angle_translation(final_scale, final_rotation, final_position)
    }

    /// Get the final 2d model matrix of entity eid.
    /// Returns None if eid and ancestors do not have any Transform component.
    /// # Example
    /// ```rust
    /// fn my_system(entities: EntitiesView, transforms: View<Transform>, children: View<Child>) {
    ///     for eid in entities.iter() {
    ///         // Affine2::default() returns Affine2::IDENTITY
    ///         let model = Transform::entity_final_model_2d(eid, &children, &transforms).unwrap_or_default();
    ///     }
    /// }
    /// ```
    pub fn entity_final_model_2d<'a>(eid: EntityId,
            children: impl Get<Out = &'a Child> + Copy, // &View<Child> or &ViewMut<Child>
            transforms: impl Get<Out = &'a Transform> + Copy) // &View<Transform> or &ViewMut<Transform>
            -> Option<Affine2> {
        if let Some((final_position, final_rotation, final_scale)) =
                Self::entity_final_position_rotation_scale_2d(eid, children, transforms) {
            Some(Affine2::from_scale_angle_translation(final_scale, final_rotation, final_position))
        } else {
            None
        }
    }

    /// Get the 2d final position, rotation, scale of this transform.
    /// * final_position = parent_final_position + self.position
    /// * final_rotation = parent_final_rotation + self.rotation
    /// * final_scale = parent_final_scale * self.scale
    pub fn final_position_rotation_scale_2d<'a>(&self, child: &Child,
            children: impl Get<Out = &'a Child> + Copy, // &View<Child> or &ViewMut<Child>
            transforms: impl Get<Out = &'a Transform> + Copy) // &View<Transform> or &ViewMut<Transform>
            -> (Vec2, f32, Vec2) {
        if let Some((parent_final_position, parent_final_rotation, parent_final_scale)) =
                Self::entity_final_position_rotation_scale_2d(child.parent(), children, transforms) {
            (
                parent_final_position + self.position.xy(),
                parent_final_rotation + self.rotation.to_scaled_axis().z,
                parent_final_scale * self.scale.xy(),
            )
        } else {
            (self.position.xy(), self.rotation.to_scaled_axis().z, self.scale.xy())
        }
    }

    /// Get the 2d final position, rotation, scale of entity eid.
    /// * final_position = parent_final_position + self.position
    /// * final_rotation = parent_final_rotation + self.rotation
    /// * final_scale = parent_final_scale * self.scale
    pub fn entity_final_position_rotation_scale_2d<'a>(eid: EntityId,
            children: impl Get<Out = &'a Child> + Copy, // &View<Child> or &ViewMut<Child>
            transforms: impl Get<Out = &'a Transform> + Copy) // &View<Transform> or &ViewMut<Transform>
            -> Option<(Vec2, f32, Vec2)> {
        if eid == EntityId::dead() {
            None
        } else {
            let child = children.get(eid).expect(format!("Missing Child component in entity: {eid:?}").as_str());
            if let Some((parent_final_position, parent_final_rotation, parent_final_scale)) =
                    Self::entity_final_position_rotation_scale_2d(child.parent(), children, transforms) {
                if let Ok(transform) = transforms.get(eid) {
                    Some((
                        parent_final_position + transform.position.xy(),
                        parent_final_rotation + transform.rotation.to_scaled_axis().z,
                        parent_final_scale * transform.scale.xy(),
                    ))
                } else {
                    Some((parent_final_position, parent_final_rotation, parent_final_scale))
                }
            } else {
                if let Ok(transform) = transforms.get(eid) {
                    Some((transform.position.xy(), transform.rotation.to_scaled_axis().z, transform.scale.xy()))
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
