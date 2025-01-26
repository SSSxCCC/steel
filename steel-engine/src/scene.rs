use crate::{
    asset::AssetManager,
    data::WorldDataExt,
    prefab::{LoadScenePrefabsParam, PrefabAssets},
};
use shipyard::{AllStorages, AllStoragesViewMut, Unique, UniqueView, UniqueViewMut};
use std::collections::HashMap;
use steel_common::{asset::AssetId, data::SceneData, platform::Platform, prefab::EntityIdWithPath};

/// The SceneManager unique. You can use [SceneManager::current_scene] to get the current scene
/// and use [SceneManager::switch_scene] to change scene at the start of next frame.
#[derive(Unique)]
pub struct SceneManager {
    current_scene: Option<AssetId>,
    to_scene: Option<AssetId>,
}

impl SceneManager {
    /// Create a new SceneManager, if scene is some, we will switch to it at the start of next frame.
    pub fn new(scene: Option<AssetId>) -> Self {
        SceneManager {
            current_scene: None,
            to_scene: scene,
        }
    }

    /// Get the current scene.
    pub fn current_scene(&self) -> Option<AssetId> {
        self.current_scene
    }

    /// Switch to the scene at the start of next frame.
    pub fn switch_scene(&mut self, scene: AssetId) {
        self.to_scene = Some(scene);
    }

    /// Clear world and load world from world_data, also be sure to call Self::set_current_scene if scene has changed.
    pub(crate) fn load(all_storages: &mut AllStorages, scene_data: &SceneData) {
        // clear all entities in ecs world
        all_storages.clear();

        // clear hierachy track data since the whole hierachy tree is going to be rebuilt
        all_storages.run(crate::hierarchy::clear_track_data_system);

        // convert scene data to world data
        let get_prefab_data_fn = |prefab_asset: AssetId| {
            all_storages.run(
                |mut prefab_assets: UniqueViewMut<PrefabAssets>,
                 mut asset_manager: UniqueViewMut<AssetManager>,
                 platform: UniqueView<Platform>| {
                    prefab_assets.get_prefab_data(
                        prefab_asset,
                        &mut asset_manager,
                        platform.as_ref(),
                    )
                },
            )
        };
        let (world_data, entity_map) = scene_data.to_world_data(get_prefab_data_fn);

        // add world_data into ecs world
        let old_id_to_new_id = world_data.add_to_world(all_storages);

        // update Prefab components
        let mut prefab_asset_and_entity_id_to_prefab_entity_id_with_path = scene_data
            .entities
            .nested_prefabs
            .iter()
            .map(|&np| (np, HashMap::new()))
            .collect::<Vec<_>>();
        for (EntityIdWithPath(prefab_eid, mut p), old_id) in entity_map {
            if !p.is_empty() {
                let new_id = old_id_to_new_id
                    .get(&old_id)
                    .expect("old_id_to_new_id should contain all EntityId!");
                let i = p.remove(0) as usize;
                prefab_asset_and_entity_id_to_prefab_entity_id_with_path[i]
                    .1
                    .insert(*new_id, EntityIdWithPath(prefab_eid, p));
            }
        }
        all_storages.add_unique(LoadScenePrefabsParam {
            prefab_asset_and_entity_id_to_prefab_entity_id_with_path,
        });
        all_storages.run(crate::prefab::load_scene_prefabs_system);
        all_storages
            .remove_unique::<LoadScenePrefabsParam>()
            .unwrap();
    }

    /// Update scene_manager.current_scene to the scene.
    pub(crate) fn set_current_scene(
        &mut self,
        scene: Option<AssetId>,
        asset_manager: &AssetManager,
    ) {
        self.current_scene = scene;
        log::info!(
            "SceneManager::set_current_scene: id={:?}, path={:?}",
            self.current_scene,
            self.current_scene
                .as_ref()
                .and_then(|scene| asset_manager.get_asset_path(*scene)),
        );
    }
}

/// Load the scene which is set by SceneManager::switch_scene.
pub fn scene_maintain_system(mut all_storages: AllStoragesViewMut) {
    let scene_data_and_scene = all_storages.run(
        |mut scene_manager: UniqueViewMut<SceneManager>,
         mut asset_manager: UniqueViewMut<AssetManager>,
         platform: UniqueView<Platform>| {
            if let Some(to_scene) = scene_manager.to_scene.take() {
                if let Some(bytes) = asset_manager.get_asset_content(to_scene, &platform) {
                    match serde_json::from_slice::<SceneData>(bytes) {
                        Ok(scene_data) => return Some((scene_data, to_scene)),
                        Err(e) => log::error!(
                            "SceneManager::maintain_system: failed to load scene data, error={e:?}"
                        ),
                    }
                }
            }
            None
        },
    );
    if let Some((scene_data, scene)) = scene_data_and_scene {
        SceneManager::load(&mut all_storages, &scene_data);
        all_storages.run(
            |mut scene_manager: UniqueViewMut<SceneManager>,
             asset_manager: UniqueView<AssetManager>| {
                scene_manager.set_current_scene(Some(scene), &asset_manager);
            },
        );
    }
}
