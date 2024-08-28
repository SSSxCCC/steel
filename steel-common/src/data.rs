use crate::{asset::AssetId, platform::Platform};
use glam::{IVec2, IVec3, IVec4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use shipyard::EntityId;
use std::{collections::HashMap, error::Error, ops::RangeInclusive, path::Path};

/// Limit Value in a range or in several enum, mainly used in Edit::get_data.
#[derive(Debug, Clone)]
pub enum Limit {
    /// Limit i32 value to a range.
    /// Int32Range can be used in IVec types to apply to all values.
    Int32Range(RangeInclusive<i32>),
    /// Limit i32 value to serval values and use String to display them.
    Int32Enum(Vec<(i32, String)>),
    /// Limit u32 value to a range.
    /// UInt32Range can be used in UVec types to apply to all values.
    UInt32Range(RangeInclusive<u32>),
    /// Limit f32 value to [0, 2π) and display in [0, 360).
    /// Float32Rotation can be used in Vec types to apply to all values.
    Float32Rotation,
    /// Limit f32 value to a range.
    /// Float32Range can be used in Vec types to apply to all values.
    Float32Range(RangeInclusive<f32>),
    /// Regard xyz in Vec3 as rgb, and use rgb color picker to edit.
    Vec3Color,
    /// Regard xyzw in Vec3 as rgba, and use rgba color picker to edit.
    Vec4Color,
    /// Display String in multiline text edit.
    StringMultiline,
    /// The value can not be changed.
    ReadOnly,
    /// Limit each element in a Vec2/Vec3/Vec4 to a different range.
    VecRange(Vec<Option<RangeInclusive<f32>>>),
    /// Limit each element in a IVec2/IVec3/IVec4 to a different range.
    IVecRange(Vec<Option<RangeInclusive<i32>>>),
    /// Limit each element in a UVec2/UVec3/UVec4 to a different range.
    UVecRange(Vec<Option<RangeInclusive<u32>>>),
}

/// Value is a data which stores in component or unique.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Value {
    Bool(bool),
    Int32(i32),
    UInt32(u32),
    Float32(f32),
    String(String),
    Entity(EntityId),
    Vec2(Vec2),
    Vec3(Vec3),
    Vec4(Vec4),
    IVec2(IVec2),
    IVec3(IVec3),
    IVec4(IVec4),
    UVec2(UVec2),
    UVec3(UVec3),
    UVec4(UVec4),
    Asset(AssetId),
    VecEntity(Vec<EntityId>),
}

/// Data contains all Value with Limit in a component or unique.
#[derive(Debug, Serialize, Deserialize, Clone)]
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
        Data {
            values: IndexMap::new(),
            limits: HashMap::new(),
        }
    }

    /// Insert a value to this data, you can chain many insert calls by using this funtion.
    pub fn insert(mut self, name: impl Into<String>, value: Value) -> Self {
        self.add_value(name, value);
        self
    }

    /// Insert a value and its limit to this data, you can chain many insert calls by using this funtion.
    pub fn insert_with_limit(
        mut self,
        name: impl Into<String>,
        value: Value,
        limit: Limit,
    ) -> Self {
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

    /// Cut useless data in Data before saving to file:
    /// 1. Erase generation value of EntityId.
    /// 2. Skip read only values if cut_read_only is true.
    pub fn cut(&self, cut_read_only: bool) -> Data {
        let mut data_cut = Data::new();
        for (name, value) in &self.values {
            if !(cut_read_only && matches!(self.limits.get(name), Some(Limit::ReadOnly))) {
                let value = match value {
                    Value::Entity(e) => Value::Entity(Self::erase_generation(e)),
                    Value::VecEntity(v) => {
                        Value::VecEntity(v.iter().map(|e| Self::erase_generation(e)).collect())
                    }
                    _ => value.clone(),
                };
                data_cut.values.insert(name.clone(), value);
            }
        }
        data_cut
    }

    /// Helper function to set generation value of EntityId to 0 if it is not EntityId::dead().
    pub fn erase_generation(eid: &EntityId) -> EntityId {
        if *eid == EntityId::dead() {
            *eid
        } else {
            EntityId::new_from_index_and_gen(eid.index(), 0)
        }
    }
}

