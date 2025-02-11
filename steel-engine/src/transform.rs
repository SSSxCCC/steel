use crate::{edit::Edit, hierarchy::Parent};
use glam::{Affine2, Affine3A, Quat, Vec3, Vec3Swizzles};
use shipyard::{Component, EntityId, Get};
use std::collections::HashMap;
use steel_common::data::{Data, Limit, Value};

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

    /// Get the final model matrix of an entity.
    /// Returns None if entity and ancestors do not have any Transform component.
    /// # Example
    /// ```rust
    /// fn my_system(entities: EntitiesView, transforms: View<Transform>, parents: View<Parent>) {
    ///     let mut cache = Some(HashMap::new());
    ///     for e in entities.iter() {
    ///         // Affine3A::default() returns Affine3A::IDENTITY
    ///         let model = Transform::entity_final_model(e, &parents, &transforms, &mut cache).unwrap_or_default();
    ///     }
    /// }
    /// ```
    pub fn entity_final_model<'a>(
        entity: EntityId,
        parents: impl Get<Out = &'a Parent> + Copy, // &View<Parent> or &ViewMut<Parent>
        transforms: impl Get<Out = &'a Transform> + Copy, // &View<Transform> or &ViewMut<Transform>
        cache: &mut Option<HashMap<EntityId, Option<Affine3A>>>,
    ) -> Option<Affine3A> {
        if entity == EntityId::dead() {
            None
        } else {
            if let Some(cache) = cache {
                if let Some(cache_model) = cache.get(&entity) {
                    return *cache_model;
                }
            }

            let parent = parents.get(entity).map(|p| **p).unwrap_or_default();
            let final_model = if let Some(parent_final_model) =
                Self::entity_final_model(parent, parents, transforms, cache)
            {
                if let Ok(transform) = transforms.get(entity) {
                    Some(parent_final_model * transform.model())
                } else {
                    Some(parent_final_model)
                }
            } else {
                if let Ok(transform) = transforms.get(entity) {
                    Some(transform.model())
                } else {
                    None
                }
            };

            if let Some(cache) = cache {
                cache.insert(entity, final_model);
            }
            final_model
        }
    }

    /// Get the 2d model matrix of this transform.
    pub fn model_2d(&self) -> Affine2 {
        Affine2::from_scale_angle_translation(
            self.scale.xy(),
            self.rotation.to_scaled_axis().z,
            self.position.xy(),
        )
    }

    /// Get the final 2d model matrix of an entity.
    /// Returns None if entity and ancestors do not have any Transform component.
    /// # Example
    /// ```rust
    /// fn my_system(entities: EntitiesView, transforms: View<Transform>, parents: View<Parent>) {
    ///     let mut cache = Some(HashMap::new());
    ///     for e in entities.iter() {
    ///         // Affine2::default() returns Affine2::IDENTITY
    ///         let model = Transform::entity_final_model_2d(e, &parents, &transforms, &mut cache).unwrap_or_default();
    ///     }
    /// }
    /// ```
    pub fn entity_final_model_2d<'a>(
        entity: EntityId,
        parents: impl Get<Out = &'a Parent> + Copy, // &View<Parent> or &ViewMut<Parent>
        transforms: impl Get<Out = &'a Transform> + Copy, // &View<Transform> or &ViewMut<Transform>
        cache: &mut Option<HashMap<EntityId, Option<Affine2>>>,
    ) -> Option<Affine2> {
        if entity == EntityId::dead() {
            None
        } else {
            if let Some(cache) = cache {
                if let Some(cache_model) = cache.get(&entity) {
                    return *cache_model;
                }
            }

            let parent = parents.get(entity).map(|p| **p).unwrap_or_default();
            let final_model = if let Some(parent_final_model) =
                Self::entity_final_model_2d(parent, parents, transforms, cache)
            {
                if let Ok(transform) = transforms.get(entity) {
                    Some(parent_final_model * transform.model_2d())
                } else {
                    Some(parent_final_model)
                }
            } else {
                if let Ok(transform) = transforms.get(entity) {
                    Some(transform.model_2d())
                } else {
                    None
                }
            };

            if let Some(cache) = cache {
                cache.insert(entity, final_model);
            }
            final_model
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Default::default(),
            rotation: Default::default(),
            scale: Vec3::ONE,
        }
    }
}

impl Edit for Transform {
    fn name() -> &'static str {
        "Transform"
    }

    fn get_data(&self) -> Data {
        Data::new()
            .insert("position", Value::Vec3(self.position))
            .insert_with_limit(
                "rotation",
                Value::Vec3(self.rotation.to_scaled_axis()),
                Limit::Float32Rotation,
            )
            .insert("scale", Value::Vec3(self.scale))
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Vec3(v)) = data.get("position") {
            self.position = *v
        }
        if let Some(Value::Vec3(v)) = data.get("rotation") {
            self.rotation = Quat::from_scaled_axis(*v)
        }
        if let Some(Value::Vec3(v)) = data.get("scale") {
            self.scale = *v
        }
    }
}
