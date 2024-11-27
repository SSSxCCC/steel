pub use steel_common::data::*;

use crate::{
    asset::AssetManager,
    edit::Edit,
    hierarchy::{Children, Parent},
};
use indexmap::IndexMap;
use shipyard::{
    track::{All, Deletion, Insertion, Modification, Removal, Untracked},
    AddComponent, Component, EntitiesView, EntityId, Get, IntoIter, IntoWithId, Unique, UniqueView,
    UniqueViewMut, View, ViewMut, World,
};
use std::{collections::HashMap, sync::Arc};
use steel_common::{asset::AssetId, platform::Platform};

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

    fn create_fn<C: Component + Edit + Default + Send + Sync>(world: &mut World, entity: EntityId) {
        world.add_component(entity, (C::default(),))
    }

    fn create_with_data_fn<C: Component + Edit + Default + Send + Sync>(
        world: &mut World,
        entity: EntityId,
        data: &Data,
    ) {
        world.add_component(entity, (C::from_data(data),))
    }

    fn destroy_fn<C: Component + Edit + Send + Sync>(world: &mut World, entity: EntityId) {
        world.delete_component::<C>(entity)
    }

    fn save_to_data_fn<C: Component + Edit + Send + Sync>(
        world_data: &mut WorldData,
        world: &World,
    ) {
        world.run(|c: View<C>| {
            for (e, c) in c.iter().with_id() {
                let entity_data = world_data
                    .entities
                    .entry(e)
                    .or_insert_with(|| EntityData::default());
                entity_data
                    .components
                    .insert(C::name().into(), c.get_data());
            }
        });
    }

    /// Currently we must write different generic functions for different tracking type, see https://github.com/leudz/shipyard/issues/157.
    /// TODO: find a way to write only one generic function to cover all tracking type.
    fn load_from_data_untracked_fn<C: Component<Tracking = Untracked> + Edit + Send + Sync>(
        world: &mut World,
        world_data: &WorldData,
    ) {
        world.run(|mut c: ViewMut<C>| {
            for (id, c) in (&mut c).iter().with_id() {
                Self::_load_from_data(id, c, world_data);
            }
        })
    }

    fn load_from_data_track_insertion_fn<
        C: Component<Tracking = Insertion> + Edit + Send + Sync,
    >(
        world: &mut World,
        world_data: &WorldData,
    ) {
        world.run(|mut c: ViewMut<C>| {
            for (id, c) in (&mut c).iter().with_id() {
                Self::_load_from_data(id, c, world_data);
            }
        })
    }

    fn load_from_data_track_modification_fn<
        C: Component<Tracking = Modification> + Edit + Send + Sync,
    >(
        world: &mut World,
        world_data: &WorldData,
    ) {
        world.run(|mut c: ViewMut<C>| {
            for (id, mut c) in (&mut c).iter().with_id() {
                Self::_load_from_data(id, c.as_mut(), world_data);
            }
        })
    }

    fn load_from_data_track_deletion_fn<C: Component<Tracking = Deletion> + Edit + Send + Sync>(
        world: &mut World,
        world_data: &WorldData,
    ) {
        world.run(|mut c: ViewMut<C>| {
            for (id, c) in (&mut c).iter().with_id() {
                Self::_load_from_data(id, c, world_data);
            }
        })
    }

    fn load_from_data_track_removal_fn<C: Component<Tracking = Removal> + Edit + Send + Sync>(
        world: &mut World,
        world_data: &WorldData,
    ) {
        world.run(|mut c: ViewMut<C>| {
            for (id, c) in (&mut c).iter().with_id() {
                Self::_load_from_data(id, c, world_data);
            }
        })
    }

    fn load_from_data_track_all_fn<C: Component<Tracking = All> + Edit + Send + Sync>(
        world: &mut World,
        world_data: &WorldData,
    ) {
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
        self.insert(
            U::name(),
            UniqueFn {
                save_to_data: Self::save_to_data_fn::<U>,
                load_from_data: Self::load_from_data_fn::<U>,
                load_from_scene_data: Self::load_from_scene_data_fn::<U>,
            },
        );
    }

    fn save_to_data_fn<U: Unique + Edit + Send + Sync>(world_data: &mut WorldData, world: &World) {
        world.run(|u: UniqueView<U>| world_data.uniques.insert(U::name().into(), u.get_data()));
    }

    fn load_from_data_fn<U: Unique + Edit + Send + Sync>(
        world: &mut World,
        world_data: &WorldData,
    ) {
        if let Some(unique_data) = world_data.uniques.get(U::name()) {
            world.run(|mut u: UniqueViewMut<U>| u.set_data(unique_data));
        }
    }

    fn load_from_scene_data_fn<U: Unique + Edit + Send + Sync>(
        world: &mut World,
        world_data: &WorldData,
    ) {
        if let Some(unique_data) = world_data.uniques.get(U::name()) {
            world.run(|mut u: UniqueViewMut<U>| u.load_data(unique_data));
        }
    }
}

