pub use steel_common::data::*;

use crate::edit::Edit;
use indexmap::IndexMap;
use shipyard::{
    track::{All, Deletion, Insertion, Modification, Removal, Untracked},
    AddComponent, AllStorages, Component, Delete, EntitiesView, EntitiesViewMut, EntityId,
    IntoIter, IntoWithId, Unique, View, ViewMut,
};
use std::collections::HashMap;

/// ComponentFn stores many functions of a component, like component create and destroy functions.
/// These functions are used by steel-editor so that we can use steel-editor ui to edit this component.
pub struct ComponentFn {
    pub create: fn(&AllStorages, EntityId),
    pub create_with_data: fn(&AllStorages, EntityId, &Data),
    pub destroy: fn(&AllStorages, EntityId),
    pub save_to_data: fn(&mut WorldData, &AllStorages),
    pub load_from_data: fn(&AllStorages, &WorldData),
}

/// A map of ComponentFn, key is component name.
#[derive(Unique)]
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
    fn register_untracked<C: Component<Tracking = Untracked> + Edit + Default + Send + Sync>(
        &mut self,
    ) {
        self.insert(
            C::name(),
            ComponentFn {
                create: Self::create_fn::<C>,
                create_with_data: Self::create_with_data_fn::<C>,
                destroy: Self::destroy_fn::<C>,
                save_to_data: Self::save_to_data_fn::<C>,
                load_from_data: Self::load_from_data_untracked_fn::<C>,
            },
        );
    }

    /// Insert a type of Component<Tracking = Insertion> to ComponentRegistry.
    fn register_track_insertion<
        C: Component<Tracking = Insertion> + Edit + Default + Send + Sync,
    >(
        &mut self,
    ) {
        self.insert(
            C::name(),
            ComponentFn {
                create: Self::create_fn::<C>,
                create_with_data: Self::create_with_data_fn::<C>,
                destroy: Self::destroy_fn::<C>,
                save_to_data: Self::save_to_data_fn::<C>,
                load_from_data: Self::load_from_data_track_insertion_fn::<C>,
            },
        );
    }

    /// Insert a type of Component<Tracking = Modification> to ComponentRegistry.
    fn register_track_modification<
        C: Component<Tracking = Modification> + Edit + Default + Send + Sync,
    >(
        &mut self,
    ) {
        self.insert(
            C::name(),
            ComponentFn {
                create: Self::create_fn::<C>,
                create_with_data: Self::create_with_data_fn::<C>,
                destroy: Self::destroy_fn::<C>,
                save_to_data: Self::save_to_data_fn::<C>,
                load_from_data: Self::load_from_data_track_modification_fn::<C>,
            },
        );
    }

    /// Insert a type of Component<Tracking = Deletion> to ComponentRegistry.
    fn register_track_deletion<C: Component<Tracking = Deletion> + Edit + Default + Send + Sync>(
        &mut self,
    ) {
        self.insert(
            C::name(),
            ComponentFn {
                create: Self::create_fn::<C>,
                create_with_data: Self::create_with_data_fn::<C>,
                destroy: Self::destroy_fn::<C>,
                save_to_data: Self::save_to_data_fn::<C>,
                load_from_data: Self::load_from_data_track_deletion_fn::<C>,
            },
        );
    }

    /// Insert a type of Component<Tracking = Removal> to ComponentRegistry.
    fn register_track_removal<C: Component<Tracking = Removal> + Edit + Default + Send + Sync>(
        &mut self,
    ) {
        self.insert(
            C::name(),
            ComponentFn {
                create: Self::create_fn::<C>,
                create_with_data: Self::create_with_data_fn::<C>,
                destroy: Self::destroy_fn::<C>,
                save_to_data: Self::save_to_data_fn::<C>,
                load_from_data: Self::load_from_data_track_removal_fn::<C>,
            },
        );
    }

    /// Insert a type of Component<Tracking = All> to ComponentRegistry.
    fn register_track_all<C: Component<Tracking = All> + Edit + Default + Send + Sync>(&mut self) {
        self.insert(
            C::name(),
            ComponentFn {
                create: Self::create_fn::<C>,
                create_with_data: Self::create_with_data_fn::<C>,
                destroy: Self::destroy_fn::<C>,
                save_to_data: Self::save_to_data_fn::<C>,
                load_from_data: Self::load_from_data_track_all_fn::<C>,
            },
        );
    }

    fn create_fn<C: Component + Edit + Default + Send + Sync>(
        all_storages: &AllStorages,
        entity: EntityId,
    ) {
        all_storages.run(|mut c: ViewMut<C>| {
            c.add_component_unchecked(entity, C::default());
        });
    }

    fn create_with_data_fn<C: Component + Edit + Default + Send + Sync>(
        all_storages: &AllStorages,
        entity: EntityId,
        data: &Data,
    ) {
        all_storages.run(|mut c: ViewMut<C>| {
            c.add_component_unchecked(entity, C::from_data(data));
        })
    }

    fn destroy_fn<C: Component + Edit + Send + Sync>(all_storages: &AllStorages, entity: EntityId) {
        all_storages.run(|mut c: ViewMut<C>| {
            c.delete(entity);
        });
    }

    fn save_to_data_fn<C: Component + Edit + Send + Sync>(
        world_data: &mut WorldData,
        all_storages: &AllStorages,
    ) {
        all_storages.run(|c: View<C>| {
            for (e, c) in c.iter().with_id() {
                let entity_data = world_data
                    .entities
                    .entry(e)
                    .or_insert_with(|| EntityData::default());
                entity_data.components.insert(C::name().into(), c.to_data());
            }
        });
    }

    /// Currently we must write different generic functions for different tracking type, see https://github.com/leudz/shipyard/issues/157.
    /// TODO: find a way to write only one generic function to cover all tracking type.
    fn load_from_data_untracked_fn<C: Component<Tracking = Untracked> + Edit + Send + Sync>(
        all_storages: &AllStorages,
        world_data: &WorldData,
    ) {
        all_storages.run(|mut c: ViewMut<C>| {
            for (id, c) in (&mut c).iter().with_id() {
                Self::load_from_data_inner(id, c, world_data);
            }
        })
    }

    fn load_from_data_track_insertion_fn<
        C: Component<Tracking = Insertion> + Edit + Send + Sync,
    >(
        all_storages: &AllStorages,
        world_data: &WorldData,
    ) {
        all_storages.run(|mut c: ViewMut<C>| {
            for (id, c) in (&mut c).iter().with_id() {
                Self::load_from_data_inner(id, c, world_data);
            }
        })
    }

    fn load_from_data_track_modification_fn<
        C: Component<Tracking = Modification> + Edit + Send + Sync,
    >(
        all_storages: &AllStorages,
        world_data: &WorldData,
    ) {
        all_storages.run(|mut c: ViewMut<C>| {
            for (id, mut c) in (&mut c).iter().with_id() {
                Self::load_from_data_inner(id, c.as_mut(), world_data);
            }
        })
    }

    fn load_from_data_track_deletion_fn<C: Component<Tracking = Deletion> + Edit + Send + Sync>(
        all_storages: &AllStorages,
        world_data: &WorldData,
    ) {
        all_storages.run(|mut c: ViewMut<C>| {
            for (id, c) in (&mut c).iter().with_id() {
                Self::load_from_data_inner(id, c, world_data);
            }
        })
    }

    fn load_from_data_track_removal_fn<C: Component<Tracking = Removal> + Edit + Send + Sync>(
        all_storages: &AllStorages,
        world_data: &WorldData,
    ) {
        all_storages.run(|mut c: ViewMut<C>| {
            for (id, c) in (&mut c).iter().with_id() {
                Self::load_from_data_inner(id, c, world_data);
            }
        })
    }

    fn load_from_data_track_all_fn<C: Component<Tracking = All> + Edit + Send + Sync>(
        all_storages: &AllStorages,
        world_data: &WorldData,
    ) {
        all_storages.run(|mut c: ViewMut<C>| {
            for (id, mut c) in (&mut c).iter().with_id() {
                Self::load_from_data_inner(id, c.as_mut(), world_data);
            }
        })
    }

    fn load_from_data_inner<C: Edit>(id: EntityId, c: &mut C, world_data: &WorldData) {
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
where
    C: Component,
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

impl<C> ComponentRegistryExtInner for (C, Untracked)
where
    C: Component<Tracking = Untracked> + Edit + Default + Send + Sync,
{
    fn register(component_registry: &mut ComponentRegistry) {
        component_registry.register_untracked::<C>();
    }
}

impl<C> ComponentRegistryExtInner for (C, Insertion)
where
    C: Component<Tracking = Insertion> + Edit + Default + Send + Sync,
{
    fn register(component_registry: &mut ComponentRegistry) {
        component_registry.register_track_insertion::<C>();
    }
}

impl<C> ComponentRegistryExtInner for (C, Modification)
where
    C: Component<Tracking = Modification> + Edit + Default + Send + Sync,
{
    fn register(component_registry: &mut ComponentRegistry) {
        component_registry.register_track_modification::<C>();
    }
}

impl<C> ComponentRegistryExtInner for (C, Deletion)
where
    C: Component<Tracking = Deletion> + Edit + Default + Send + Sync,
{
    fn register(component_registry: &mut ComponentRegistry) {
        component_registry.register_track_deletion::<C>();
    }
}

impl<C> ComponentRegistryExtInner for (C, Removal)
where
    C: Component<Tracking = Removal> + Edit + Default + Send + Sync,
{
    fn register(component_registry: &mut ComponentRegistry) {
        component_registry.register_track_removal::<C>();
    }
}

impl<C> ComponentRegistryExtInner for (C, All)
where
    C: Component<Tracking = All> + Edit + Default + Send + Sync,
{
    fn register(component_registry: &mut ComponentRegistry) {
        component_registry.register_track_all::<C>();
    }
}

/// UniqueFn stores many functions of a unique, like unique save_to_data and load_from_data functions.
/// These functions are used by steel-editor so that we can use steel-editor ui to edit this unique.
pub struct UniqueFn {
    pub save_to_data: fn(&mut WorldData, &AllStorages),
    pub load_from_data: fn(&AllStorages, &WorldData),
    pub load_from_scene_data: fn(&AllStorages, &WorldData),
    pub reset: fn(&AllStorages),
}

/// A map of UniqueFn, key is unique name.
#[derive(Unique)]
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
    pub fn register<U: Unique + Edit + Default + Send + Sync>(&mut self) {
        self.insert(
            U::name(),
            UniqueFn {
                save_to_data: Self::save_to_data_fn::<U>,
                load_from_data: Self::load_from_data_fn::<U>,
                load_from_scene_data: Self::load_from_scene_data_fn::<U>,
                reset: Self::reset::<U>,
            },
        );
    }

    fn save_to_data_fn<U: Unique + Edit + Send + Sync>(
        world_data: &mut WorldData,
        all_storages: &AllStorages,
    ) {
        let u = all_storages.get_unique::<&U>().unwrap();
        world_data.uniques.insert(U::name().into(), u.to_data());
    }

    fn load_from_data_fn<U: Unique + Edit + Send + Sync>(
        all_storages: &AllStorages,
        world_data: &WorldData,
    ) {
        if let Some(unique_data) = world_data.uniques.get(U::name()) {
            all_storages
                .get_unique::<&mut U>()
                .unwrap()
                .set_data(unique_data);
        }
    }

    fn load_from_scene_data_fn<U: Unique + Edit + Send + Sync>(
        all_storages: &AllStorages,
        world_data: &WorldData,
    ) {
        if let Some(unique_data) = world_data.uniques.get(U::name()) {
            all_storages
                .get_unique::<&mut U>()
                .unwrap()
                .load_data(unique_data);
        }
    }

    fn reset<U: Unique + Edit + Default + Send + Sync>(all_storages: &AllStorages) {
        *all_storages.get_unique::<&mut U>().unwrap() = U::default();
    }
}