/// EntityData contains all component Data in a entity.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntityData {
    pub components: IndexMap<String, Data>,
}

impl EntityData {
    /// Create an empty EntityData.
    pub fn new() -> Self {
        EntityData {
            components: IndexMap::new(),
        }
    }
}

/// A collection of EntityData. This is a wrapper of IndexMap<EntityId, EntityData>.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntitiesData(
    #[serde(with = "vectorize")] // TODO: #[serde_as(as = "Vec<(_, _)>")]
    pub  IndexMap<EntityId, EntityData>,
);

impl EntitiesData {
    /// Create a new EntitiesData.
    pub fn new() -> Self {
        EntitiesData(IndexMap::new())
    }

    /// Cut useless data in EntitiesData before saving to file:
    /// 1. Erase generation value of EntityId.
    /// 2. Skip read only values.
    pub fn cut(&self) -> EntitiesData {
        let mut entities_data_cut = EntitiesData::new();
        for (eid, entity_data) in self {
            let mut entity_data_cut = EntityData::new();
            for (comopnent_name, component_data) in &entity_data.components {
                let cut_read_only = comopnent_name != "Child" && comopnent_name != "Parent"; // TODO: use a more generic way to allow read-only values of some components to save to file
                let component_data_cut = component_data.cut(cut_read_only);
                entity_data_cut
                    .components
                    .insert(comopnent_name.clone(), component_data_cut);
            }
            entities_data_cut.insert(Data::erase_generation(eid), entity_data_cut);
        }
        entities_data_cut
    }
}

impl std::ops::Deref for EntitiesData {
    type Target = IndexMap<EntityId, EntityData>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for EntitiesData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for EntitiesData {
    type Item = <IndexMap<EntityId, EntityData> as IntoIterator>::Item;
    type IntoIter = <IndexMap<EntityId, EntityData> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a EntitiesData {
    type Item = <&'a IndexMap<EntityId, EntityData> as IntoIterator>::Item;
    type IntoIter = <&'a IndexMap<EntityId, EntityData> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl<'a> IntoIterator for &'a mut EntitiesData {
    type Item = <&'a mut IndexMap<EntityId, EntityData> as IntoIterator>::Item;
    type IntoIter = <&'a mut IndexMap<EntityId, EntityData> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&mut self.0).into_iter()
    }
}

/// A collection of unique Data. This is a wrapper of IndexMap<String, Data>.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UniquesData(pub IndexMap<String, Data>);

impl UniquesData {
    /// Create a new UniquesData.
    pub fn new() -> Self {
        UniquesData(IndexMap::new())
    }

    /// Cut useless data in UniquesData before saving to file:
    /// 1. Erase generation value of EntityId.
    /// 2. Skip read only values.
    pub fn cut(&self) -> UniquesData {
        let mut uniques_data_cut = UniquesData::new();
        for (unique_name, unique_data) in self {
            let cut_read_only = unique_name != "Hierarchy"; // TODO: use a more generic way to allow read-only values of some uniques to save to file
            let unique_data_cut = unique_data.cut(cut_read_only);
            uniques_data_cut.insert(unique_name.clone(), unique_data_cut);
        }
        uniques_data_cut
    }
}

impl std::ops::Deref for UniquesData {
    type Target = IndexMap<String, Data>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for UniquesData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for UniquesData {
    type Item = <IndexMap<String, Data> as IntoIterator>::Item;
    type IntoIter = <IndexMap<String, Data> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a UniquesData {
    type Item = <&'a IndexMap<String, Data> as IntoIterator>::Item;
    type IntoIter = <&'a IndexMap<String, Data> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl<'a> IntoIterator for &'a mut UniquesData {
    type Item = <&'a mut IndexMap<String, Data> as IntoIterator>::Item;
    type IntoIter = <&'a mut IndexMap<String, Data> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&mut self.0).into_iter()
    }
}

/// WorldData contains all EntityData and UniqueData in the world.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorldData {
    pub entities: EntitiesData,
    pub uniques: UniquesData,
}

impl WorldData {
    /// Create an empty WorldData.
    pub fn new() -> Self {
        WorldData {
            entities: EntitiesData::new(),
            uniques: UniquesData::new(),
        }
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
                log::warn!(
                    "Failed to load world_data, file={}, error={error}",
                    file.as_ref().display()
                );
                None
            }
        }
    }

    fn _load_from_file(
        file: impl AsRef<Path>,
        platform: &Platform,
    ) -> Result<WorldData, Box<dyn Error>> {
        let s = platform.read_asset_to_string(file)?;
        Ok(serde_json::from_str::<WorldData>(&s)?)
    }

    /// Helper funtion to load WorldData from bytes.
    pub fn load_from_bytes(bytes: &[u8]) -> Option<WorldData> {
        match Self::_load_from_bytes(bytes) {
            Ok(world_data) => Some(world_data),
            Err(error) => {
                log::warn!(
                    "Failed to load world_data, bytes={:?}, error={error}",
                    bytes
                );
                None
            }
        }
    }

    fn _load_from_bytes(bytes: &[u8]) -> Result<WorldData, Box<dyn Error>> {
        Ok(serde_json::from_slice::<WorldData>(bytes)?)
    }

    /// Cut useless data in WorldData before saving to file:
    /// 1. Erase generation value of EntityId.
    /// 2. Skip read only values.
    pub fn cut(&self) -> WorldData {
        WorldData {
            entities: self.entities.cut(),
            uniques: self.uniques.cut(),
        }
    }
}

