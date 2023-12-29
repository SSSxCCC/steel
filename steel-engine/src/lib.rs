pub mod engine;
pub mod physics2d;
pub mod render2d;

pub use steel_common::*;

use render2d::Renderer2D;
use physics2d::{RigidBody2D, Collider2D};
use shipyard::{Component, IntoIter, IntoWithId, View, World};
use glam::{Vec3, Vec2};
use log::{Log, LevelFilter, SetLoggerError};

#[no_mangle]
pub fn setup_logger(logger: &'static dyn Log, level: LevelFilter) -> Result<(), SetLoggerError> {
    log::set_max_level(level);
    log::set_logger(logger)
}

pub trait Edit: Component {
    fn name() -> &'static str;

    fn to_data(&self) -> ComponentData {
        ComponentData::new(Self::name())
    }

    fn from_data(&mut self, data: ComponentData) { }
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
                let index = *self.id_index_map.entry(e).or_insert(self.entities.len());
                if index == self.entities.len() {
                    self.entities.push(EntityData { id: e, components: Vec::new() });
                }
                self.entities[index].components.push(c.to_data());
            }
        })
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

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::new(Self::name());
        data.variants.push(Variant::new("position", Value::Vec3(self.position)));
        data.variants.push(Variant::new("rotation", Value::Float32(self.rotation)));
        data.variants.push(Variant::new("scale", Value::Vec2(self.scale)));
        data
    }

    fn from_data(&mut self, data: ComponentData) {
        for v in data.variants {
            match v.name.as_str() {
                "position" => self.position = if let Value::Vec3(position) = v.value { position } else { Default::default() },
                "rotation" => self.rotation = if let Value::Float32(rotation) = v.value { rotation } else { Default::default() },
                "scale" => self.scale = if let Value::Vec2(scale) = v.value { scale } else { Vec2::ONE },
                _ => (),
            }
        }
    }
}