/// WorldData extension functions in steel core library.
pub trait WorldDataExt {
    /// Add entities and uniques of self into ecs world. Return old_id_to_new_id map.
    fn add_to_world(&self, all_storages: &AllStorages) -> HashMap<EntityId, EntityId>;
}

impl WorldDataExt for WorldData {
    fn add_to_world(&self, all_storages: &AllStorages) -> HashMap<EntityId, EntityId> {
        // create new_world_data from self by changing old entity ids to new entity ids.
        let mut new_world_data = WorldData::default();
        let old_id_to_new_id = create_old_id_to_new_id_map(&self.entities, all_storages);
        fill_new_entities_data(
            &mut new_world_data.entities,
            &self.entities,
            &old_id_to_new_id,
            &all_storages,
        );
        for (unique_name, unique_data) in &self.uniques {
            let new_unique_data = update_eid_in_data(unique_data, &old_id_to_new_id, &all_storages);
            new_world_data
                .uniques
                .insert(unique_name.clone(), new_unique_data);
        }

        // create components in ecs world.
        create_components_in_world(&new_world_data.entities, all_storages);

        // load uniques in ecs world.
        for unique_fn in all_storages
            .get_unique::<&UniqueRegistry>()
            .unwrap()
            .values()
        {
            (unique_fn.reset)(all_storages);
            (unique_fn.load_from_scene_data)(all_storages, &new_world_data);
        }

        old_id_to_new_id
    }
}

