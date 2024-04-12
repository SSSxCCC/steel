use std::{collections::HashMap, path::PathBuf};
use shipyard::{Unique, UniqueView, UniqueViewMut, World};
use steel_common::{data::WorldData, platform::Platform};
use crate::data::{ComponentFns, UniqueFns};

#[derive(Unique)]
pub struct SceneManager {
    current_scene: Option<PathBuf>,
    pub to_scene: Option<PathBuf>,
}

impl SceneManager {
    pub fn new(scene: Option<PathBuf>) -> Self {
        SceneManager { current_scene: None, to_scene: scene }
    }

    pub fn current_scene(&self) -> Option<&PathBuf> {
        self.current_scene.as_ref()
    }

    pub fn switch_scene(&mut self, scene: PathBuf) {
        self.to_scene = Some(scene);
    }

    pub fn maintain_system(world: &mut World, component_fns: &ComponentFns, unique_fns: &UniqueFns) {
        let world_data_and_scene = world.run(|mut scene_manager: UniqueViewMut<SceneManager>, platform: UniqueView<Platform>| {
            if let Some(to_scene) = scene_manager.to_scene.take() {
                if let Some(world_data) = WorldData::load_from_file(&to_scene, &platform) {
                    return Some((world_data, to_scene));
                }
            }
            None
        });
        if let Some((world_data, scene)) = world_data_and_scene {
            Self::load(world, &world_data, component_fns, unique_fns);
            Self::set_current_scene(world, Some(scene));
        }
    }

    /// clear world and load world from world_data, also be sure to call Self::set_current_scene if scene path has changed
    pub fn load(world: &mut World, world_data: &WorldData, component_fns: &ComponentFns, unique_fns: &UniqueFns) {
        world.clear();
        let mut old_id_to_new_id = HashMap::new();
        for (old_id, entity_data) in &world_data.entities {
            let new_id = *old_id_to_new_id.entry(old_id).or_insert_with(|| world.add_entity(()));
            for (component_name, component_data) in &entity_data.components {
                if let Some(component_fn) = component_fns.get(component_name.as_str()) {
                    (component_fn.create_with_data)(world, new_id, component_data);
                }
            }
        }
        for unique_fn in unique_fns.values() {
            (unique_fn.load_from_data)(world, world_data);
        }
    }

    pub fn set_current_scene(world: &mut World, scene: Option<PathBuf>) {
        world.run(|mut scene_manager: UniqueViewMut<SceneManager>| {
            scene_manager.current_scene = scene;
            log::info!("SceneManager.current_scene={:?}", scene_manager.current_scene);
        });
    }
}