/// SceneData is a compressed version of [WorldData].
/// SceneData stores prefabs as their asset ids while WorldData stores all data for every entity.
pub struct SceneData {
    /// SceneData regard all eneities in world as one prefab.
    pub entities: Prefab,
    /// All uniques data.
    pub uniques: UniquesData,
    /// All id paths in uniques. unique_name -> data_name -> entity_id_path.
    pub unique_id_paths: IndexMap<String, DataEntityIdPaths>,
}

/// PrefabData is either a Prefab or a PrefabVariant.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PrefabData {
    Prefab(Prefab),
    Variant(PrefabVariant),
}

/// Prefab is a collection of entities, which stored in an asset, to be used to
/// create entities as template. A prefab can have many nested prefabs.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prefab {
    /// All entities in this prefab.
    pub entities: EntitiesData,
    /// All id paths in entities. entity_id -> component_name -> data_name -> entity_id_path.
    pub id_paths: IndexMap<EntityId, IndexMap<String, DataEntityIdPaths>>,
    /// All nested prefabs and their local modification.
    pub nested_prefabs: Vec<NestedPrefab>,
}

/// PrefabVariant is the prefab that have some modifications from the original prefab.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrefabVariant {
    /// The asset id of the original prefab.
    pub prefab: AssetId,
    /// All modifications from the original prefab.
    pub variant: Vec<EntityDataVariant>,
    /// All nested prefabs and their local modification.
    pub nested_prefabs: Vec<NestedPrefab>,
}

/// NestedPrefab is the prefab in another prefab, and have some modifications from the original prefab.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NestedPrefab {
    /// The asset id of the original prefab.
    pub prefab: AssetId,
    /// All modifications from the original prefab.
    pub variant: Vec<EntityDataVariant>,
}

/// EntityDataVariant is an EntityData with nested ids.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntityDataVariant {
    /// The id of this entity.
    pub id: EntityId,
    /// The entity id path.
    pub id_path: EntityIdPath,
    /// The entity data.
    pub data: EntityData,
    /// All entity id paths in data.
    pub data_id_paths: IndexMap<String, DataEntityIdPaths>,
}

/// EntityIdPath is the path to an entity in a nested prefab. Examples:
/// 1. \[\] or None means EntityId is in the current Prefab.
/// 2. \[a\] means EntityId is in the nested prefab at index a.
/// 3. \[a, b\] means the EntityId is in the nested prefab at index b of the nested prefab at index a.
pub type EntityIdPath = Vec<usize>;

/// All [EntityIdPath] in [Data].
pub type DataEntityIdPaths = IndexMap<String, EntityIdPathInValue>;

/// The [EntityIdPath] in [Value].
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EntityIdPathInValue {
    EntityId(EntityIdPath),
    EntityVec(IndexMap<usize, EntityIdPath>),
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