/// EntitiesData extension functions in steel core library.
pub trait EntitiesDataExt {
    /// Add entities of self into ecs world. Return old_id_to_new_id map.
    fn add_to_world(&self, all_storages: &AllStorages) -> HashMap<EntityId, EntityId>;
}

impl EntitiesDataExt for EntitiesData {
    fn add_to_world(&self, all_storages: &AllStorages) -> HashMap<EntityId, EntityId> {
        // create new_entities_data from self by changing old entity ids to new entity ids.
        let mut new_entities_data = EntitiesData::default();
        let old_id_to_new_id = create_old_id_to_new_id_map(self, all_storages);
        fill_new_entities_data(
            &mut new_entities_data,
            self,
            &old_id_to_new_id,
            all_storages,
        );

        // create components in ecs world.
        create_components_in_world(&new_entities_data, all_storages);

        old_id_to_new_id
    }
}

/// Create old_id_to_new_id map, the new ids are generated by adding new entities in ecs world.
fn create_old_id_to_new_id_map(
    entities_data: &EntitiesData,
    all_storages: &AllStorages,
) -> HashMap<EntityId, EntityId> {
    let mut old_id_to_new_id = HashMap::new();
    for old_id in entities_data.keys() {
        old_id_to_new_id.insert(
            *old_id,
            all_storages
                .borrow::<EntitiesViewMut>()
                .unwrap()
                .add_entity((), ()),
        );
    }
    old_id_to_new_id
}

