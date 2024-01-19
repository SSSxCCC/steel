pub mod engine;
pub mod physics2d;
pub mod render2d;

pub use steel_common::*;

use std::collections::HashMap;
use render2d::Renderer2D;
use physics2d::{RigidBody2D, Collider2D};
use shipyard::{Component, IntoIter, IntoWithId, View, World, EntityId};
use glam::{Vec3, Vec2};
use log::{Log, LevelFilter, SetLoggerError};

#[no_mangle]
pub fn setup_logger(logger: &'static dyn Log, level: LevelFilter) -> Result<(), SetLoggerError> {
    log::set_max_level(level);
    log::set_logger(logger)
}

pub trait Edit: Component + Default {
    fn name() -> &'static str;

    fn get_data(&self) -> ComponentData {
        ComponentData::new(Self::name())
    }

    fn set_data(&mut self, data: &ComponentData) { }

    fn from(data: &ComponentData) -> Self {
        let mut e = Self::default();
        e.set_data(data);
        e
    }
}

pub trait WorldDataExt {
    fn with_core_components(world: &World) -> Self;
    fn add_component<T: Edit + Send + Sync>(&mut self, world: &World);
}

impl WorldDataExt for WorldData {
    fn with_core_components(world: &World) -> Self {
        let mut world_data = WorldData::new();
        world_data.add_component::<Transform2D>(world);
        world_data.add_component::<RigidBody2D>(world);
        world_data.add_component::<Collider2D>(world);
        world_data.add_component::<Renderer2D>(world);
        world_data
    }

    fn add_component<T: Edit + Send + Sync>(&mut self, world: &World) {
        world.run(|c: View<T>| {
            for (e, c) in c.iter().with_id() {
                let len = self.entities.len();
                let index = *self.entity_index_map().entry(e).or_insert(len);
                if index == self.entities.len() {
                    self.entities.push(EntityData { id: e, components: Vec::new() });
                }
                self.entities[index].components.push(c.get_data());
            }
        })
    }
}

pub trait WorldExt {
    fn create_components(&mut self, data: &WorldData);
    fn create_component<T: Edit + Send + Sync>(&mut self, id: EntityId, data: &ComponentData);
}

impl WorldExt for World {
    fn create_components(&mut self, data: &WorldData) {
        self.clear();
        let mut old_id_to_new_id = HashMap::new();
        for entity_data in &data.entities {
            let id = *old_id_to_new_id.entry(entity_data.id).or_insert_with(|| self.add_entity(()));
            for component_data in &entity_data.components {
                self.create_component::<Transform2D>(id, component_data);
                self.create_component::<RigidBody2D>(id, component_data);
                self.create_component::<Collider2D>(id, component_data);
                self.create_component::<Renderer2D>(id, component_data);
            }
        }
    }

    fn create_component<T: Edit + Send + Sync>(&mut self, id: EntityId, data: &ComponentData) {
        if T::name() == data.name {
            self.add_component(id, (T::from(data),));
        }
    }
}

#[derive(Component, Debug, Default)]
pub struct Transform2D {
    pub position: Vec3,
    pub rotation: f32, // radian
    pub scale: Vec2
}

impl Edit for Transform2D {
    fn name() -> &'static str { "Transform2D" }

    fn get_data(&self) -> ComponentData {
        let mut data = ComponentData::new(Self::name());
        data.variants.push(Variant::new("position", Value::Vec3(self.position)));
        data.variants.push(Variant::new("rotation", Value::Float32(self.rotation)));
        data.variants.push(Variant::new("scale", Value::Vec2(self.scale)));
        data
    }

    fn set_data(&mut self, data: &ComponentData) {
        let value_map = data.value_map();
        if let Some(Value::Vec3(v)) = value_map.get("position") { self.position = *v }
        if let Some(Value::Float32(f)) = value_map.get("rotation") { self.rotation = *f }
        if let Some(Value::Vec2(v)) = value_map.get("scale") { self.scale = *v }
    }
}
