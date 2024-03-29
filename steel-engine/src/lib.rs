pub mod engine;
pub mod transform;
pub mod camera;
pub mod physics2d;
pub mod render;

pub use steel_common::*;

use camera::Camera;
use indexmap::IndexMap;
use transform::Transform;
use std::collections::HashMap;
use render::renderer2d::Renderer2D;
use physics2d::{RigidBody2D, Collider2D};
use shipyard::{Component, IntoIter, IntoWithId, View, World, EntityId, ViewMut, track::{Untracked, All}};
use log::{Log, LevelFilter, SetLoggerError};

#[no_mangle]
pub fn setup_logger(logger: &'static dyn Log, level: LevelFilter) -> Result<(), SetLoggerError> {
    log::set_max_level(level);
    log::set_logger(logger)
}

pub trait Edit: Component + Default {
    fn name() -> &'static str;

    fn get_data(&self) -> ComponentData {
        ComponentData::new()
    }

    fn set_data(&mut self, #[allow(unused)] data: &ComponentData) { }

    fn from(data: &ComponentData) -> Self {
        let mut e = Self::default();
        e.set_data(data);
        e
    }
}

#[derive(Component, Debug, Default)]
pub struct EntityInfo {
    pub name: String,
    // steel-editor can read EntityId from EntityData so that we don't need to store EntityId here
}

impl EntityInfo {
    pub fn new(name: impl Into<String>) -> Self {
        EntityInfo { name: name.into() }
    }
}

impl Edit for EntityInfo {
    fn name() -> &'static str { "EntityInfo" }

    fn get_data(&self) -> ComponentData {
        let mut data = ComponentData::new();
        data.values.insert("name".into(), Value::String(self.name.clone()));
        data
    }

    fn set_data(&mut self, data: &ComponentData) {
        if let Some(Value::String(s)) = data.values.get("name") { self.name = s.clone() }
    }
}

pub trait WorldDataExt {
    fn with_core_components(world: &World) -> Self;
    fn add_component<T: Edit + Send + Sync>(&mut self, world: &World);
}

impl WorldDataExt for WorldData {
    fn with_core_components(world: &World) -> Self {
        let mut world_data = WorldData::new();
        world_data.add_component::<EntityInfo>(world);
        world_data.add_component::<Transform>(world);
        world_data.add_component::<Camera>(world);
        world_data.add_component::<RigidBody2D>(world);
        world_data.add_component::<Collider2D>(world);
        world_data.add_component::<Renderer2D>(world);
        world_data
    }

    fn add_component<T: Edit + Send + Sync>(&mut self, world: &World) {
        world.run(|c: View<T>| {
            for (e, c) in c.iter().with_id() {
                let entity_data = self.entities.entry(e).or_insert_with(|| EntityData::new());
                entity_data.components.insert(T::name().into(), c.get_data());
            }
        })
    }
}

pub trait WorldExt {
    fn load_core_components(&mut self, world_data: &WorldData);
    // Currently we must write different generic functions for different tracking type, see https://github.com/leudz/shipyard/issues/157
    // TODO: find a way to write only one generic load_component function to cover all tracking type
    fn load_component_untracked<T: Edit<Tracking = Untracked> + Send + Sync>(&mut self, world_data: &WorldData);
    fn load_component_trackall<T: Edit<Tracking = All> + Send + Sync>(&mut self, world_data: &WorldData);
    fn load_component<T: Edit>(id: EntityId, t: &mut T, world_data: &WorldData);
    fn recreate_core_components(&mut self, data: &WorldData);
    fn create_component<T: Edit + Send + Sync>(&mut self, id: EntityId, name: &String, data: &ComponentData);
}

impl WorldExt for World {
    fn load_core_components(&mut self, world_data: &WorldData) {
        self.load_component_untracked::<EntityInfo>(world_data);
        self.load_component_untracked::<Transform>(world_data);
        self.load_component_untracked::<Camera>(world_data);
        self.load_component_trackall::<RigidBody2D>(world_data);
        self.load_component_trackall::<Collider2D>(world_data);
        self.load_component_untracked::<Renderer2D>(world_data);
    }

    fn load_component_untracked<T: Edit<Tracking = Untracked> + Send + Sync>(&mut self, world_data: &WorldData) {
        self.run(|mut t: ViewMut<T>| {
            for (id, t) in (&mut t).iter().with_id() {
                Self::load_component(id, t, world_data);
            }
        })
    }

    fn load_component_trackall<T: Edit<Tracking = All> + Send + Sync>(&mut self, world_data: &WorldData) {
        self.run(|mut t: ViewMut<T>| {
            for (id, mut t) in (&mut t).iter().with_id() {
                Self::load_component(id, t.as_mut(), world_data);
            }
        })
    }

    fn load_component<T: Edit>(id: EntityId, t: &mut T, world_data: &WorldData) {
        if let Some(entity_data) = world_data.entities.get(&id) {
            if let Some(component_data) = entity_data.components.get(T::name()) {
                t.set_data(component_data);
            }
        }
    }

    fn recreate_core_components(&mut self, data: &WorldData) {
        self.clear();
        let mut old_id_to_new_id = HashMap::new();
        for (old_id, entity_data) in &data.entities {
            let new_id = *old_id_to_new_id.entry(old_id).or_insert_with(|| self.add_entity(()));
            for (component_name, component_data) in &entity_data.components {
                self.create_component::<EntityInfo>(new_id, component_name, component_data);
                self.create_component::<Transform>(new_id, component_name, component_data);
                self.create_component::<Camera>(new_id, component_name, component_data);
                self.create_component::<RigidBody2D>(new_id, component_name, component_data);
                self.create_component::<Collider2D>(new_id, component_name, component_data);
                self.create_component::<Renderer2D>(new_id, component_name, component_data);
            }
        }
    }

    fn create_component<T: Edit + Send + Sync>(&mut self, id: EntityId, name: &String, data: &ComponentData) {
        if T::name() == name {
            self.add_component(id, (T::from(data),));
        }
    }
}

pub struct ComponentFn {
    pub create: fn(&mut World, EntityId),
    pub destroy: fn(&mut World, EntityId),
}

impl ComponentFn {
    pub fn with_core_components() -> IndexMap<&'static str, ComponentFn> {
        let mut component_fn = IndexMap::new();
        Self::add_component::<EntityInfo>(&mut component_fn);
        Self::add_component::<Transform>(&mut component_fn);
        Self::add_component::<Camera>(&mut component_fn);
        Self::add_component::<RigidBody2D>(&mut component_fn);
        Self::add_component::<Collider2D>(&mut component_fn);
        Self::add_component::<Renderer2D>(&mut component_fn);
        component_fn
    }

    pub fn add_component<T: Edit + Send + Sync>(component_fn: &mut IndexMap<&'static str, ComponentFn>) {
        component_fn.insert(T::name(), ComponentFn {
            create: |world, entity| world.add_component(entity, (T::default(),)),
            destroy: |world, entity| world.delete_component::<T>(entity),
        });
    }
}
