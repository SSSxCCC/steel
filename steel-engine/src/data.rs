pub use steel_common::data::*;

use indexmap::IndexMap;
use shipyard::{track::{All, Deletion, Insertion, Modification, Removal, Untracked}, Component, EntityId, IntoIter, IntoWithId, Unique, UniqueView, UniqueViewMut, View, ViewMut, World};
use crate::edit::Edit;

/// ComponentFn stores many functions of a component, like component create and destroy functions.
/// These functions are used by steel-editor so that we can use steel-editor ui to edit this component.
pub struct ComponentFn {
    pub create: fn(&mut World, EntityId),
    pub create_with_data: fn(&mut World, EntityId, &Data),
    pub destroy: fn(&mut World, EntityId),
    pub save_to_data: fn(&mut WorldData, &World),
    pub load_from_data: fn(&mut World, &WorldData),
}

/// A map of ComponentFn, key is component name.
pub struct ComponentRegistry(IndexMap<&'static str, ComponentFn>);

impl std::ops::Deref for ComponentRegistry {
    type Target = IndexMap<&'static str, ComponentFn>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ComponentRegistry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ComponentRegistry {
    /// Create a new ComponentRegistry.
    pub fn new() -> Self {
        ComponentRegistry(IndexMap::new())
    }

    /// Insert a type of Component<Tracking = Untracked> to ComponentRegistry.
    pub fn register<C: Component<Tracking = Untracked> + Edit + Default + Send + Sync>(&mut self) {
        self.insert(C::name(), ComponentFn {
            create: Self::create_fn::<C>,
            create_with_data: Self::create_with_data_fn::<C>,
            destroy: Self::destroy_fn::<C>,
            save_to_data: Self::save_to_data_fn::<C>,
            load_from_data: Self::load_from_data_untracked_fn::<C>,
        });
    }

    /// Insert a type of Component<Tracking = Insertion> to ComponentRegistry.
    pub fn register_track_insertion<C: Component<Tracking = Insertion> + Edit + Default + Send + Sync>(&mut self) {
        self.insert(C::name(), ComponentFn {
            create: Self::create_fn::<C>,
            create_with_data: Self::create_with_data_fn::<C>,
            destroy: Self::destroy_fn::<C>,
            save_to_data: Self::save_to_data_fn::<C>,
            load_from_data: Self::load_from_data_track_insertion_fn::<C>,
        });
    }

    /// Insert a type of Component<Tracking = Modification> to ComponentRegistry.
    pub fn register_track_modification<C: Component<Tracking = Modification> + Edit + Default + Send + Sync>(&mut self) {
        self.insert(C::name(), ComponentFn {
            create: Self::create_fn::<C>,
            create_with_data: Self::create_with_data_fn::<C>,
            destroy: Self::destroy_fn::<C>,
            save_to_data: Self::save_to_data_fn::<C>,
            load_from_data: Self::load_from_data_track_modification_fn::<C>,
        });
    }

    /// Insert a type of Component<Tracking = Deletion> to ComponentRegistry.
    pub fn register_track_deletion<C: Component<Tracking = Deletion> + Edit + Default + Send + Sync>(&mut self) {
        self.insert(C::name(), ComponentFn {
            create: Self::create_fn::<C>,
            create_with_data: Self::create_with_data_fn::<C>,
            destroy: Self::destroy_fn::<C>,
            save_to_data: Self::save_to_data_fn::<C>,
            load_from_data: Self::load_from_data_track_deletion_fn::<C>,
        });
    }

    /// Insert a type of Component<Tracking = Removal> to ComponentRegistry.
    pub fn register_track_removal<C: Component<Tracking = Removal> + Edit + Default + Send + Sync>(&mut self) {
        self.insert(C::name(), ComponentFn {
            create: Self::create_fn::<C>,
            create_with_data: Self::create_with_data_fn::<C>,
            destroy: Self::destroy_fn::<C>,
            save_to_data: Self::save_to_data_fn::<C>,
            load_from_data: Self::load_from_data_track_removal_fn::<C>,
        });
    }

    /// Insert a type of Component<Tracking = All> to ComponentRegistry.
    pub fn register_track_all<C: Component<Tracking = All> + Edit + Default + Send + Sync>(&mut self) {
        self.insert(C::name(), ComponentFn {
            create: Self::create_fn::<C>,
            create_with_data: Self::create_with_data_fn::<C>,
            destroy: Self::destroy_fn::<C>,
            save_to_data: Self::save_to_data_fn::<C>,
            load_from_data: Self::load_from_data_track_all_fn::<C>,
        });
    }

    fn create_fn<C: Component + Edit + Default + Send + Sync>(world: &mut World, entity: EntityId) {
        world.add_component(entity, (C::default(),))
    }

    fn create_with_data_fn<C: Component + Edit + Default + Send + Sync>(world: &mut World, entity: EntityId, data: &Data) {
        world.add_component(entity, (C::from_data(data),))
    }

    fn destroy_fn<C: Component + Edit + Send + Sync>(world: &mut World, entity: EntityId) {
        world.delete_component::<C>(entity)
    }

    fn save_to_data_fn<C: Component + Edit + Send + Sync>(world_data: &mut WorldData, world: &World) {
        world.run(|c: View<C>| {
            for (e, c) in c.iter().with_id() {
                let entity_data = world_data.entities.entry(e).or_insert_with(|| EntityData::new());
                entity_data.components.insert(C::name().into(), c.get_data());
            }
        });
    }

    /// Currently we must write different generic functions for different tracking type, see https://github.com/leudz/shipyard/issues/157.
    /// TODO: find a way to write only one generic function to cover all tracking type.
    fn load_from_data_untracked_fn<C: Component<Tracking = Untracked> + Edit + Send + Sync>(world: &mut World, world_data: &WorldData) {
        world.run(|mut c: ViewMut<C>| {
            for (id, c) in (&mut c).iter().with_id() {
                Self::_load_from_data(id, c, world_data);
            }
        })
    }

    fn load_from_data_track_insertion_fn<C: Component<Tracking = Insertion> + Edit + Send + Sync>(world: &mut World, world_data: &WorldData) {
        world.run(|mut c: ViewMut<C>| {
            for (id, c) in (&mut c).iter().with_id() {
                Self::_load_from_data(id, c, world_data);
            }
        })
    }

    fn load_from_data_track_modification_fn<C: Component<Tracking = Modification> + Edit + Send + Sync>(world: &mut World, world_data: &WorldData) {
        world.run(|mut c: ViewMut<C>| {
            for (id, mut c) in (&mut c).iter().with_id() {
                Self::_load_from_data(id, c.as_mut(), world_data);
            }
        })
    }

    fn load_from_data_track_deletion_fn<C: Component<Tracking = Deletion> + Edit + Send + Sync>(world: &mut World, world_data: &WorldData) {
        world.run(|mut c: ViewMut<C>| {
            for (id, c) in (&mut c).iter().with_id() {
                Self::_load_from_data(id, c, world_data);
            }
        })
    }

    fn load_from_data_track_removal_fn<C: Component<Tracking = Removal> + Edit + Send + Sync>(world: &mut World, world_data: &WorldData) {
        world.run(|mut c: ViewMut<C>| {
            for (id, c) in (&mut c).iter().with_id() {
                Self::_load_from_data(id, c, world_data);
            }
        })
    }

    fn load_from_data_track_all_fn<C: Component<Tracking = All> + Edit + Send + Sync>(world: &mut World, world_data: &WorldData) {
        world.run(|mut c: ViewMut<C>| {
            for (id, mut c) in (&mut c).iter().with_id() {
                Self::_load_from_data(id, c.as_mut(), world_data);
            }
        })
    }

    fn _load_from_data<C: Edit>(id: EntityId, c: &mut C, world_data: &WorldData) {
        if let Some(entity_data) = world_data.entities.get(&id) {
            if let Some(component_data) = entity_data.components.get(C::name()) {
                c.set_data(component_data);
            }
        }
    }
}

/// UniqueFn stores many functions of a unique, like unique save_to_data and load_from_data functions.
/// These functions are used by steel-editor so that we can use steel-editor ui to edit this unique.
pub struct UniqueFn {
    pub save_to_data: fn(&mut WorldData, &World),
    pub load_from_data: fn(&mut World, &WorldData),
    pub load_from_scene_data: fn(&mut World, &WorldData),
}

/// A map of UniqueFn, key is unique name.
pub struct UniqueRegistry(IndexMap<&'static str, UniqueFn>);

impl std::ops::Deref for UniqueRegistry {
    type Target = IndexMap<&'static str, UniqueFn>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for UniqueRegistry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl UniqueRegistry {
    /// Create a new UniqueRegistry.
    pub fn new() -> Self {
        UniqueRegistry(IndexMap::new())
    }

    /// Insert a type of Unique to UniqueRegistry.
    pub fn register<U: Unique + Edit + Send + Sync>(&mut self) {
        self.insert(U::name(), UniqueFn {
            save_to_data: Self::save_to_data_fn::<U>,
            load_from_data: Self::load_from_data_fn::<U>,
            load_from_scene_data: Self::load_from_scene_data_fn::<U>,
        });
    }

    fn save_to_data_fn<U: Unique + Edit + Send + Sync>(world_data: &mut WorldData, world: &World) {
        world.run(|u: UniqueView<U>| world_data.uniques.insert(U::name().into(), u.get_data()));
    }

    fn load_from_data_fn<U: Unique + Edit + Send + Sync>(world: &mut World, world_data: &WorldData) {
        if let Some(unique_data) = world_data.uniques.get(U::name()) {
            world.run(|mut u: UniqueViewMut<U>| u.set_data(unique_data));
        }
    }

    fn load_from_scene_data_fn<U: Unique + Edit + Send + Sync>(world: &mut World, world_data: &WorldData) {
        if let Some(unique_data) = world_data.uniques.get(U::name()) {
            world.run(|mut u: UniqueViewMut<U>| u.load_data(unique_data));
        }
    }
}
