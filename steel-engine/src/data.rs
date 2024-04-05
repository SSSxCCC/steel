pub use steel_common::data::*;

use indexmap::IndexMap;
use shipyard::{track::{All, Insertion, Modification, Removal, Untracked}, EntityId, IntoIter, IntoWithId, View, ViewMut, World};
use crate::{camera::Camera, edit::Edit, entityinfo::EntityInfo, physics2d::{Collider2D, RigidBody2D}, render::renderer2d::Renderer2D, transform::Transform};

/// ComponentFn stores many functions of a component, like component create and destroy functions.
/// These functions are used by steel-editor so that we can use steel-editor ui to edit this component.
pub struct ComponentFn {
    pub create: fn(&mut World, EntityId),
    pub create_with_data: fn(&mut World, EntityId, &ComponentData),
    pub destroy: fn(&mut World, EntityId),
    pub save_to_data: fn(&mut WorldData, &World),
    pub load_from_data: fn(&mut World, &WorldData),
}

/// key is component name
pub type ComponentFns = IndexMap<&'static str, ComponentFn>;

impl ComponentFn {
    pub fn with_core_components() -> ComponentFns {
        let mut component_fns = ComponentFns::new();
        Self::register::<EntityInfo>(&mut component_fns);
        Self::register::<Transform>(&mut component_fns);
        Self::register::<Camera>(&mut component_fns);
        Self::register_track_all::<RigidBody2D>(&mut component_fns);
        Self::register_track_all::<Collider2D>(&mut component_fns);
        Self::register::<Renderer2D>(&mut component_fns);
        component_fns
    }

    pub fn register<T: Edit<Tracking = Untracked> + Send + Sync>(component_fns: &mut ComponentFns) {
        component_fns.insert(T::name(), ComponentFn {
            create: Self::create_fn::<T>,
            create_with_data: Self::create_with_data_fn::<T>,
            destroy: Self::destroy_fn::<T>,
            save_to_data: Self::save_to_data_fn::<T>,
            load_from_data: Self::load_from_data_untracked_fn::<T>,
        });
    }

    pub fn register_track_insertion<T: Edit<Tracking = Insertion> + Send + Sync>(component_fns: &mut ComponentFns) {
        component_fns.insert(T::name(), ComponentFn {
            create: Self::create_fn::<T>,
            create_with_data: Self::create_with_data_fn::<T>,
            destroy: Self::destroy_fn::<T>,
            save_to_data: Self::save_to_data_fn::<T>,
            load_from_data: Self::load_from_data_track_insertion_fn::<T>,
        });
    }

    pub fn register_track_modification<T: Edit<Tracking = Modification> + Send + Sync>(component_fns: &mut ComponentFns) {
        component_fns.insert(T::name(), ComponentFn {
            create: Self::create_fn::<T>,
            create_with_data: Self::create_with_data_fn::<T>,
            destroy: Self::destroy_fn::<T>,
            save_to_data: Self::save_to_data_fn::<T>,
            load_from_data: Self::load_from_data_track_modification_fn::<T>,
        });
    }

    pub fn register_track_removal<T: Edit<Tracking = Removal> + Send + Sync>(component_fns: &mut ComponentFns) {
        component_fns.insert(T::name(), ComponentFn {
            create: Self::create_fn::<T>,
            create_with_data: Self::create_with_data_fn::<T>,
            destroy: Self::destroy_fn::<T>,
            save_to_data: Self::save_to_data_fn::<T>,
            load_from_data: Self::load_from_data_track_removal_fn::<T>,
        });
    }

    pub fn register_track_all<T: Edit<Tracking = All> + Send + Sync>(component_fns: &mut ComponentFns) {
        component_fns.insert(T::name(), ComponentFn {
            create: Self::create_fn::<T>,
            create_with_data: Self::create_with_data_fn::<T>,
            destroy: Self::destroy_fn::<T>,
            save_to_data: Self::save_to_data_fn::<T>,
            load_from_data: Self::load_from_data_track_all_fn::<T>,
        });
    }

    fn create_fn<T: Edit + Send + Sync>(world: &mut World, entity: EntityId) {
        world.add_component(entity, (T::default(),))
    }

    fn create_with_data_fn<T: Edit + Send + Sync>(world: &mut World, entity: EntityId, data: &ComponentData) {
        world.add_component(entity, (T::from(data),))
    }

    fn destroy_fn<T: Edit + Send + Sync>(world: &mut World, entity: EntityId) {
        world.delete_component::<T>(entity)
    }

    fn save_to_data_fn<T: Edit + Send + Sync>(world_data: &mut WorldData, world: &World) {
        world.run(|c: View<T>| {
            for (e, c) in c.iter().with_id() {
                let entity_data = world_data.entities.entry(e).or_insert_with(|| EntityData::new());
                entity_data.components.insert(T::name().into(), c.get_data());
            }
        })
    }

    /// Currently we must write different generic functions for different tracking type, see https://github.com/leudz/shipyard/issues/157
    /// TODO: find a way to write only one generic function to cover all tracking type
    fn load_from_data_untracked_fn<T: Edit<Tracking = Untracked> + Send + Sync>(world: &mut World, world_data: &WorldData) {
        world.run(|mut t: ViewMut<T>| {
            for (id, t) in (&mut t).iter().with_id() {
                Self::_load_from_data(id, t, world_data);
            }
        })
    }

    fn load_from_data_track_insertion_fn<T: Edit<Tracking = Insertion> + Send + Sync>(world: &mut World, world_data: &WorldData) {
        world.run(|mut t: ViewMut<T>| {
            for (id, t) in (&mut t).iter().with_id() {
                Self::_load_from_data(id, t, world_data);
            }
        })
    }

    fn load_from_data_track_modification_fn<T: Edit<Tracking = Modification> + Send + Sync>(world: &mut World, world_data: &WorldData) {
        world.run(|mut t: ViewMut<T>| {
            for (id, mut t) in (&mut t).iter().with_id() {
                Self::_load_from_data(id, t.as_mut(), world_data);
            }
        })
    }

    fn load_from_data_track_removal_fn<T: Edit<Tracking = Removal> + Send + Sync>(world: &mut World, world_data: &WorldData) {
        world.run(|mut t: ViewMut<T>| {
            for (id, t) in (&mut t).iter().with_id() {
                Self::_load_from_data(id, t, world_data);
            }
        })
    }

    fn load_from_data_track_all_fn<T: Edit<Tracking = All> + Send + Sync>(world: &mut World, world_data: &WorldData) {
        world.run(|mut t: ViewMut<T>| {
            for (id, mut t) in (&mut t).iter().with_id() {
                Self::_load_from_data(id, t.as_mut(), world_data);
            }
        })
    }

    fn _load_from_data<T: Edit>(id: EntityId, t: &mut T, world_data: &WorldData) {
        if let Some(entity_data) = world_data.entities.get(&id) {
            if let Some(component_data) = entity_data.components.get(T::name()) {
                t.set_data(component_data);
            }
        }
    }
}
