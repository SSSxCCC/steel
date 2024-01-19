use std::{collections::HashMap, sync::Arc};
use glam::{Vec2, Vec3, Vec4};
use serde::{Serialize, Deserialize};
use shipyard::EntityId;
use vulkano::{sync::GpuFuture, image::ImageViewAbstract};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};

pub trait Engine {
    fn init(&mut self, world_data: Option<&WorldData>);
    fn maintain(&mut self);
    fn update(&mut self);
    fn draw(&mut self, info: DrawInfo) -> Box<dyn GpuFuture>;
    fn draw_editor(&mut self, info: DrawInfo) -> Box<dyn GpuFuture>;
    fn save(&self) -> WorldData;
    fn load(&mut self, world_data: &WorldData);
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Value {
    Int32(i32),
    Float32(f32),
    String(String),
    Vec2(Vec2),
    Vec3(Vec3),
    Vec4(Vec4),
}

/// name -> value hash map
pub type ValueMap<'a> = HashMap<&'a str, &'a Value>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Variant {
    // &'static str is too dangerous to be used here because
    // its memory is no longer exist when steel.dll is unloaded!
    pub name: String,
    pub value: Value,
}

impl Variant {
    pub fn new(name: impl Into<String>, value: Value) -> Self {
        Variant { name: name.into(), value }
    }
}

// ComponentData contains all variant in a component
#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentData {
    pub name: String,
    pub variants: Vec<Variant>,
}

impl ComponentData {
    pub fn new(name: impl Into<String>) -> Self {
        ComponentData { name: name.into(), variants: Vec::new() }
    }

    pub fn value_map(&self) -> ValueMap {
        HashMap::from_iter(self.variants.iter().map(|v| (v.name.as_str(), &v.value)))
    }
}

// EntityData contains all component data in a entity, key is component name
#[derive(Debug, Serialize, Deserialize)]
pub struct EntityData {
    pub id: EntityId,
    pub components: Vec<ComponentData>,
}

// WorldData contains all entity data in the world
#[derive(Debug, Serialize, Deserialize)]
pub struct WorldData {
    pub entities: Vec<EntityData>,
    #[serde(skip)]
    entity_index_map: Option<HashMap<EntityId, usize>>,
}

impl WorldData {
    pub fn new() -> Self {
        WorldData { entities: Vec::new(), entity_index_map: None }
    }

    pub fn entity_index_map(&mut self) -> &mut HashMap<EntityId, usize> {
        self.entity_index_map.get_or_insert_with(|| Self::build_entity_index_map(&self.entities))
    }

    fn build_entity_index_map(entities: &Vec<EntityData>) -> HashMap<EntityId, usize> {
        let mut entity_index_map = HashMap::new();
        for (index, entity_data) in entities.iter().enumerate() {
            entity_index_map.insert(entity_data.id, index);
        }
        entity_index_map
    }
}

pub struct DrawInfo<'a> {
    pub before_future: Box<dyn GpuFuture>,
    pub context: &'a VulkanoContext,
    pub renderer: &'a VulkanoWindowRenderer,
    pub image: Arc<dyn ImageViewAbstract>, // the image we will draw
    pub window_size: Vec2,
}
