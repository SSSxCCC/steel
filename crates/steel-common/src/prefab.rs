use crate::{
    asset::AssetId,
    data::{vectorize, Data, EntitiesData, EntityData, Limit, Value},
};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use shipyard::EntityId;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

/// Prefab is a collection of entities, which stored in an asset, to be used to
/// create entities as template. A prefab can have many nested prefabs.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PrefabData {
    /// All entities with id paths in self prefab and all modifications from the original prefab in nested prefabs.
    /// for keys whose path is empty, it means the entity is in self prefab.
    /// The first entity should be the root entity of the prefab.
    #[serde(with = "vectorize")] // TODO: #[serde_as(as = "Vec<(_, _)>")]
    pub entities: IndexMap<EntityIdWithPath, EntityDataWithIdPaths>,
    /// All nested prefabs.
    pub nested_prefabs: Vec<AssetId>,
    /// Deleted entities and components in nested prefabs.
    /// Value is a set that contains all the components deleted in the key entity,
    /// if it is an empty set, it means this key entity is deleted.
    #[serde(with = "vectorize")] // TODO: #[serde_as(as = "Vec<(_, _)>")]
    pub delete: IndexMap<EntityIdWithPath, IndexSet<String>>,
}

impl PrefabData {
    /// Create a new prefab data from entities, also return prefab_root_entity_to_nested_prefabs_index map.
    /// The first entity of input entities must be the root, so that after creating the prefab,
    /// we can regard the first entity as the root entity.
    pub fn new(
        mut entities: EntitiesData,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
    ) -> (Self, HashMap<EntityId, u64>) {
        // remove Parent component in root entity so that hierarchy maintain system
        // can work correctly when we create entities from this prefab in the scene
        let root_entity_data = entities
            .get_index_mut(0)
            .expect("PrefabData::new: prefab should have at least one entity!")
            .1;
        root_entity_data.components.shift_remove("Parent");

        Self::new_with_prefab_data_override(&entities, get_prefab_data_fn, None)
    }

