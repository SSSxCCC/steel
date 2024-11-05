use crate::{edit::Edit, hierarchy::Parent};
use glam::{Affine2, Affine3A, Quat, Vec2, Vec3, Vec3Swizzles};
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

    /// Get the model matrix without scale of this transform.
    pub fn model_without_scale(&self) -> Affine3A {
        Affine3A::from_rotation_translation(self.rotation, self.position)
    }

    /// Get the final model matrix of entity eid.
    /// Returns None if eid and ancestors do not have any Transform component.
    /// # Example
    /// ```rust
    /// fn my_system(entities: EntitiesView, transforms: View<Transform>, parents: View<Parent>) {
    ///     let mut model_cache = Some(HashMap::new());
    ///     let mut scale_cache = Some(HashMap::new());
    ///     for eid in entities.iter() {
    ///         // Affine3A::default() returns Affine3A::IDENTITY
    ///         let model = Transform::entity_final_model(eid, &parents, &transforms, &mut model_cache, &mut scale_cache).unwrap_or_default();
    ///     }
    /// }
    /// ```
    pub fn entity_final_model<'a>(
        eid: EntityId,
        parents: impl Get<Out = &'a Parent> + Copy, // &View<Parent> or &ViewMut<Parent>
        transforms: impl Get<Out = &'a Transform> + Copy, // &View<Transform> or &ViewMut<Transform>
        model_cache: &mut Option<HashMap<EntityId, Option<Affine3A>>>,
        scale_cache: &mut Option<HashMap<EntityId, Option<Vec3>>>,
    ) -> Option<Affine3A> {
        if let (Some(final_scale), Some(final_model_without_scale)) = (
            Self::entity_final_scale(eid, parents, transforms, scale_cache),
            Self::entity_final_model_without_scale(eid, parents, transforms, model_cache),
        ) {
            let final_scale = Affine3A::from_scale(final_scale);
            Some(final_model_without_scale * final_scale)
        } else {
            None
        }
    }

    /// Get the final model matrix without scale of entity eid,
    /// which is parent_final_model_without_scale * self.model_without_scale.
    /// Returns None if eid and ancestors do not have any Transform component.
    pub fn entity_final_model_without_scale<'a>(
        eid: EntityId,
        parents: impl Get<Out = &'a Parent> + Copy, // &View<Parent> or &ViewMut<Parent>
        transforms: impl Get<Out = &'a Transform> + Copy, // &View<Transform> or &ViewMut<Transform>
        cache: &mut Option<HashMap<EntityId, Option<Affine3A>>>,
    ) -> Option<Affine3A> {
        if eid == EntityId::dead() {
            None
        } else {
            if let Some(cache) = cache {
                if let Some(cache_model) = cache.get(&eid) {
                    return *cache_model;
                }
            }

            let parent = parents.get(eid).map(|p| **p).unwrap_or_default();
            let final_model = if let Some(parent_final_model_without_scale) =
                Self::entity_final_model_without_scale(parent, parents, transforms, cache)
            {
                if let Ok(transform) = transforms.get(eid) {
                    Some(parent_final_model_without_scale * transform.model_without_scale())
                } else {
                    Some(parent_final_model_without_scale)
                }
            } else {
                if let Ok(transform) = transforms.get(eid) {
                    Some(transform.model_without_scale())
                } else {
                    None
                }
            };

            if let Some(cache) = cache {
                cache.insert(eid, final_model);
            }
            final_model
        }
    }

    /// Get the final scale of entity eid, which is parent_final_scale * self.scale.
    /// Returns None if eid and ancestors do not have any Transform component.
    pub fn entity_final_scale<'a>(
        eid: EntityId,
        parents: impl Get<Out = &'a Parent> + Copy, // &View<Parent> or &ViewMut<Parent>
        transforms: impl Get<Out = &'a Transform> + Copy, // &View<Transform> or &ViewMut<Transform>
        cache: &mut Option<HashMap<EntityId, Option<Vec3>>>,
    ) -> Option<Vec3> {
        if eid == EntityId::dead() {
            None
        } else {
            if let Some(cache) = cache {
                if let Some(cache_scale) = cache.get(&eid) {
                    return *cache_scale;
                }
            }

            let parent = parents.get(eid).map(|p| **p).unwrap_or_default();
            let final_scale = if let Some(parent_final_scale) =
                Self::entity_final_scale(parent, parents, transforms, cache)
            {
                if let Ok(transform) = transforms.get(eid) {
                    Some(parent_final_scale * transform.scale)
                } else {
                    Some(parent_final_scale)
                }
            } else {
                if let Ok(transform) = transforms.get(eid) {
                    Some(transform.scale)
                } else {
                    None
                }
            };

            if let Some(cache) = cache {
                cache.insert(eid, final_scale);
            }
            final_scale
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

    /// Get the 2d model matrix without scale of this transform.
    pub fn model_without_scale_2d(&self) -> Affine2 {
        Affine2::from_angle_translation(self.rotation.to_scaled_axis().z, self.position.xy())
    }

    /// Get the final 2d model matrix of entity eid.
    /// Returns None if eid and ancestors do not have any Transform component.
    /// # Example
    /// ```rust
    /// fn my_system(entities: EntitiesView, transforms: View<Transform>, parents: View<Parent>) {
    ///     let mut model_cache = Some(HashMap::new());
    ///     let mut scale_cache = Some(HashMap::new());
    ///     for eid in entities.iter() {
    ///         // Affine2::default() returns Affine2::IDENTITY
    ///         let model = Transform::entity_final_model_2d(eid, &parents, &transforms, &mut model_cache, &mut scale_cache).unwrap_or_default();
    ///     }
    /// }
    /// ```
    pub fn entity_final_model_2d<'a>(
        eid: EntityId,
        parents: impl Get<Out = &'a Parent> + Copy, // &View<Parent> or &ViewMut<Parent>
        transforms: impl Get<Out = &'a Transform> + Copy, // &View<Transform> or &ViewMut<Transform>
        model_cache: &mut Option<HashMap<EntityId, Option<Affine2>>>,
        scale_cache: &mut Option<HashMap<EntityId, Option<Vec2>>>,
    ) -> Option<Affine2> {
        if let (Some(final_scale), Some(final_model_without_scale)) = (
            Self::entity_final_scale_2d(eid, parents, transforms, scale_cache),
            Self::entity_final_model_without_scale_2d(eid, parents, transforms, model_cache),
        ) {
            let final_scale = Affine2::from_scale(final_scale);
            Some(final_model_without_scale * final_scale)
        } else {
            None
        }
    }

    /// Get the final 2d model matrix without scale of entity eid,
    /// which is parent_final_model_without_scale * self.model_without_scale.
    /// Returns None if eid and ancestors do not have any Transform component.
    pub fn entity_final_model_without_scale_2d<'a>(
        eid: EntityId,
        parents: impl Get<Out = &'a Parent> + Copy, // &View<Parent> or &ViewMut<Parent>
        transforms: impl Get<Out = &'a Transform> + Copy, // &View<Transform> or &ViewMut<Transform>
        cache: &mut Option<HashMap<EntityId, Option<Affine2>>>,
    ) -> Option<Affine2> {
        if eid == EntityId::dead() {
            None
        } else {
            if let Some(cache) = cache {
                if let Some(cache_model) = cache.get(&eid) {
                    return *cache_model;
                }
            }

            let parent = parents.get(eid).map(|p| **p).unwrap_or_default();
            let final_model = if let Some(parent_final_model_without_scale) =
                Self::entity_final_model_without_scale_2d(parent, parents, transforms, cache)
            {
                if let Ok(transform) = transforms.get(eid) {
                    Some(parent_final_model_without_scale * transform.model_without_scale_2d())
                } else {
                    Some(parent_final_model_without_scale)
                }
            } else {
                if let Ok(transform) = transforms.get(eid) {
                    Some(transform.model_without_scale_2d())
                } else {
                    None
                }
            };

            if let Some(cache) = cache {
                cache.insert(eid, final_model);
            }
            final_model
        }
    }

    /// Get the final 2d scale of entity eid, which is parent_final_scale * self.scale.
    /// Returns None if eid and ancestors do not have any Transform component.
    pub fn entity_final_scale_2d<'a>(
        eid: EntityId,
        parents: impl Get<Out = &'a Parent> + Copy, // &View<Parent> or &ViewMut<Parent>
        transforms: impl Get<Out = &'a Transform> + Copy,
        cache: &mut Option<HashMap<EntityId, Option<Vec2>>>,
    ) -> Option<Vec2> {
        if eid == EntityId::dead() {
            None
        } else {
            if let Some(cache) = cache {
                if let Some(cache_scale) = cache.get(&eid) {
                    return *cache_scale;
                }
            }

            let parent = parents.get(eid).map(|p| **p).unwrap_or_default();
            let final_scale = if let Some(parent_final_scale) =
                Self::entity_final_scale_2d(parent, parents, transforms, cache)
            {
                if let Ok(transform) = transforms.get(eid) {
                    Some(parent_final_scale * transform.scale.xy())
                } else {
                    Some(parent_final_scale)
                }
            } else {
                if let Ok(transform) = transforms.get(eid) {
                    Some(transform.scale.xy())
                } else {
                    None
                }
            };

            if let Some(cache) = cache {
                cache.insert(eid, final_scale);
            }
            final_scale
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
