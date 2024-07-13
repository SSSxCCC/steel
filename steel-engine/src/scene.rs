use std::path::PathBuf;
use shipyard::{Unique, UniqueView, UniqueViewMut, World};
use steel_common::{data::WorldData, platform::Platform};
use crate::data::{ComponentRegistry, UniqueRegistry, WorldDataExt};

/// The SceneManager unique. You can use SceneManager::current_scene to get the current scene
/// and use SceneManager::switch_scene to change scene at the start of next frame.
/// Note that the scene is a PathBuf which is relative to the asset folder.
#[derive(Unique)]
pub struct SceneManager {
    current_scene: Option<PathBuf>,
    to_scene: Option<PathBuf>,
}

impl SceneManager {
    /// Create a new SceneManager, if scene is some, we will switch to it at the start of next frame.
    pub fn new(scene: Option<PathBuf>) -> Self {
        SceneManager { current_scene: None, to_scene: scene }
    }

    /// Get the current scene.
    pub fn current_scene(&self) -> Option<&PathBuf> {
        self.current_scene.as_ref()
    }

    /// Switch to the scene at the start of next frame.
    pub fn switch_scene(&mut self, scene: PathBuf) {
        self.to_scene = Some(scene);
    }

    /// Load the scene which is set by SceneManager::switch_scene.
    pub fn maintain_system(world: &mut World, component_registry: &ComponentRegistry, unique_registry: &UniqueRegistry) {
        let world_data_and_scene = world.run(|mut scene_manager: UniqueViewMut<SceneManager>, platform: UniqueView<Platform>| {
            if let Some(to_scene) = scene_manager.to_scene.take() {
                if let Some(world_data) = WorldData::load_from_file(&to_scene, &platform) {
                    return Some((world_data, to_scene));
                }
            }
            None
        });
        if let Some((world_data, scene)) = world_data_and_scene {
            Self::load(world, &world_data, component_registry, unique_registry);
            Self::set_current_scene(world, Some(scene));
        }
    }

    /// Clear world and load world from world_data, also be sure to call Self::set_current_scene if scene path has changed.
    pub(crate) fn load(world: &mut World, world_data: &WorldData, component_registry: &ComponentRegistry, unique_registry: &UniqueRegistry) {
        // clear all entities in ecs world.
        world.clear();

        // clear hierachy track data since the whole hierachy tree is going to be rebuilt.
        world.run(crate::hierarchy::clear_track_data_system);

        world_data.add_to_world(world, component_registry, unique_registry);
    }

    /// Update scene_manager.current_scene to the scene.
    pub(crate) fn set_current_scene(world: &mut World, scene: Option<PathBuf>) {
        world.run(|mut scene_manager: UniqueViewMut<SceneManager>| {
            scene_manager.current_scene = scene;
            log::info!("SceneManager.current_scene={:?}", scene_manager.current_scene);
        });
    }
}