    /// This function has an extra parameter 'prefab_data_override' compared to [Self::new].
    /// prefab_data_override defines a different [PrefabData] for an prefab root entity, this is used for updating a prefab.
    pub fn new_with_prefab_data_override(
        entities: &EntitiesData,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
        prefab_data_override: Option<(EntityId, Arc<PrefabData>)>,
    ) -> (Self, HashMap<EntityId, u64>) {
        // prepare entity maps for values comparison between entities and prefabs,
        // and prepare prefab_root_entity to nested_prefabs_index map for id path generation
        let mut entity_maps: HashMap<_, HashMap<_, _>> = HashMap::new(); // prefab_root_entity_id in entities -> entity_id_with_path in prefab -> entity_id in entities
        let mut prefab_root_entity_to_nested_prefabs_index = HashMap::new();
        let mut nested_prefabs = Vec::new();
        for (&id, entity_data) in entities {
            if let Some((
                prefab_asset,
                prefab_entity_index,
                prefab_entity_path,
                prefab_root_entity,
            )) = entity_data.prefab_info()
            {
                if entities.contains_key(&prefab_root_entity) {
                    entity_maps.entry(prefab_root_entity).or_default().insert(
                        EntityIdWithPath(
                            EntityId::new_from_index_and_gen(prefab_entity_index, 0),
                            prefab_entity_path.clone(),
                        ),
                        id,
                    );
                    prefab_root_entity_to_nested_prefabs_index
                        .entry(prefab_root_entity)
                        .or_insert_with(|| {
                            nested_prefabs.push(prefab_asset);
                            nested_prefabs.len() as u64 - 1
                        });
                }
            }
        }

        // calculate prefab contents
        let mut prefab_entities = IndexMap::new();
        let mut prefab_delete = IndexMap::<_, IndexSet<_>>::new();
        let mut existing_entities_in_prefabs = HashSet::new();
        for (&id, entity_data) in entities {
            if let Some((
                prefab_asset,
                prefab_entity_index,
                prefab_entity_path,
                prefab_root_entity,
            )) = entity_data.prefab_info()
            {
                // we only consider this entity to belong to a nested prefab if the root entity of this entity's prefab is also in entities
                if entities.contains_key(&prefab_root_entity) {
                    // get the prefab data of this entity
                    if let Some(prefab_data) = &prefab_data_override
                        .as_ref()
                        .filter(|(root_entity, _)| *root_entity == prefab_root_entity)
                        .map(|(_, prefab_data)| Some(prefab_data.clone()))
                        .unwrap_or_else(|| get_prefab_data_fn(prefab_asset))
                    {
                        // get the entity data in this entity's prefab
                        let prefab_entity =
                            EntityId::new_from_index_and_gen(prefab_entity_index, 0);
                        if let Some(prefab_entity_data) = prefab_data.get_entity_data(
                            prefab_entity,
                            prefab_entity_path,
                            entity_maps
                                .get(&prefab_root_entity)
                                .as_ref()
                                .expect("All prefab root entity should be in entity_maps!"),
                            get_prefab_data_fn,
                        ) {
                            // get the index in nested_prefabs of this entity
                            let nested_prefab_index = *prefab_root_entity_to_nested_prefabs_index
                                .get(&prefab_root_entity)
                                .expect("prefab_root_entity_to_nested_prefabs_index should have been initialized!");

                            // generate EntityIdWithPath in new prefab
                            let mut path = vec![nested_prefab_index];
                            path.extend(prefab_entity_path);
                            let entity_id_with_path = EntityIdWithPath(prefab_entity, path);

                            // compare entity_data and prefab_entity_data and record differences
                            for (component_name, component_data) in &entity_data.components {
                                if component_name == "Prefab" {
                                    continue; // not store Prefab component because it is noly used at runtime
                                }

                                if let Some(prefab_component_data) =
                                    prefab_entity_data.components.get(component_name)
                                {
                                    // this component is in both entity_data and prefab_entity_data, compare them
                                    for (data_name, data_value) in &component_data.values {
                                        if let Some(prefab_data_value) =
                                            prefab_component_data.values.get(data_name)
                                        {
                                            // this value is in both component_data and prefab_component_data, record difference if not equal
                                            if data_value != prefab_data_value {
                                                Self::record_value(
                                                    entity_id_with_path.clone(),
                                                    component_name,
                                                    data_name,
                                                    data_value,
                                                    component_data.limits.get(data_name),
                                                    entities,
                                                    &prefab_root_entity_to_nested_prefabs_index,
                                                    &mut prefab_entities,
                                                );
                                            }
                                        } else {
                                            // this value is only in component_data, record difference
                                            Self::record_value(
                                                entity_id_with_path.clone(),
                                                component_name,
                                                data_name,
                                                data_value,
                                                component_data.limits.get(data_name),
                                                entities,
                                                &prefab_root_entity_to_nested_prefabs_index,
                                                &mut prefab_entities,
                                            );
                                        }
                                    }
                                } else {
                                    // this component is not in the prefab, record this component

                                    // it is important to insert component here because there will be no chance for empty data component to be inserted
                                    prefab_entities
                                        .entry(entity_id_with_path.clone())
                                        .or_default()
                                        .0
                                        .components
                                        .entry(component_name.clone())
                                        .or_default();

                                    // record data values in this component
                                    for (data_name, data_value) in &component_data.values {
                                        Self::record_value(
                                            entity_id_with_path.clone(),
                                            component_name,
                                            data_name,
                                            data_value,
                                            component_data.limits.get(data_name),
                                            entities,
                                            &prefab_root_entity_to_nested_prefabs_index,
                                            &mut prefab_entities,
                                        );
                                    }
                                }
                            }

                            // record deleted components in this entity
                            let delete_components = prefab_entity_data
                                .components
                                .keys()
                                .filter(|&c| !entity_data.components.contains_key(c))
                                .cloned()
                                .collect::<IndexSet<_>>();
                            if !delete_components.is_empty() {
                                prefab_delete
                                    .entry(entity_id_with_path.clone())
                                    .or_default()
                                    .extend(delete_components);
                            }

                            // record that this entity is not deleted
                            existing_entities_in_prefabs.insert(entity_id_with_path);

                            // current entity is a prefab and has been saved as nested prefab
                            continue;
                        }
                    }
                }
            }
            // current entity is not a prefab
            for (component_name, component_data) in &entity_data.components {
                if component_name == "Prefab" {
                    continue; // not store Prefab component because it is noly used at runtime
                }

                // entity id path of entity not in prefab is empty
                let entity_id_with_path = EntityIdWithPath(id, EntityIdPath::new());

                // it is important to insert component here because there will be no chance for empty data component to be inserted
                prefab_entities
                    .entry(entity_id_with_path.clone())
                    .or_default()
                    .0
                    .components
                    .entry(component_name.clone())
                    .or_default();

                // record data values in this component
                for (data_name, data_value) in &component_data.values {
                    Self::record_value(
                        entity_id_with_path.clone(),
                        component_name,
                        data_name,
                        data_value,
                        component_data.limits.get(data_name),
                        entities,
                        &prefab_root_entity_to_nested_prefabs_index,
                        &mut prefab_entities,
                    );
                }
            }
        }

        // record deleted entities.
        for (i, &nested_prefab) in nested_prefabs.iter().enumerate() {
            if let Some(prefab_data) = get_prefab_data_fn(nested_prefab) {
                // get all entities in this nested prefab
                let (_, nested_entity_map) = prefab_data.to_entities_data(get_prefab_data_fn);
                for (EntityIdWithPath(e, p), _) in nested_entity_map {
                    // generate EntityIdWithPath in new prefab
                    let mut path = vec![i as u64];
                    path.extend(p);
                    let entity_id_with_path = EntityIdWithPath(e, path);

                    // if this entity exists in prefab but not in entities data, it means that this entity was deleted
                    if !existing_entities_in_prefabs.contains(&entity_id_with_path) {
                        // empty value means that this entity is deleted
                        prefab_delete.insert(entity_id_with_path, Default::default());
                    }
                }
            }
        }

        (
            PrefabData {
                entities: prefab_entities,
                nested_prefabs,
                delete: prefab_delete,
            },
            prefab_root_entity_to_nested_prefabs_index,
        )
    }