/// WorldData extension functions in steel core library.
pub trait WorldDataExt {
    /// Add entities and uniques of self into ecs world. Return old_id_to_new_id map.
    fn add_to_world(
        &self,
        world: &mut World,
        component_registry: &ComponentRegistry,
        unique_registry: &UniqueRegistry,
    ) -> HashMap<EntityId, EntityId>;
}

impl WorldDataExt for WorldData {
    fn add_to_world(
        &self,
        world: &mut World,
        component_registry: &ComponentRegistry,
        unique_registry: &UniqueRegistry,
    ) -> HashMap<EntityId, EntityId> {
        // create new_world_data from self by changing old entity ids to new entity ids.
        let mut new_world_data = WorldData::default();
        let old_id_to_new_id = create_old_id_to_new_id_map(&self.entities, world);
        fill_new_entities_data(
            &mut new_world_data.entities,
            &self.entities,
            &old_id_to_new_id,
            &world,
        );
        for (unique_name, unique_data) in &self.uniques {
            let new_unique_data = update_eid_in_data(unique_data, &old_id_to_new_id, &world);
            new_world_data
                .uniques
                .insert(unique_name.clone(), new_unique_data);
        }

        // create components in ecs world.
        create_components_in_world(&new_world_data.entities, world, component_registry);

        // load uniques in ecs world.
        for unique_fn in unique_registry.values() {
            (unique_fn.load_from_scene_data)(world, &new_world_data);
        }

        old_id_to_new_id
    }
}

/// EntitiesData extension functions in steel core library.
pub trait EntitiesDataExt {
    /// Add entities of self into ecs world. Return old_id_to_new_id map.
    fn add_to_world(
        &self,
        world: &mut World,
        component_registry: &ComponentRegistry,
    ) -> HashMap<EntityId, EntityId>;
}

impl EntitiesDataExt for EntitiesData {
    fn add_to_world(
        &self,
        world: &mut World,
        component_registry: &ComponentRegistry,
    ) -> HashMap<EntityId, EntityId> {
        // create new_entities_data from self by changing old entity ids to new entity ids.
        let mut new_entities_data = EntitiesData::default();
        let old_id_to_new_id = create_old_id_to_new_id_map(self, world);
        fill_new_entities_data(&mut new_entities_data, self, &old_id_to_new_id, &world);

        // create components in ecs world.
        create_components_in_world(&new_entities_data, world, component_registry);

        old_id_to_new_id
    }
}

/// Create old_id_to_new_id map, the new ids are generated by adding new entities in ecs world.
fn create_old_id_to_new_id_map(
    entities_data: &EntitiesData,
    world: &mut World,
) -> HashMap<EntityId, EntityId> {
    let mut old_id_to_new_id = HashMap::new();
    for old_id in entities_data.keys() {
        old_id_to_new_id.insert(*old_id, world.add_entity(()));
    }
    old_id_to_new_id
}

