pub use steel_common::data::*;

use std::collections::HashMap;
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

    /// Insert a type of Component with any tracking type to ComponentRegistry.
    /// Trait bounds <C: ComponentRegistryExt> equals to <C: Component + Edit + Default + Send + Sync>.
    pub fn register<C: ComponentRegistryExt>(&mut self) {
        C::register(self);
    }

    /// Insert a type of Component<Tracking = Untracked> to ComponentRegistry.
    fn register_untracked<C: Component<Tracking = Untracked> + Edit + Default + Send + Sync>(&mut self) {
        self.insert(C::name(), ComponentFn {
            create: Self::create_fn::<C>,
            create_with_data: Self::create_with_data_fn::<C>,
            destroy: Self::destroy_fn::<C>,
            save_to_data: Self::save_to_data_fn::<C>,
            load_from_data: Self::load_from_data_untracked_fn::<C>,
        });
    }

    /// Insert a type of Component<Tracking = Insertion> to ComponentRegistry.
    fn register_track_insertion<C: Component<Tracking = Insertion> + Edit + Default + Send + Sync>(&mut self) {
        self.insert(C::name(), ComponentFn {
            create: Self::create_fn::<C>,
            create_with_data: Self::create_with_data_fn::<C>,
            destroy: Self::destroy_fn::<C>,
            save_to_data: Self::save_to_data_fn::<C>,
            load_from_data: Self::load_from_data_track_insertion_fn::<C>,
        });
    }

    /// Insert a type of Component<Tracking = Modification> to ComponentRegistry.
    fn register_track_modification<C: Component<Tracking = Modification> + Edit + Default + Send + Sync>(&mut self) {
        self.insert(C::name(), ComponentFn {
            create: Self::create_fn::<C>,
            create_with_data: Self::create_with_data_fn::<C>,
            destroy: Self::destroy_fn::<C>,
            save_to_data: Self::save_to_data_fn::<C>,
            load_from_data: Self::load_from_data_track_modification_fn::<C>,
        });
    }

    /// Insert a type of Component<Tracking = Deletion> to ComponentRegistry.
    fn register_track_deletion<C: Component<Tracking = Deletion> + Edit + Default + Send + Sync>(&mut self) {
        self.insert(C::name(), ComponentFn {
            create: Self::create_fn::<C>,
            create_with_data: Self::create_with_data_fn::<C>,
            destroy: Self::destroy_fn::<C>,
            save_to_data: Self::save_to_data_fn::<C>,
            load_from_data: Self::load_from_data_track_deletion_fn::<C>,
        });
    }

    /// Insert a type of Component<Tracking = Removal> to ComponentRegistry.
    fn register_track_removal<C: Component<Tracking = Removal> + Edit + Default + Send + Sync>(&mut self) {
        self.insert(C::name(), ComponentFn {
            create: Self::create_fn::<C>,
            create_with_data: Self::create_with_data_fn::<C>,
            destroy: Self::destroy_fn::<C>,
            save_to_data: Self::save_to_data_fn::<C>,
            load_from_data: Self::load_from_data_track_removal_fn::<C>,
        });
    }

    /// Insert a type of Component<Tracking = All> to ComponentRegistry.
    fn register_track_all<C: Component<Tracking = All> + Edit + Default + Send + Sync>(&mut self) {
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

/// Helper trait for registering components with different tracking types.
/// This trait bounds equals to "Component + Edit + Default + Send + Sync".
pub trait ComponentRegistryExt {
    /// Register a Component type into component_registry.
    fn register(component_registry: &mut ComponentRegistry);
}

impl<C> ComponentRegistryExt for C
    where C: Component,
          (C, <C as Component>::Tracking): ComponentRegistryExtInner,
{
    fn register(component_registry: &mut ComponentRegistry) {
        <(C, <C as Component>::Tracking)>::register(component_registry);
    }
}

/// Helper trait for registering components with different tracking types.
trait ComponentRegistryExtInner {
    fn register(component_registry: &mut ComponentRegistry);
}

impl<C> ComponentRegistryExtInner for (C, Untracked) where C: Component<Tracking = Untracked> + Edit + Default + Send + Sync {
    fn register(component_registry: &mut ComponentRegistry) {
        component_registry.register_untracked::<C>();
    }
}

impl<C> ComponentRegistryExtInner for (C, Insertion) where C: Component<Tracking = Insertion> + Edit + Default + Send + Sync {
    fn register(component_registry: &mut ComponentRegistry) {
        component_registry.register_track_insertion::<C>();
    }
}

impl<C> ComponentRegistryExtInner for (C, Modification) where C: Component<Tracking = Modification> + Edit + Default + Send + Sync {
    fn register(component_registry: &mut ComponentRegistry) {
        component_registry.register_track_modification::<C>();
    }
}

impl<C> ComponentRegistryExtInner for (C, Deletion) where C: Component<Tracking = Deletion> + Edit + Default + Send + Sync {
    fn register(component_registry: &mut ComponentRegistry) {
        component_registry.register_track_deletion::<C>();
    }
}

impl<C> ComponentRegistryExtInner for (C, Removal) where C: Component<Tracking = Removal> + Edit + Default + Send + Sync {
    fn register(component_registry: &mut ComponentRegistry) {
        component_registry.register_track_removal::<C>();
    }
}

impl<C> ComponentRegistryExtInner for (C, All) where C: Component<Tracking = All> + Edit + Default + Send + Sync {
    fn register(component_registry: &mut ComponentRegistry) {
        component_registry.register_track_all::<C>();
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

/// WorldData extension functions.
pub trait WorldDataExt {
    /// Add entities and uniques of this WorldData into ecs world.
    fn add_to_world(&self, world: &mut World, component_registry: &ComponentRegistry, unique_registry: &UniqueRegistry);
}

impl WorldDataExt for WorldData {
    fn add_to_world(&self, world: &mut World, component_registry: &ComponentRegistry, unique_registry: &UniqueRegistry) {
        // create new_world_data from world_data by changing old entity ids to new entity ids.
        let mut new_world_data = WorldData::new();
        let mut old_id_to_new_id = HashMap::new();
        for old_id in self.entities.keys() {
            old_id_to_new_id.insert(*old_id, world.add_entity(()));
        }
        for (old_id, entity_data) in &self.entities {
            let new_id = *old_id_to_new_id.get(old_id).unwrap();
            let mut new_entity_data = EntityData::new();
            for (component_name, component_data) in &entity_data.components {
                let new_component_data = update_eid_in_data(component_data, &old_id_to_new_id);
                new_entity_data.components.insert(component_name.clone(), new_component_data);
            }
            new_world_data.entities.insert(new_id, new_entity_data);
        }
        for (unique_name, unique_data) in &self.uniques {
            let new_unique_data = update_eid_in_data(unique_data, &old_id_to_new_id);
            new_world_data.uniques.insert(unique_name.clone(), new_unique_data);
        }

        // create components and uniques in ecs world.
        for (new_id, entity_data) in &new_world_data.entities {
            for (component_name, component_data) in &entity_data.components {
                if let Some(component_fn) = component_registry.get(component_name.as_str()) {
                    (component_fn.create_with_data)(world, *new_id, component_data);
                }
            }
        }
        for unique_fn in unique_registry.values() {
            (unique_fn.load_from_scene_data)(world, &new_world_data);
        }
    }
}

fn update_eid_in_data(data: &Data, old_id_to_new_id: &HashMap<EntityId, EntityId>) -> Data {
    let get_id_fn = |e: &EntityId| {
        if let Some(new_id) = old_id_to_new_id.get(e) {
            *new_id
        } else if *e == EntityId::dead() {
            EntityId::dead()
        } else {
            panic!("non-exist EntityId: {e:?}");
        }
    };

    let mut new_data = Data::new();
    for (name, value) in &data.values {
        let new_value = match value {
            Value::Entity(e) => Value::Entity(get_id_fn(e)),
            Value::VecEntity(v) => Value::VecEntity(v.iter().map(|e| get_id_fn(e)).collect()),
            _ => value.clone(),
        };
        new_data.add_value(name, new_value);
    }
    new_data
}