    /// Record value in prefab_entities.
    fn record_value(
        entity_id_with_path: EntityIdWithPath,
        component_name: &String,
        data_name: &String,
        data_value: &Value,
        data_limit: Option<&Limit>,
        entities: &EntitiesData,
        prefab_root_entity_to_nested_prefabs_index: &HashMap<EntityId, u64>,
        prefab_entities: &mut IndexMap<EntityIdWithPath, EntityDataWithIdPaths>,
    ) {
        let entity_data_override = prefab_entities.entry(entity_id_with_path).or_default();
        let component_data_override = entity_data_override
            .0
            .components
            .entry(component_name.clone())
            .or_default();

        // for EntityId value, we need to also store its id path,
        // or set them to Entity::dead() if it's not in the prefab_entities
        let data_value = Self::convert_value(
            component_name,
            data_name,
            data_value,
            entities,
            prefab_root_entity_to_nested_prefabs_index,
            &mut entity_data_override.1,
        );

        if let Some(data_limit) = data_limit {
            // also record the limit if exists, because PrefabData will be cut
            // before saving to file, and the cutting process depends on the limit
            component_data_override.insert_with_limit(data_name, data_value, data_limit.clone());
        } else {
            component_data_override.insert(data_name, data_value);
        }
    }

    /// Convert entity ids in [Value] to entity id with path in prefab.
    pub(crate) fn convert_value(
        component_or_unique_name: &String,
        data_name: &String,
        data_value: &Value,
        entities: &EntitiesData,
        prefab_root_entity_to_nested_prefabs_index: &HashMap<EntityId, u64>,
        id_paths: &mut DataEntityIdPaths,
    ) -> Value {
        data_value.map_entity_and_insert_id_paths(
            data_name,
            component_or_unique_name,
            id_paths,
            |e| Self::convert_entity(e, entities, prefab_root_entity_to_nested_prefabs_index),
        )
    }

