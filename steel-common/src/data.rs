use std::{collections::HashMap, error::Error, ops::RangeInclusive, path::Path};
use glam::{Vec2, Vec3, Vec4};
use indexmap::IndexMap;
use serde::{Serialize, Deserialize};
use shipyard::EntityId;
use crate::platform::Platform;

/// Limit Value in a range or in several enum, mainly used in Edit::get_data.
#[derive(Debug)]
pub enum Limit {
    /// Limit i32 value to a range.
    Int32Range(RangeInclusive<i32>),
    /// Limit i32 value to serval values and use String to display them.
    Int32Enum(Vec<(i32, String)>),
    /// Limit f32 value to [0, 2π) and display in [0, 360).
    /// Float32Rotation can be used in Vec types to apply to all values.
    Float32Rotation,
    /// Limit f32 value to a range.
    Float32Range(RangeInclusive<f32>),
    /// Limit each element in a Vec2 to a different range.
    Vec2Range {
        x: Option<RangeInclusive<f32>>,
        y: Option<RangeInclusive<f32>>,
    },
    /// Limit each element in a Vec3 to a different range.
    Vec3Range {
        x: Option<RangeInclusive<f32>>,
        y: Option<RangeInclusive<f32>>,
        z: Option<RangeInclusive<f32>>,
    },
    /// Regard xyz in Vec3 as rgb, and use rgb color picker to edit.
    Vec3Color,
    /// Limit each element in a Vec4 to a different range.
    Vec4Range {
        x: Option<RangeInclusive<f32>>,
        y: Option<RangeInclusive<f32>>,
        z: Option<RangeInclusive<f32>>,
        w: Option<RangeInclusive<f32>>,
    },
    /// Regard xyzw in Vec3 as rgba, and use rgba color picker to edit.
    Vec4Color,
    /// Display String in multiline text edit.
    StringMultiline,
    /// The value can not be changed.
    ReadOnly,
}

/// Value is a data which stores in component or unique.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Value {
    Bool(bool),
    Int32(i32),
    Float32(f32),
    String(String),
    Vec2(Vec2),
    Vec3(Vec3),
    Vec4(Vec4),
}

/// Data contains all Value with Limit in a component or unique.
#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    // &'static str is too dangerous to be used in here because its memory is no longer exist when steel.dll is unloaded!
    pub values: IndexMap<String, Value>,
    #[serde(skip)]
    pub limits: HashMap<String, Limit>,
}

impl Data {
    /// create a new data, then you can continue to call insert or insert_with_limit to fill this data.
    /// # example
    /// ```rust
    /// let data = Data::new().insert("name", Value::String("Sam".into()))
    ///     .insert("age", Value::Float32(18.0));
    /// ```
    pub fn new() -> Self {
        Data { values: IndexMap::new(), limits: HashMap::new() }
    }

    /// Insert a value to this data, you can chain many insert calls by using this funtion.
    pub fn insert(mut self, name: impl Into<String>, value: Value) -> Self {
        self.add_value(name, value);
        self
    }

    /// Insert a value and its limit to this data, you can chain many insert calls by using this funtion.
    pub fn insert_with_limit(mut self, name: impl Into<String>, value: Value, limit: Limit) -> Self {
        self.add_value_with_limit(name, value, limit);
        self
    }

    /// Add a value to this data.
    pub fn add_value(&mut self, name: impl Into<String>, value: Value) {
        self.values.insert(name.into(), value);
    }

    /// Add a value and its limit to this data.
    pub fn add_value_with_limit(&mut self, name: impl Into<String>, value: Value, limit: Limit) {
        let name = name.into();
        self.values.insert(name.clone(), value);
        self.limits.insert(name, limit);
    }

    /// Get a value from this data.
    pub fn get(&self, name: impl AsRef<str>) -> Option<&Value> {
        self.values.get(name.as_ref())
    }
}

/// EntityData contains all component Data in a entity.
#[derive(Debug, Serialize, Deserialize)]
pub struct EntityData {
    pub components: IndexMap<String, Data>,
}

impl EntityData {
    /// Create an empty EntityData.
    pub fn new() -> Self {
        EntityData { components: IndexMap::new() }
    }
}

/// WorldData contains all EntityData and UniqueData in the world.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorldData {
    #[serde(with = "vectorize")] // TODO: #[serde_as(as = "Vec<(_, _)>")]
    pub entities: IndexMap<EntityId, EntityData>,
    pub uniques: IndexMap<String, Data>,
}

impl WorldData {
    /// Create an empty WorldData.
    pub fn new() -> Self {
        WorldData { entities: IndexMap::new(), uniques: IndexMap::new() }
    }

    /// The WorldData becomes empty after clear.
    pub fn clear(&mut self) {
        self.entities.clear();
        self.uniques.clear();
    }

    /// Helper funtion to load WorldData from file, the file path must be relative to the asset folder.
    pub fn load_from_file(file: impl AsRef<Path>, platform: &Platform) -> Option<WorldData> {
        match Self::_load_from_file(file.as_ref(), &platform) {
            Ok(world_data) => Some(world_data),
            Err(error) => {
                log::warn!("Failed to load world_data, file={}, error={error}", file.as_ref().display());
                None
            }
        }
    }

    fn _load_from_file(file: impl AsRef<Path>, platform: &Platform) -> Result<WorldData, Box<dyn Error>> {
        let s = platform.read_asset_to_string(file)?;
        Ok(serde_json::from_str::<WorldData>(&s)?)
    }
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
