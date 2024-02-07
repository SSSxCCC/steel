use std::sync::Arc;
use glam::{Vec2, Vec3, Vec4};
use indexmap::IndexMap;
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
    fn reload(&mut self, world_data: &WorldData);
}

/// Value is a data store in component
#[derive(Debug, Serialize, Deserialize)]
pub enum Value {
    Int32(i32),
    Float32(f32),
    String(String),
    Vec2(Vec2),
    Vec3(Vec3),
    Vec4(Vec4),
}

/// ComponentData contains all Value in a component
#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentData {
    // &'static str is too dangerous to be used in here because
    // its memory is no longer exist when steel.dll is unloaded!
    pub values: IndexMap<String, Value>,
}

impl ComponentData {
    pub fn new() -> Self {
        ComponentData { values: IndexMap::new() }
    }
}

/// EntityData contains all ComponentData in a entity
#[derive(Debug, Serialize, Deserialize)]
pub struct EntityData {
    pub components: IndexMap<String, ComponentData>,
}

impl EntityData {
    pub fn new() -> Self {
        EntityData { components: IndexMap::new() }
    }
}

/// WorldData contains all EntityData in the world
#[derive(Debug, Serialize, Deserialize)]
pub struct WorldData {
    #[serde(with = "vectorize")] // TODO: #[serde_as(as = "Vec<(_, _)>")]
    pub entities: IndexMap<EntityId, EntityData>,
}

impl WorldData {
    pub fn new() -> Self {
        WorldData { entities: IndexMap::new() }
    }
}

pub struct DrawInfo<'a> {
    pub before_future: Box<dyn GpuFuture>,
    pub context: &'a VulkanoContext,
    pub renderer: &'a VulkanoWindowRenderer,
    pub image: Arc<dyn ImageViewAbstract>, // the image we will draw
    pub window_size: Vec2,
}

pub mod vectorize {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::iter::FromIterator;

    pub fn serialize<'a, T, K, V, S>(target: T, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: IntoIterator<Item = (&'a K, &'a V)>,
        K: Serialize + 'a,
        V: Serialize + 'a,
    {
        let container: Vec<_> = target.into_iter().collect();
        serde::Serialize::serialize(&container, ser)
    }

    pub fn deserialize<'de, T, K, V, D>(des: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: FromIterator<(K, V)>,
        K: Deserialize<'de>,
        V: Deserialize<'de>,
    {
        let container: Vec<_> = serde::Deserialize::deserialize(des)?;
        Ok(T::from_iter(container.into_iter()))
    }
}