    /// Convert entity id in entities to entity id with path in prefab.
    fn convert_entity(
        e: EntityId,
        entities: &EntitiesData,
        prefab_root_entity_to_nested_prefabs_index: &HashMap<EntityId, u64>,
    ) -> (EntityId, Option<EntityIdPath>) {
        if let Some(entity_data) = entities.get(&e) {
            if let Some((prefab_entity_index, id_path)) = entity_data.prefab_info().and_then(
                |(_, prefab_entity_index, prefab_entity_path, prefab_root_entity)| {
                    prefab_root_entity_to_nested_prefabs_index
                        .get(&prefab_root_entity)
                        .map(|&nested_prefab_index| {
                            let mut id_path = vec![nested_prefab_index];
                            id_path.extend(prefab_entity_path);
                            (prefab_entity_index, id_path)
                        })
                },
            ) {
                // this entity is in the nested prefab, we need to store it's id_path
                (
                    EntityId::new_from_index_and_gen(prefab_entity_index, 0),
                    Some(id_path),
                )
            } else {
                // this entity is not in the nested prefab, we store it's id_path as empty
                (e, Some(EntityIdPath::new()))
            }
        } else {
            // this entity is not in the prefab_entities, set it to EntityId::dead()
            (EntityId::dead(), None)
        }
    }

    /// Get EntityData of entity_id with id_path.
    fn get_entity_data(
        &self,
        id: EntityId,
        id_path: &EntityIdPath,
        entity_map: &HashMap<EntityIdWithPath, EntityId>,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>>,
    ) -> Option<EntityData> {
        self.get_entity_data_recursive(
            id,
            id_path,
            0,
            entity_map,
            Default::default(),
            &mut HashSet::new(),
            get_prefab_data_fn,
        )
    }

    fn get_entity_data_recursive(
        &self,
        id: EntityId,
        id_path: &EntityIdPath,
        path_index: usize,
        entity_map: &HashMap<EntityIdWithPath, EntityId>,
        mut data_override: EntityDataWithIdPaths,
        deleted_components: &mut HashSet<String>,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>>,
    ) -> Option<EntityData> {
        if path_index < id_path.len() {
            // find entity in nested prefab
            if (id_path[path_index] as usize) < self.nested_prefabs.len() {
                if let Some(nested_prefab_data) =
                    get_prefab_data_fn(self.nested_prefabs[id_path[path_index] as usize])
                {
                    // get entity id with path in self prefab
                    let entity_id_with_path =
                        EntityIdWithPath(id, id_path[path_index..].iter().cloned().collect());

                    // get the components or entity that are deleted in self prefab
                    if let Some(self_deleted_components) = self.delete.get(&entity_id_with_path) {
                        if self_deleted_components.is_empty() {
                            // empty set means that this entity is deleted
                            return None;
                        }
                        // record deleted components in self prefab
                        deleted_components.extend(self_deleted_components.clone());
                    }

                    // get override entity data for this entity in sef prefab
                    if let Some(current_data_overide) = self.entities.get(&entity_id_with_path) {
                        data_override.merge(
                            current_data_overide,
                            deleted_components,
                            &id_path[..path_index],
                        );
                    }

                    // get entity data from nested prefab
                    return nested_prefab_data.get_entity_data_recursive(
                        id,
                        id_path,
                        path_index + 1,
                        entity_map,
                        data_override,
                        deleted_components,
                        get_prefab_data_fn,
                    );
                }
            }
            None
        } else {
            // find entity in self prefab
            self.entities
                .get(&EntityIdWithPath(id, Vec::new()))
                .map(|entity_data| {
                    data_override.merge(entity_data, deleted_components, id_path);
                    data_override.map(entity_map)
                })
        }
    }

    /// Cut useless data in self before saving to file:
    /// 1. Erase generation value of [EntityId].
    /// 2. Skip read only values.
    pub fn cut(&mut self) {
        let entities = std::mem::take(&mut self.entities);
        for (mut id_with_path, mut entity_data_with_paths) in entities {
            id_with_path.0 = Data::erase_generation(id_with_path.0);
            entity_data_with_paths.0 = entity_data_with_paths.0.cut();
            self.entities.insert(id_with_path, entity_data_with_paths);
        }
    }