/// Fill new_entities_data from old_entities_data by changing old entity ids to new entity ids.
fn fill_new_entities_data(
    new_entities_data: &mut EntitiesData,
    old_entities_data: &EntitiesData,
    old_id_to_new_id: &HashMap<EntityId, EntityId>,
    world: &World,
) {
    for (old_id, entity_data) in old_entities_data {
        let new_id = *old_id_to_new_id.get(old_id).unwrap();
        let mut new_entity_data = EntityData::default();
        for (component_name, component_data) in &entity_data.components {
            let new_component_data = update_eid_in_data(component_data, &old_id_to_new_id, world);
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
    world: &World,
) -> Data {
    let get_id_fn = |e: &EntityId| {
        if let Some(new_id) = old_id_to_new_id.get(e) {
            *new_id
        } else if *e == EntityId::dead() {
            EntityId::dead()
        } else if world.run(|entities: EntitiesView| entities.is_alive(*e)) {
            *e
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

/// Create components in ecs world according to entities_data.
fn create_components_in_world(
    entities_data: &EntitiesData,
    world: &mut World,
    component_registry: &ComponentRegistry,
) {
    for (eid, entity_data) in entities_data {
        for (component_name, component_data) in &entity_data.components {
            if let Some(component_fn) = component_registry.get(component_name.as_str()) {
                (component_fn.create_with_data)(world, *eid, component_data);
            }
        }
    }
}

/// Prefab component contains prefab info about this entity:
/// 1. Prefab asset id
/// 2. Entity id in prefab
/// 3. Entity id path in prefab
/// 4. Prefab root entity id
///
/// An entity without this component means that it is not created from a prefab.
#[derive(Component, Edit, Default, Debug)]
pub struct Prefab {
    /// The asset id of the prefab that this entity belongs to.
    #[edit(limit = "Limit::ReadOnly")]
    asset: AssetId,
    /// The entity id index in the prefab that this entity belongs to.
    /// This type is not [EntityId] because:
    /// 1. this is a static id so that it should not be mapped with other entity ids when saving prefab.
    /// 2. the generation of entity id is always 0 in prefabs.
    ///
    /// You can use [EntityId::new_from_index_and_gen] and [EntityId::index] to convert beetween [u64] and [EntityId].
    #[edit(limit = "Limit::ReadOnly")]
    entity_index: u64,
    /// The entity id path of the prefab that this entity belongs to. Note that this type is [EntityIdPath],
    /// but we use Vec\<u64\> here because currently edit derive macro dosen't support rust type alias.
    /// TODO: support rust type alias in edit derive macro.
    #[edit(limit = "Limit::ReadOnly")]
    entity_path: Vec<u64>,
    /// The root entity of the prefab that this entity belongs to.
    #[edit(limit = "Limit::ReadOnly")]
    root_entity: EntityId,
}

impl Prefab {
    /// Get the asset id of the prefab that this entity belongs to.
    pub fn asset(&self) -> AssetId {
        self.asset
    }

    /// Get the entity id index in prefab.
    pub fn entity_index(&self) -> u64 {
        self.entity_index
    }

    /// Get the entity id path in prefab.
    pub fn entity_path(&self) -> &EntityIdPath {
        &self.entity_path
    }

    /// Get the root entity of prefab that this entity belongs to.
    pub fn root_entity(&self) -> EntityId {
        self.root_entity
    }
}

/// Parameters for [create_prefab_system].
#[derive(Unique)]
pub(crate) struct CreatePrefabParam {
    pub prefab_root_entity: EntityId,
    pub prefab_asset: AssetId,
    pub prefab_root_entity_to_nested_prefabs_index: HashMap<EntityId, u64>,
}

/// After creating a prefab, we must run this system to update [Prefab] components.
pub(crate) fn create_prefab_system(
    param: UniqueView<CreatePrefabParam>,
    childrens: View<Children>,
    mut prefabs: ViewMut<Prefab>,
) {
    // traverse param.prefab_root_entity and all its ancestors
    let mut es = vec![param.prefab_root_entity];
    while let Some(e) = es.pop() {
        if let Ok(children) = childrens.get(e) {
            es.extend(children);
        }

        // get the Prefab component for e
        if !prefabs.contains(e) {
            prefabs.add_component_unchecked(e, Prefab::default());
        }
        let mut prefab = (&mut prefabs).get(e).unwrap();

        // update Prefab component values
        if prefab.asset == AssetId::INVALID {
            prefab.entity_index = e.index();
        } else {
            let nested_prefab_index =
                param.prefab_root_entity_to_nested_prefabs_index[&prefab.root_entity];
            prefab.entity_path.insert(0, nested_prefab_index);
        }
        prefab.asset = param.prefab_asset;
        prefab.root_entity = param.prefab_root_entity;
    }
}

/// Parameters for [load_prefab_system].
#[derive(Unique)]
pub(crate) struct LoadPrefabParam {
    pub prefab_root_entity: EntityId,
    pub prefab_asset: AssetId,
    pub entity_id_to_prefab_entity_id_with_path: HashMap<EntityId, EntityIdWithPath>,
}

/// After loading a prefab, we must run this system to update [Prefab] components.
pub(crate) fn load_prefab_system(
    mut param: UniqueViewMut<LoadPrefabParam>,
    mut prefabs: ViewMut<Prefab>,
) {
    // traverse all entities in this prefab
    for (e, EntityIdWithPath(prefab_entity, prefab_entity_path)) in
        std::mem::take(&mut param.entity_id_to_prefab_entity_id_with_path)
    {
        // get the Prefab component for e
        if !prefabs.contains(e) {
            prefabs.add_component_unchecked(e, Prefab::default());
        }
        let mut prefab = (&mut prefabs).get(e).unwrap();

        // update Prefab component values
        prefab.entity_index = prefab_entity.index();
        prefab.entity_path = prefab_entity_path;
        prefab.asset = param.prefab_asset;
        prefab.root_entity = param.prefab_root_entity;
    }
}

/// Parameters for [load_scene_prefabs_system].
#[derive(Unique)]
pub(crate) struct LoadScenePrefabsParam {
    pub prefab_asset_and_entity_id_to_prefab_entity_id_with_path:
        Vec<(AssetId, HashMap<EntityId, EntityIdWithPath>)>,
}

/// After loading a scene, we must run this system to update [Prefab] components for all prefabs in the scene.
pub(crate) fn load_scene_prefabs_system(
    mut param: UniqueViewMut<LoadScenePrefabsParam>,
    parents: View<Parent>,
    mut prefabs: ViewMut<Prefab>,
) {
    // for every prefab
    for (prefab_asset, entity_id_to_prefab_entity_id_with_path) in
        std::mem::take(&mut param.prefab_asset_and_entity_id_to_prefab_entity_id_with_path)
    {
        // no entity in this prefab, maybe this prefab was deleted
        if entity_id_to_prefab_entity_id_with_path.is_empty() {
            continue;
        }

        // find prefab root entity
        let prefab_root_entity = (|| {
            for &e in entity_id_to_prefab_entity_id_with_path.keys() {
                let parent = parents.get(e).map(|p| **p).unwrap_or_default();
                if !entity_id_to_prefab_entity_id_with_path.contains_key(&parent) {
                    return e;
                }
            }
            panic!("load_scene_prefabs_system: no root found for prefab: {prefab_asset:?}");
        })();

        // for every entity in this prefab
        for (e, EntityIdWithPath(prefab_entity, prefab_entity_path)) in
            entity_id_to_prefab_entity_id_with_path
        {
            // get the Prefab component for e
            if !prefabs.contains(e) {
                prefabs.add_component_unchecked(e, Prefab::default());
            }
            let mut prefab = (&mut prefabs).get(e).unwrap();

            // update Prefab component values
            prefab.entity_index = prefab_entity.index();
            prefab.entity_path = prefab_entity_path;
            prefab.asset = prefab_asset;
            prefab.root_entity = prefab_root_entity;
        }
    }
}

struct PrefabAsset {
    bytes: Arc<Vec<u8>>,
    data: Arc<PrefabData>,
}

#[derive(Unique, Default)]
/// Cache [PrefabData] in assets.
pub struct PrefabAssets {
    prefabs: HashMap<AssetId, PrefabAsset>,
}

impl PrefabAssets {
    pub fn get_prefab_data(
        &mut self,
        asset_id: AssetId,
        asset_manager: &mut AssetManager,
        platform: &Platform,
    ) -> Option<Arc<PrefabData>> {
        if let Some(bytes) = asset_manager.get_asset_content(asset_id, platform) {
            if let Some(prefab_asset) = self.prefabs.get(&asset_id) {
                if Arc::ptr_eq(bytes, &prefab_asset.bytes) {
                    // cache is still valid
                    return Some(prefab_asset.data.clone());
                }
            }
            // cache is not valid, reload data
            match serde_json::from_slice::<PrefabData>(&bytes) {
                Ok(data) => {
                    let prefab_data = Arc::new(data);
                    self.prefabs.insert(
                        asset_id,
                        PrefabAsset {
                            bytes: bytes.clone(),
                            data: prefab_data.clone(),
                        },
                    );
                    return Some(prefab_data);
                }
                Err(e) => log::error!("PrefabAssets::get_prefab_data: error: {}", e),
            }
        }
        self.prefabs.remove(&asset_id);
        None
    }
}
