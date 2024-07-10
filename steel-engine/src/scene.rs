use std::{collections::HashMap, path::PathBuf};
use shipyard::{EntityId, Unique, UniqueView, UniqueViewMut, World};
use steel_common::{data::{Data, EntityData, Value, WorldData}, platform::Platform};
use crate::data::{ComponentRegistry, UniqueRegistry};

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

        // create new_world_data from world_data by changing old entity ids to new entity ids.
        let mut new_world_data = WorldData::new();
        let mut old_id_to_new_id = HashMap::new();
        for old_id in world_data.entities.keys() {
            old_id_to_new_id.insert(*old_id, world.add_entity(()));
        }
        for (old_id, entity_data) in &world_data.entities {
            let new_id = *old_id_to_new_id.get(old_id).unwrap();
            let mut new_entity_data = EntityData::new();
            for (component_name, component_data) in &entity_data.components {
                let new_component_data = Self::_update_data(component_data, &old_id_to_new_id);
                new_entity_data.components.insert(component_name.clone(), new_component_data);
            }
            new_world_data.entities.insert(new_id, new_entity_data);
        }
        for (unique_name, unique_data) in &world_data.uniques {
            let new_unique_data = Self::_update_data(unique_data, &old_id_to_new_id);
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

    fn _update_data(data: &Data, old_id_to_new_id: &HashMap<EntityId, EntityId>) -> Data {
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

    /// Update scene_manager.current_scene to the scene.
    pub(crate) fn set_current_scene(world: &mut World, scene: Option<PathBuf>) {
        world.run(|mut scene_manager: UniqueViewMut<SceneManager>| {
            scene_manager.current_scene = scene;
            log::info!("SceneManager.current_scene={:?}", scene_manager.current_scene);
        });
    }
}