    /// Convert [PrefabData] to [EntitiesData]. Also return a map that maps every
    /// [EntityIdWithPath] in this [PrefabData] to a new [EntityId] in [EntitiesData].
    pub fn to_entities_data(
        &self,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
    ) -> (EntitiesData, HashMap<EntityIdWithPath, EntityId>) {
        let mut entity_map = self.prepare_entity_map(get_prefab_data_fn);
        let mut entities_data = EntitiesData::default();
        let mut keys_to_remove = Vec::new();
        for (EntityIdWithPath(id, path), &entity_id) in &entity_map {
            if let Some(entity_data) =
                self.get_entity_data(*id, path, &entity_map, get_prefab_data_fn)
            {
                entities_data.insert(entity_id, entity_data);
            } else {
                // this entity is deleted, we should remove it from entity_map
                keys_to_remove.push(EntityIdWithPath(*id, path.clone()));
            }
        }
        for key in keys_to_remove {
            entity_map.remove(&key);
        }

        // we must fix hierarchy components here, because a Children component may
        // be overrided by multiple prefabs, which will cause abcense of entities
        let mut children_to_add = HashMap::new();
        for (e, entity_data) in &entities_data {
            let parent = entity_data.parent();
            if let Some(parent_entity_data) = entities_data.get(&parent) {
                if !parent_entity_data
                    .children()
                    .is_some_and(|children| children.contains(e))
                {
                    children_to_add
                        .entry(parent)
                        .or_insert_with(Vec::new)
                        .push(*e);
                }
            }
        }
        for (parent, children) in children_to_add {
            let parent_entity_data = entities_data.get_mut(&parent).unwrap();
            parent_entity_data.children_mut().unwrap().extend(children);
        }
        for entity_data in &mut entities_data.values_mut() {
            entity_data.trim_children();
        }

        (entities_data, entity_map)
    }

    /// Create a map that maps every [EntityIdWithPath] in self prefab to a new [EntityId].
    /// Note that the map returned by this function may contains entities that should have been deleted in self prefab.
    /// If you want to get the exact entities in self prefab, you should use [Self::to_entities_data].
    fn prepare_entity_map(
        &self,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
    ) -> HashMap<EntityIdWithPath, EntityId> {
        let mut entity_map = HashMap::new();
        self.prepare_entity_map_recursive(
            get_prefab_data_fn,
            &mut Vec::new(),
            &mut 0,
            &mut HashSet::new(),
            &mut entity_map,
        );
        entity_map
    }

    fn prepare_entity_map_recursive(
        &self,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
        id_path_prefix: &mut Vec<u64>,
        entity_id_index: &mut u64,
        deleted_entities: &mut HashSet<EntityIdWithPath>,
        entity_map: &mut HashMap<EntityIdWithPath, EntityId>,
    ) {
        for (EntityIdWithPath(id, path), deleted_components) in &self.delete {
            // empty components list means that this entity is deleted.
            if deleted_components.is_empty() {
                let mut id_path = id_path_prefix.clone();
                id_path.extend(path);
                deleted_entities.insert(EntityIdWithPath(*id, id_path));
            }
        }
        for EntityIdWithPath(id, path) in self.entities.keys() {
            let mut id_path = id_path_prefix.clone();
            id_path.extend(path);
            let entity_id_with_path = EntityIdWithPath(*id, id_path);
            if !deleted_entities.contains(&entity_id_with_path) {
                entity_map.entry(entity_id_with_path).or_insert_with(|| {
                    *entity_id_index += 1;
                    EntityId::new_from_index_and_gen(*entity_id_index - 1, 0)
                });
            }
        }
        for (i, &asset_id) in self.nested_prefabs.iter().enumerate() {
            if let Some(prefab_data) = get_prefab_data_fn(asset_id) {
                id_path_prefix.push(i as u64);
                Self::prepare_entity_map_recursive(
                    &prefab_data,
                    get_prefab_data_fn,
                    id_path_prefix,
                    entity_id_index,
                    deleted_entities,
                    entity_map,
                );
                id_path_prefix.pop();
            }
        }
    }

