pub use steel_common::*;
pub mod physics2d;

use shipyard::{Component, IntoIter, IntoWithId, View, World};
use glam::{Vec3, Vec2};

#[derive(Component, Debug)]
pub struct Renderer2D; // can only render cuboid currently. TODO: render multiple shape

pub trait Edit: Component {
    fn name() -> &'static str;

    fn to_data(&self) -> ComponentData {
        ComponentData::new(Self::name())
    }

    fn from_data(&mut self, data: ComponentData) { }
}

pub fn add_component<T: Edit + Send + Sync>(world_data: &mut WorldData, world: &World) {
    world.run(|c: View<T>| {
        for (e, c) in c.iter().with_id() {
            let index = *world_data.id_index_map.entry(e).or_insert(world_data.entities.len());
            if index == world_data.entities.len() {
                world_data.entities.push(EntityData { id: e, components: Vec::new() });
            }
            world_data.entities[index].components.push(c.to_data());
        }
    })
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
        data.variants.push(Variant { name: "position", value: Value::Vec3(self.position) });
        data.variants.push(Variant { name: "rotation", value: Value::Float32(self.rotation) });
        data.variants.push(Variant { name: "scale", value: Value::Vec2(self.scale) });
        data
    }

    fn from_data(&mut self, data: ComponentData) {
        for v in data.variants {
            match v.name {
                "position" => self.position = if let Value::Vec3(position) = v.value { position } else { Default::default() },
                "rotation" => self.rotation = if let Value::Float32(rotation) = v.value { rotation } else { Default::default() },
                "scale" => self.scale = if let Value::Vec2(scale) = v.value { scale } else { Vec2::ONE },
                _ => (),
            }
        }
    }
}

