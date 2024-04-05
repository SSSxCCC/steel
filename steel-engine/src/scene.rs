use std::{collections::HashMap, path::PathBuf};
use shipyard::{Unique, UniqueView, UniqueViewMut, World};
use steel_common::{data::WorldData, platform::Platform};

use crate::data::ComponentFns;

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

    pub fn maintain_system(world: &mut World, component_fns: &ComponentFns) {
        let world_data = world.run(|mut scene_manager: UniqueViewMut<SceneManager>, platform: UniqueView<Platform>| {
            if let Some(to_scene) = scene_manager.to_scene.take() {
                let world_data = WorldData::load_from_file(&to_scene, &platform);
                if world_data.is_some() {
                    scene_manager.current_scene = Some(to_scene);
                }
                return world_data;
            }
            None
        });
        if let Some(world_data) = world_data {
            Self::load(world, &world_data, component_fns);
        }
    }

    pub fn load(world: &mut World, world_data: &WorldData, component_fns: &ComponentFns) {
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
    }
}