    /// Update prefab data according to the input entities data which created from an exisiting prefab,
    /// also return entity_id_to_prefab_entity_id_with_path map. The first entity of input entities must be the root.
    pub fn update(
        entities: EntitiesData,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
    ) -> Result<(Self, HashMap<EntityId, EntityIdWithPath>), String> {
        // get self prefab asset id and old prefab data
        let (&root_entity, root_entity_data) = entities
            .get_index(0)
            .expect("PrefabData::update: no entity found");
        let (self_prefab_asset, _, _, self_prefab_root_entity) = root_entity_data
            .prefab_info()
            .expect("PrefabData::update: no prefab info in root entity");
        assert!(
            root_entity == self_prefab_root_entity,
            "PrefabData::update: first entity is not a prefab root"
        );
        let old_prefab_data =
            get_prefab_data_fn(self_prefab_asset).ok_or("PrefabData::update: prefab not found")?;

        // remove other entities that are or nest the same prefab to prevent prefabs from nesting themselves
        let mut prefab_asset_to_nested_self = HashMap::new();
        let entities = EntitiesData(
            entities
                .into_iter()
                .filter(|(_, entity_data)| {
                    if let Some((prefab_asset, _, _, prefab_root_entity)) =
                        entity_data.prefab_info()
                    {
                        if prefab_root_entity != root_entity {
                            if prefab_asset == self_prefab_asset {
                                return false;
                            } else {
                                if !prefab_asset_to_nested_self.contains_key(&prefab_asset) {
                                    prefab_asset_to_nested_self.insert(
                                        prefab_asset,
                                        get_prefab_data_fn(prefab_asset)
                                            .map(|prefab_data| {
                                                prefab_data
                                                    .all_nested_prefabs(get_prefab_data_fn)
                                                    .contains(&self_prefab_asset)
                                            })
                                            .unwrap_or(false),
                                    );
                                }
                                return !prefab_asset_to_nested_self[&prefab_asset];
                            }
                        }
                    }
                    true
                })
                .collect(),
        );

        // remove all entities that were detached due to previous deletions
        let attached_map = entities.check_attached_to_first_entity();
        let entities = EntitiesData(
            entities
                .into_iter()
                .filter(|(e, _)| attached_map[e])
                .collect(),
        );

        // map entity ids in entities to entity ids in the original prefab data.
        let mut entity_map = HashMap::new();
        let mut current_entity_index = 0;
        let prefab_entity_set = old_prefab_data
            .entities
            .keys()
            .filter(|EntityIdWithPath(_, p)| p.is_empty())
            .map(|EntityIdWithPath(e, _)| *e)
            .collect::<HashSet<_>>();
        for (&e, entity_data) in &entities {
            if let Some((_, prefab_entity_index, prefab_entity_path, prefab_root)) =
                entity_data.prefab_info()
            {
                if prefab_root == root_entity && prefab_entity_path.is_empty() {
                    entity_map.insert(e, EntityId::new_from_index_and_gen(prefab_entity_index, 0));
                    continue;
                }
            }
            loop {
                let new_entity_id = EntityId::new_from_index_and_gen(current_entity_index, 0);
                if !prefab_entity_set.contains(&new_entity_id) {
                    entity_map.insert(e, new_entity_id);
                    current_entity_index += 1;
                    break;
                }
                current_entity_index += 1;
            }
        }
        let mut mapped_entities = EntitiesData::default();
        for (e, mut entity_data) in entities {
            let e = entity_map[&e];
            entity_data.components.iter_mut().for_each(|(_, data)| {
                data.values.iter_mut().for_each(|(_, v)| {
                    v.iter_entity_mut(|e| *e = entity_map.get(e).cloned().unwrap_or_default());
                })
            });
            mapped_entities.insert(e, entity_data);
        }

        // revert prefab infos in mapped entities
        let mut nested_root_map = vec![None; old_prefab_data.nested_prefabs.len()];
        for (&e, entity_data) in &mapped_entities {
            if let Some((prefab_asset, prefab_entity_index, prefab_entity_path, _)) =
                entity_data.prefab_info()
            {
                // if prefab_asset == self_prefab_asset, this entity must be in self prefab,
                // because we have removed any entities that may cause self nesting
                if prefab_asset == self_prefab_asset && !prefab_entity_path.is_empty() {
                    if let Some(nested_prefab_data) = get_prefab_data_fn(
                        old_prefab_data.nested_prefabs[prefab_entity_path[0] as usize],
                    ) {
                        let EntityIdWithPath(
                            nested_prefab_root_entity,
                            nested_prefab_root_entity_path,
                        ) = nested_prefab_data
                            .entities
                            .get_index(0)
                            .expect("PrefabData::update: prefab should have at least one entity!")
                            .0;
                        if nested_prefab_root_entity.index() == prefab_entity_index
                            && *nested_prefab_root_entity_path == prefab_entity_path[1..]
                        {
                            nested_root_map[prefab_entity_path[0] as usize] = Some(e);
                        }
                    }
                }
            }
        }
        for (_, entity_data) in &mut mapped_entities {
            if let Some((prefab_asset, _, prefab_entity_path, _)) = entity_data.prefab_info() {
                // if prefab_asset == self_prefab_asset, this entity must be in self prefab,
                // because we have removed any entities that may cause self nesting
                if prefab_asset == self_prefab_asset {
                    if prefab_entity_path.is_empty() {
                        entity_data.components.shift_remove("Prefab");
                    } else if let Some(revert_prefab_root_entity) =
                        nested_root_map[prefab_entity_path[0] as usize]
                    {
                        let revert_prefab_asset =
                            old_prefab_data.nested_prefabs[prefab_entity_path[0] as usize];
                        let revert_prefab_entity_path = prefab_entity_path[1..].to_vec();
                        entity_data.update_prefab_info(
                            revert_prefab_asset,
                            revert_prefab_entity_path,
                            revert_prefab_root_entity,
                        );
                    } else {
                        // the root entity of this prefab is not found, maybe it was removed
                        entity_data.components.shift_remove("Prefab");
                    }
                }
            }
        }

        // create new prefab data
        let (new_prefab_data, prefab_root_entity_to_nested_prefabs_index) =
            PrefabData::new(mapped_entities.clone(), get_prefab_data_fn);

        // create entity_id to prefab_entity_id_with_path map for updating prefab
        let mut entity_id_to_prefab_entity_id_with_path = HashMap::new();
        // map back entity ids in entities.
        let entity_map = entity_map
            .into_iter()
            .map(|(k, v)| (v, k))
            .collect::<HashMap<_, _>>();
        for (e, entity_data) in &mapped_entities {
            if let Some((_, prefab_entity_index, prefab_entity_path, prefab_root_entity)) =
                entity_data.prefab_info()
            {
                let mut entity_path =
                    vec![prefab_root_entity_to_nested_prefabs_index[&prefab_root_entity]];
                entity_path.extend(prefab_entity_path);
                entity_id_to_prefab_entity_id_with_path.insert(
                    entity_map[e],
                    EntityIdWithPath(
                        EntityId::new_from_index_and_gen(prefab_entity_index, 0),
                        entity_path,
                    ),
                );
            } else {
                entity_id_to_prefab_entity_id_with_path
                    .insert(entity_map[e], EntityIdWithPath(*e, EntityIdPath::default()));
            }
        }

        // return the updated prefab data
        Ok((new_prefab_data, entity_id_to_prefab_entity_id_with_path))
    }

