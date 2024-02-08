use std::{collections::HashMap, sync::Arc};
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

/// Define min and max value in a range
#[derive(Debug)]
pub struct Range<T> {
    pub min: T,
    pub max: T,
    pub min_include: bool,
    pub max_include: bool,
}

/// Limit Value in a range or in several enum
#[derive(Debug)]
pub enum Limit {
    Int32Range(Range<i32>),
    /// limit i32 value to serval values and use String to display them
    Int32Enum(Vec<(i32, String)>),
    /// limit f32 value to [0, 2π) and display in [0, 360)
    Float32Rotation,
    Float32Range(Range<f32>),
    Vec2Range {
        x: Range<f32>,
        y: Range<f32>,
    },
    Vec3Range {
        x: Range<f32>,
        y: Range<f32>,
        z: Range<f32>,
    },
    Vec4Range {
        x: Range<f32>,
        y: Range<f32>,
        z: Range<f32>,
        w: Range<f32>,
    },
    /// display String in multiline text edit
    StringMultiline,
    // TODO: ReadOnly
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
    #[serde(skip)]
    pub limits: HashMap<String, Limit>,
}

impl ComponentData {
    pub fn new() -> Self {
        ComponentData { values: IndexMap::new(), limits: HashMap::new() }
    }

    pub fn add(&mut self, name: impl Into<String>, value: Value, limit: Limit) {
        let name = name.into();
        self.values.insert(name.clone(), value);
        self.limits.insert(name, limit);
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
