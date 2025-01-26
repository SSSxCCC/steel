pub use steel_common::prefab::*;

use crate::{
    asset::AssetManager,
    data::EntitiesDataExt,
    edit::Edit,
    hierarchy::{Children, Parent},
};
use shipyard::{
    AddComponent, AllStorages, Component, EntityId, Get, Unique, UniqueView, UniqueViewMut, View,
    ViewMut,
};
use std::{collections::HashMap, error::Error, sync::Arc};
use steel_common::{
    asset::AssetId,
    data::{Data, Limit, Value},
    platform::Platform,
};

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

/// Add entities to the ecs world from a prefab, return the root entity id of the prefab.
pub fn add_entities_from_prefab(
    all_storages: &AllStorages,
    prefab_asset: AssetId,
) -> Result<EntityId, Box<dyn Error>> {
    let get_prefab_data_fn = |prefab_asset: AssetId| {
        all_storages.run(
            |mut prefab_assets: UniqueViewMut<PrefabAssets>,
             mut asset_manager: UniqueViewMut<AssetManager>,
             platform: UniqueView<Platform>| {
                prefab_assets.get_prefab_data(
                    prefab_asset,
                    asset_manager.as_mut(),
                    platform.as_ref(),
                )
            },
        )
    };
    let prefab_data = get_prefab_data_fn(prefab_asset).ok_or("failed to get prefab data!")?;
    let (entities_data, entity_map) = prefab_data.to_entities_data(get_prefab_data_fn);
    let old_id_to_new_id = entities_data.add_to_world(all_storages);

    // prefab asset is successfully loaded, we must update Prefab components
    let mut entity_id_to_prefab_entity_id_with_path = HashMap::new();
    for (entity_id_with_path, old_id) in entity_map {
        let new_id = *old_id_to_new_id
            .get(&old_id)
            .ok_or("old_id_to_new_id did not contain all EntityId!")?;
        entity_id_to_prefab_entity_id_with_path.insert(new_id, entity_id_with_path);
    }
    let prefab_root_entity = *entities_data
        .root()
        .and_then(|e| old_id_to_new_id.get(&e))
        .ok_or("Could not find root entity in prefab!")?;
    all_storages.add_unique(LoadPrefabParam {
        prefab_root_entity,
        prefab_asset,
        entity_id_to_prefab_entity_id_with_path,
    });
    all_storages.run(crate::prefab::load_prefab_system);
    all_storages.remove_unique::<LoadPrefabParam>().unwrap();

    Ok(prefab_root_entity)
}