/// Fill new_entities_data from old_entities_data by changing old entity ids to new entity ids.
fn fill_new_entities_data(
    new_entities_data: &mut EntitiesData,
    old_entities_data: &EntitiesData,
    old_id_to_new_id: &HashMap<EntityId, EntityId>,
    all_storages: &AllStorages,
) {
    for (old_id, entity_data) in old_entities_data {
        let new_id = *old_id_to_new_id.get(old_id).unwrap();
        let mut new_entity_data = EntityData::default();
        for (component_name, component_data) in &entity_data.components {
            let new_component_data =
                update_eid_in_data(component_data, &old_id_to_new_id, all_storages);
            new_entity_data
                .components
                .insert(component_name.clone(), new_component_data);
        }
        new_entities_data.insert(new_id, new_entity_data);
    }
}

/// Update entity ids in data according to old_id_to_new_id.
fn update_eid_in_data(
    data: &Data,
    old_id_to_new_id: &HashMap<EntityId, EntityId>,
    all_storages: &AllStorages,
) -> Data {
    let mut new_data = Data::new();
    for (name, value) in &data.values {
        let new_value = value.map_entity(|e: EntityId| {
            if let Some(new_id) = old_id_to_new_id.get(&e) {
                *new_id
            } else if e == EntityId::dead() {
                EntityId::dead()
            } else if all_storages.borrow::<EntitiesView>().unwrap().is_alive(e) {
                e
            } else {
                panic!("non-exist EntityId: {e:?}");
            }
        });
        new_data.insert(name, new_value);
    }
    new_data
}

/// Create components in ecs world according to entities_data.
fn create_components_in_world(entities_data: &EntitiesData, all_storages: &AllStorages) {
    for (eid, entity_data) in entities_data {
        for (component_name, component_data) in &entity_data.components {
            if let Some(component_fn) = all_storages
                .get_unique::<&ComponentRegistry>()
                .unwrap()
                .get(component_name.as_str())
            {
                (component_fn.create_with_data)(all_storages, *eid, component_data);
            }
        }
    }
}
