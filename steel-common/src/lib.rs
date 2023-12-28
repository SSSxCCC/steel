use std::{collections::HashMap, sync::Arc};
use glam::{Vec2, Vec3, Vec4};
use shipyard::EntityId;
use vulkano::{sync::GpuFuture, image::ImageViewAbstract};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};

pub trait Engine {
    fn init(&mut self);
    fn update(&mut self);
    fn draw(&mut self, info: DrawInfo) -> Box<dyn GpuFuture>;
    fn save(&self) -> WorldData;
    fn load(&mut self, world_data: WorldData);
}

#[derive(Debug)]
pub enum Value {
    Int32(i32),
    Float32(f32),
    String(String),
    Vec2(Vec2),
    Vec3(Vec3),
    Vec4(Vec4),
}

#[derive(Debug)]
pub struct Variant {
    pub name: &'static str,
    pub value: Value,
}

// ComponentData contains all variant in a component
#[derive(Debug)]
pub struct ComponentData {
    pub name: &'static str,
    pub variants: Vec<Variant>,
}

impl ComponentData {
    pub fn new(name: &'static str) -> Self {
        ComponentData { name, variants: Vec::new() }
    }
}

// EntityData contains all component data in a entity, key is component name
#[derive(Debug)]
pub struct EntityData {
    pub id: EntityId,
    pub components: Vec<ComponentData>,
}

// WorldData contains all entity data in the world
#[derive(Debug)]
pub struct WorldData {
    pub entities: Vec<EntityData>,
    pub id_index_map: HashMap<EntityId, usize>,
}

impl WorldData {
    pub fn new() -> Self {
        WorldData{entities: Vec::new(), id_index_map: HashMap::new()}
    }
}

pub struct DrawInfo<'a> {
    pub before_future: Box<dyn GpuFuture>,
    pub context: &'a VulkanoContext,
    pub renderer: &'a VulkanoWindowRenderer,
    pub image: Arc<dyn ImageViewAbstract>, // the image we will draw
    pub window_size: Vec2,
}