    /// Get all nested prefab asset ids in this prefab, including nested prefabs of nested prefabs.
    fn all_nested_prefabs(
        &self,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
    ) -> HashSet<AssetId> {
        let mut all_nested_prefabs = HashSet::new();
        let mut prefabs_to_search = self.nested_prefabs.clone();
        while let Some(prefab) = prefabs_to_search.pop() {
            if let Some(prefab_data) = get_prefab_data_fn(prefab) {
                all_nested_prefabs.insert(prefab);
                prefabs_to_search.extend(
                    prefab_data
                        .nested_prefabs
                        .iter()
                        .filter(|prefab| !all_nested_prefabs.contains(prefab))
                        .collect::<Vec<_>>(),
                );
            }
        }
        all_nested_prefabs
    }
}

/// EntityIdWithPath contains the entity id and its path in the prefab.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq, Hash)]
pub struct EntityIdWithPath(pub EntityId, pub EntityIdPath);

/// EntityDataWithIdPaths containts enitity data and all entity id paths in it.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EntityDataWithIdPaths(pub EntityData, pub DataEntityIdPaths);

impl EntityDataWithIdPaths {
    /// Merge other into self, leaving already existing data in self unchanged,
    /// insert front_id_path in front of all id paths in other before merging.
    pub fn merge(
        &mut self,
        other: &Self,
        exclude_components: &HashSet<String>,
        front_id_path: &[u64],
    ) {
        let insert_front_fn = |path: &EntityIdPathInValue| {
            path.map(|p: &EntityIdPath| {
                let mut r = front_id_path.iter().cloned().collect::<Vec<_>>();
                r.extend_from_slice(p);
                r
            })
        };
        for (component_name, component_data) in &other.0.components {
            if exclude_components.contains(component_name) {
                continue;
            }
            if let Some(self_component_data) = self.0.components.get_mut(component_name) {
                for (data_name, data_value) in &component_data.values {
                    if !self_component_data.values.contains_key(data_name) {
                        self_component_data
                            .values
                            .insert(data_name.clone(), data_value.clone());
                        if let Some(id_paths) = other
                            .1
                            .get(component_name)
                            .and_then(|id_paths| id_paths.get(data_name))
                        {
                            let id_paths = if front_id_path.is_empty() {
                                id_paths.clone()
                            } else {
                                // insert front id path to existing id path, so it is important to
                                // save empty id path as empty vector rather than not saving it.
                                insert_front_fn(id_paths)
                            };
                            self.1
                                .entry(component_name.clone())
                                .or_default()
                                .insert(data_name.clone(), id_paths);
                        }
                    }
                }
            } else {
                self.0
                    .components
                    .insert(component_name.clone(), component_data.clone());
                if let Some(id_paths) = other.1.get(component_name) {
                    let mut id_paths = id_paths.clone();
                    if !front_id_path.is_empty() {
                        // insert front id path to each existing id paths, so it is important to
                        // save empty id path as empty vector rather than not saving it.
                        id_paths.iter_mut().for_each(|(_, id_paths)| {
                            *id_paths = insert_front_fn(id_paths);
                        });
                    }
                    self.1.insert(component_name.clone(), id_paths);
                }
            }
        }
    }

    /// Convert all [EntityIdWithPath] to [EntityId] by entity_map.
    pub fn map(mut self, entity_map: &HashMap<EntityIdWithPath, EntityId>) -> EntityData {
        self.0.components.iter_mut().for_each(|(component_name, component_data)| {
            component_data.values.iter_mut().for_each(|(data_name, data_value)| {
                data_value.iter_entity_mut_with_path(self.1.get(component_name).and_then(|id_paths| id_paths.get(data_name)), |e, id_path| {
                    let entity_id_with_path = EntityIdWithPath(*e, id_path.cloned().unwrap_or_default());
                    *e = if let Some(e) = entity_map.get(&entity_id_with_path) {
                        *e
                    } else {
                        if *e != EntityId::dead() {
                            log::warn!("EntityDataWithIdPaths::map: {entity_id_with_path:?} not found.");
                        }
                        EntityId::dead()
                    };
                });
            });
        });
        self.0
    }
}

/// EntityIdPath is the path to an entity in a prefab. Examples:
/// 1. \[\] means EntityId is in the current Prefab. (empty id path should store as empty vector rather than not storing it)
/// 2. \[a\] means EntityId is in the nested prefab at index a.
/// 3. \[a, b\] means the EntityId is in the nested prefab at index b of the nested prefab at index a.
pub type EntityIdPath = Vec<u64>;

/// All [EntityIdPath] in [Data]. component_name/unique_name -> data_name -> entity_id_path.
pub type DataEntityIdPaths = IndexMap<String, IndexMap<String, EntityIdPathInValue>>;

/// The [EntityIdPath] in [Value].
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EntityIdPathInValue {
    /// The [EntityIdPath] for [Value::Entity].
    EntityId(EntityIdPath),
    /// All [EntityIdPath] for [Value::VecEntity]. Key is the index of entity vector, there is no entry for EntityId::dead().
    EntityVec(IndexMap<u64, EntityIdPath>),
}

impl EntityIdPathInValue {
    /// Map each [EntityIdPath] in self to another [EntityIdPath].
    fn map(&self, f: impl Fn(&EntityIdPath) -> EntityIdPath) -> Self {
        match self {
            EntityIdPathInValue::EntityId(p) => EntityIdPathInValue::EntityId(f(p)),
            EntityIdPathInValue::EntityVec(ps) => {
                EntityIdPathInValue::EntityVec(ps.iter().map(|(&i, p)| (i, f(p))).collect())
            }
        }
    }
}
