use crate::{
    asset::AssetId,
    prefab::{DataEntityIdPaths, EntityIdPath, EntityIdPathInValue, EntityIdWithPath, PrefabData},
};
use glam::{IVec2, IVec3, IVec4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use shipyard::EntityId;
use std::{borrow::Cow, collections::HashMap, ops::RangeInclusive, sync::Arc};

/// Limit [Value] in a range or in several enum, mainly used in Edit::get_data.
#[derive(Debug, Clone, PartialEq)]
pub enum Limit {
    /// Limit i32 value to a range.
    /// Int32Range can be used in IVec types and VecInt32 to apply to all values.
    Int32Range(RangeInclusive<i32>),
    /// Limit i32 value to serval values and use String to display them.
    Int32Enum(Vec<(i32, String)>),
    /// Limit i64 value to a range.
    /// Int64Range can be used in VecInt64 to apply to all values.
    Int64Range(RangeInclusive<i64>),
    /// Limit u32 value to a range.
    /// UInt32Range can be used in UVec types and VecUInt32 to apply to all values.
    UInt32Range(RangeInclusive<u32>),
    /// Limit u64 value to a range.
    /// UInt64Range can be used in VecUInt64 to apply to all values.
    UInt64Range(RangeInclusive<u64>),
    /// Limit f32 value to [0, 2π) and display in [0, 360).
    /// Float32Rotation can be used in Vec types to apply to all values.
    Float32Rotation,
    /// Limit f32 value to a range.
    /// Float32Range can be used in Vec types and VecFloat32 to apply to all values.
    Float32Range(RangeInclusive<f32>),
    /// Limit f64 value to a range.
    /// Float64Range can be used in VecFloat64 to apply to all values.
    Float64Range(RangeInclusive<f64>),
    /// Regard xyz in Vec3 as rgb, and use rgb color picker to edit.
    Vec3Color,
    /// Regard xyzw in Vec3 as rgba, and use rgba color picker to edit.
    Vec4Color,
    /// Display String in multiline text edit.
    StringMultiline,
    /// The value can not be changed.
    ReadOnly,
}

/// Value is a data which stores in component or unique.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Int32(i32),
    Int64(i64),
    UInt32(u32),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
    String(String),
    Vec2(Vec2),
    Vec3(Vec3),
    Vec4(Vec4),
    IVec2(IVec2),
    IVec3(IVec3),
    IVec4(IVec4),
    UVec2(UVec2),
    UVec3(UVec3),
    UVec4(UVec4),
    /// This points to an entity in the scene. Note: this value may be mapped when saving prefab,
    /// so if you need to store an offline or static value, use other value type instead.
    Entity(EntityId),
    Asset(AssetId),
    VecBool(Vec<bool>),
    VecInt32(Vec<i32>),
    VecInt64(Vec<i64>),
    VecUInt32(Vec<u32>),
    VecUInt64(Vec<u64>),
    VecFloat32(Vec<f32>),
    VecFloat64(Vec<f64>),
    VecString(Vec<String>),
    VecEntity(Vec<EntityId>),
    VecAsset(Vec<AssetId>),
    /// Value::Data is used to store a struct which implements Edit or can be converted to Data.
    Data(Data),
    /// Value::VecData is used to store a vector of struct which implements Edit or can be converted to Data.
    /// We could use this to replace other Value::Vec types but this has a performance cost.
    /// So be priority to use other Value::Vec types if exists.
    VecData(Vec<Data>),
}

impl Value {
    /// Map each entity in this value to another entity. If this value does not contain entity, just clone this value.
    pub fn map_entity(&self, f: impl Fn(EntityId) -> EntityId) -> Self {
        match self {
            Value::Entity(e) => Value::Entity(f(*e)),
            Value::VecEntity(v) => Value::VecEntity(v.iter().map(|e| f(*e)).collect()),
            _ => self.clone(),
        }
    }

    /// Map each entity in this value to another entity if path is provided. Otherwise, the entity will be mapped to [EntityId::dead].
    /// If this value does not contain entity, just clone this value.
    pub(crate) fn map_entity_with_path(
        &self,
        id_path: Option<&EntityIdPathInValue>,
        f: impl Fn(EntityIdWithPath) -> EntityId,
    ) -> Self {
        match self {
            Value::Entity(e) => {
                if let Some(EntityIdPathInValue::EntityId(p)) = id_path {
                    Value::Entity(f(EntityIdWithPath(*e, p.clone())))
                } else {
                    Value::Entity(EntityId::dead())
                }
            }
            Value::VecEntity(es) => {
                if let Some(EntityIdPathInValue::EntityVec(ps)) = id_path {
                    Value::VecEntity(
                        es.iter()
                            .enumerate()
                            .map(|(i, e)| {
                                ps.get(&(i as u64))
                                    .map(|p| f(EntityIdWithPath(*e, p.clone())))
                                    .unwrap_or_default()
                            })
                            .collect(),
                    )
                } else {
                    Value::VecEntity(vec![EntityId::dead(); es.len()])
                }
            }
            _ => self.clone(),
        }
    }

    /// Map each entity in this value to another entity and path, and insert path to [DataEntityIdPaths].
    /// If this value does not contain entity, just clone this value.
    pub(crate) fn map_entity_and_insert_id_paths(
        &self,
        data_name: &String,
        component_or_unique_name: &String,
        id_paths: &mut DataEntityIdPaths,
        f: impl Fn(EntityId) -> (EntityId, Option<EntityIdPath>),
    ) -> Self {
        match self {
            Value::Entity(e) => {
                let (entity_id, id_path) = f(*e);
                if let Some(id_path) = id_path {
                    id_paths
                        .entry(component_or_unique_name.clone())
                        .or_default()
                        .insert(data_name.clone(), EntityIdPathInValue::EntityId(id_path));
                }
                Value::Entity(entity_id)
            }
            Value::VecEntity(es) => Value::VecEntity(
                es.iter()
                    .enumerate()
                    .map(|(i, e)| {
                        let (entity_id, id_path) = f(*e);
                        if let Some(id_path) = id_path {
                            let id_paths = id_paths
                                .entry(component_or_unique_name.clone())
                                .or_default();
                            if let Some(EntityIdPathInValue::EntityVec(ev)) =
                                id_paths.get_mut(data_name)
                            {
                                ev.insert(i as u64, id_path);
                            } else {
                                let mut ev = IndexMap::new();
                                ev.insert(i as u64, id_path);
                                id_paths
                                    .insert(data_name.clone(), EntityIdPathInValue::EntityVec(ev));
                            }
                        }
                        entity_id
                    })
                    .collect(),
            ),
            _ => self.clone(),
        }
    }

    /// Iterate each entity in this value. If this value does not contain entity, do nothing.
    pub(crate) fn iter_entity_mut(&mut self, f: impl Fn(&mut EntityId)) {
        match self {
            Value::Entity(e) => f(e),
            Value::VecEntity(v) => v.iter_mut().for_each(f),
            _ => (),
        }
    }

    /// Iterate each entity in this value. If this value does not contain entity, do nothing.
    pub(crate) fn iter_entity_mut_with_path(
        &mut self,
        id_path: Option<&EntityIdPathInValue>,
        f: impl Fn(&mut EntityId, Option<&EntityIdPath>),
    ) {
        match self {
            Value::Entity(e) => {
                let id_path = if let Some(EntityIdPathInValue::EntityId(id_path)) = id_path {
                    Some(id_path)
                } else {
                    None
                };
                f(e, id_path);
            }
            Value::VecEntity(es) => {
                let id_paths = if let Some(EntityIdPathInValue::EntityVec(id_paths)) = id_path {
                    Cow::Borrowed(id_paths)
                } else {
                    Cow::Owned(Default::default())
                };
                for (i, e) in es.iter_mut().enumerate() {
                    f(e, id_paths.get(&(i as u64)));
                }
            }
            _ => (),
        }
    }
}

/// Data contains all [Value] with [Limit] in a component or unique.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct Data {
    // &'static str is too dangerous to be used in here because its memory is no longer exist when steel.dll is unloaded!
    pub values: IndexMap<String, Value>,
    #[serde(skip)]
    pub limits: HashMap<String, Limit>,
}

impl Data {
    /// create a new data, then you can continue to call [Data::insert] or [Data::insert_with_limit] to fill this data.
    /// # example
    /// ```rust
    /// let data = Data::new().insert("name", Value::String("Sam".into()))
    ///     .insert("age", Value::Float32(18.0));
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a value to this data, you can chain many insert calls by using this funtion.
    pub fn insert(&mut self, name: impl Into<String>, value: Value) -> &mut Self {
        self.values.insert(name.into(), value);
        self
    }

    /// Insert a value and its limit to this data, you can chain many insert calls by using this funtion.
    pub fn insert_with_limit(
        &mut self,
        name: impl Into<String>,
        value: Value,
        limit: Limit,
    ) -> &mut Self {
        let name = name.into();
        self.values.insert(name.clone(), value);
        self.limits.insert(name, limit);
        self
    }

    /// Get a value from this data.
    pub fn get(&self, name: impl AsRef<str>) -> Option<&Value> {
        self.values.get(name.as_ref())
    }

    /// Cut useless data in self before saving to file:
    /// 1. Erase generation value of [EntityId].
    /// 2. Skip read only values if cut_read_only is true.
    pub fn cut(&self, cut_read_only: bool) -> Self {
        let mut data_cut = Data::new();
        for (name, value) in &self.values {
            if !(cut_read_only && matches!(self.limits.get(name), Some(Limit::ReadOnly))) {
                let value = value.map_entity(Self::erase_generation);
                data_cut.values.insert(name.clone(), value);
            }
        }
        data_cut
    }

    /// Helper function to set generation value of EntityId to 0 if it is not EntityId::dead().
    pub fn erase_generation(e: EntityId) -> EntityId {
        if e == EntityId::dead() {
            e
        } else {
            EntityId::new_from_index_and_gen(e.index(), 0)
        }
    }
}

/// EntityData contains all component data in a entity.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EntityData {
    pub components: IndexMap<String, Data>,
}

impl EntityData {
    /// Get the name in Name component.
    /// Return None if it does not exist.
    pub fn name(&self) -> Option<&String> {
        self.components
            .get("Name")
            .and_then(|data| data.get("unnamed-0"))
            .and_then(|value| match value {
                Value::String(v) => Some(v),
                _ => None,
            })
    }

    /// Get the parent entity id of this entity.
    /// Return [EntityId::dead] if this entity is at top layer.
    pub fn parent(&self) -> EntityId {
        self.components
            .get("Parent")
            .map(|parent| match parent.get("unnamed-0") {
                Some(Value::Entity(v)) => *v,
                _ => panic!("Parent entity value not found in Parent component: {parent:?}"),
            })
            .unwrap_or_default()
    }

    /// Get the children entity ids of this entity.
    /// Return None if there are no children.
    pub fn children(&self) -> Option<&Vec<EntityId>> {
        self.components
            .get("Children")
            .map(|children| match children.get("unnamed-0") {
                Some(Value::VecEntity(v)) => v,
                _ => panic!("Children entity vector not found in Children component: {children:?}"),
            })
    }

    /// Get the mutable children entity ids of this entity.
    /// Return None if the children component does not exist.
    pub fn children_mut(&mut self) -> Option<&mut Vec<EntityId>> {
        match self.components.get_mut("Children") {
            Some(data) => match data.values.get_mut("unnamed-0") {
                Some(Value::VecEntity(v)) => Some(v),
                _ => panic!("Children entity vector not found in Children component"),
            },
            None => None,
        }
    }

    /// Create the Children component if it does not already exist.
    pub fn ensure_children_component(&mut self) {
        if !self.components.contains_key("Children") {
            let mut children_component = Data::default();
            children_component
                .values
                .insert("unnamed-0".to_string(), Value::VecEntity(Vec::new()));
            self.components
                .insert("Children".to_string(), children_component);
        }
    }

    /// Trim the Children component by removing all EntityId::dead entries.
    /// Remove the Children component if it becomes empty.
    pub fn trim_children(&mut self) {
        if let Some(children) = self.children_mut() {
            children.retain(|&id| id != EntityId::dead());
            if children.is_empty() {
                self.components.shift_remove("Children");
            }
        }
    }

    /// Get prefab_asset, prefab_entity_index, prefab_entity_path, and prefab_root_entity at the same time.
    /// Return None if this entity is not created from a prefab.
    pub fn prefab_info(&self) -> Option<(AssetId, u64, &EntityIdPath, EntityId)> {
        if let (
            Some(prefab_asset),
            Some(prefab_entity_index),
            Some(prefab_entity_path),
            Some(prefab_root_entity),
        ) = (
            self.prefab_asset(),
            self.prefab_entity_index(),
            self.prefab_entity_path(),
            self.prefab_root_entity(),
        ) {
            Some((
                prefab_asset,
                prefab_entity_index,
                prefab_entity_path,
                prefab_root_entity,
            ))
        } else {
            None
        }
    }

    /// Get the prefab asset that this entity belongs to.
    /// Return None if this entity is not in a prefab.
    pub fn prefab_asset(&self) -> Option<AssetId> {
        self.components
            .get("Prefab")
            .and_then(|data| data.get("asset"))
            .and_then(|value| match value {
                Value::Asset(v) => Some(*v),
                _ => None,
            })
    }

    /// Get the entity id index of this entity in the prefab.
    /// Return None if this entity is not in a prefab.
    pub fn prefab_entity_index(&self) -> Option<u64> {
        self.components
            .get("Prefab")
            .and_then(|data| data.get("entity_index"))
            .and_then(|value| match value {
                Value::UInt64(v) => Some(*v),
                _ => None,
            })
    }

    /// Get the entity id path of this entity in the prefab.
    /// Return None if this entity is not in a prefab.
    pub fn prefab_entity_path(&self) -> Option<&EntityIdPath> {
        self.components
            .get("Prefab")
            .and_then(|data| data.get("entity_path"))
            .and_then(|value| match value {
                Value::VecUInt64(v) => Some(v),
                _ => None,
            })
    }

    /// Get the prefab root entity id in the scene.
    /// Return None if this entity is not in a prefab.
    pub fn prefab_root_entity(&self) -> Option<EntityId> {
        self.components
            .get("Prefab")
            .and_then(|data| data.get("root_entity"))
            .and_then(|value| match value {
                Value::Entity(v) => Some(*v),
                _ => None,
            })
    }

    /// Update prefab info in this entity. This a helper function for reverting prefab.
    /// This dosen't update the entity_index, because it is not changed when reverting prefab.
    /// This will panic if the prefab component is not present on this entity.
    pub(crate) fn update_prefab_info(
        &mut self,
        asset: AssetId,
        entity_path: EntityIdPath,
        root_entity: EntityId,
    ) {
        let prefab_component = self.components.get_mut("Prefab").unwrap();
        prefab_component
            .values
            .insert("asset".into(), Value::Asset(asset));
        prefab_component
            .values
            .insert("entity_path".into(), Value::VecUInt64(entity_path));
        prefab_component
            .values
            .insert("root_entity".into(), Value::Entity(root_entity));
    }

    /// Cut useless data in self before saving to file:
    /// 1. Erase generation value of [EntityId].
    /// 2. Skip read only values.
    pub fn cut(&self) -> Self {
        let mut entity_data_cut = EntityData::default();
        for (comopnent_name, component_data) in &self.components {
            let cut_read_only = comopnent_name != "Parent" && comopnent_name != "Children"; // TODO: use a more generic way to allow read-only values of some components to save to file
            let component_data_cut = component_data.cut(cut_read_only);
            entity_data_cut
                .components
                .insert(comopnent_name.clone(), component_data_cut);
        }
        entity_data_cut
    }
}

/// A collection of [EntityData]. This is a wrapper of [IndexMap<EntityId, EntityData>].
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EntitiesData(
    #[serde(with = "vectorize")] // TODO: #[serde_as(as = "Vec<(_, _)>")]
    pub  IndexMap<EntityId, EntityData>,
);

impl EntitiesData {
    /// Cut useless data in self before saving to file:
    /// 1. Erase generation value of [EntityId].
    /// 2. Skip read only values.
    pub fn cut(&self) -> Self {
        let mut entities_data_cut = EntitiesData::default();
        for (eid, entity_data) in self {
            entities_data_cut.insert(Data::erase_generation(*eid), entity_data.cut());
        }
        entities_data_cut
    }

    /// Get first found entity id that do not have parent entity which is in this entities data.
    pub fn root(&self) -> Option<EntityId> {
        for (&e, entity_data) in self {
            if !self.contains_key(&entity_data.parent()) {
                return Some(e);
            }
        }
        None
    }

    /// Get all entity ids that do not have parent entity which is in this entities data.
    pub fn roots(&self) -> Vec<EntityId> {
        let mut roots = Vec::new();
        for (&e, entity_data) in self {
            if !self.contains_key(&entity_data.parent()) {
                roots.push(e);
            }
        }
        roots
    }

    /// Checks if all entities are directly or indirectly attached to the first entity.
    /// The returned map is a mapping of entities to whether they pass the check.
    pub(crate) fn check_attached_to_first_entity(&self) -> HashMap<EntityId, bool> {
        let mut result = HashMap::new();
        result.insert(*self.get_index(0).unwrap().0, true);
        for &e in self.keys() {
            self.check_attached_to_first_entity_recursive(e, &mut result);
        }
        result
    }

    fn check_attached_to_first_entity_recursive(
        &self,
        e: EntityId,
        result: &mut HashMap<EntityId, bool>,
    ) -> bool {
        if !result.contains_key(&e) {
            let entity_data = self.get(&e).unwrap();
            let parent = entity_data.parent();
            let r = if self.contains_key(&parent) {
                self.check_attached_to_first_entity_recursive(parent, result)
            } else {
                false
            };
            result.insert(e, r);
        }
        result[&e]
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
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct UniquesData(pub IndexMap<String, Data>);

impl UniquesData {
    /// Cut useless data in self before saving to file:
    /// 1. Erase generation value of [EntityId].
    /// 2. Skip read only values.
    pub fn cut(&self) -> UniquesData {
        let mut uniques_data_cut = UniquesData::default();
        for (unique_name, unique_data) in self {
            let cut_read_only = unique_name != "Hierarchy"; // TODO: use a more generic way to allow read-only values of some uniques to save to file
            let unique_data_cut = unique_data.cut(cut_read_only);
            uniques_data_cut.insert(unique_name.clone(), unique_data_cut);
        }
        uniques_data_cut
    }

    /// For every data_value in self, call `f` with (unique_name, data_name, data_value), to create a new [UniquesData].
    pub fn map_value(&self, mut f: impl FnMut(&String, &String, &Value) -> Value) -> UniquesData {
        UniquesData(
            self.iter()
                .map(|(unique_name, unique_data)| {
                    (
                        unique_name.clone(),
                        Data {
                            values: unique_data
                                .values
                                .iter()
                                .map(|(data_name, data_value)| {
                                    (data_name.clone(), f(unique_name, data_name, data_value))
                                })
                                .collect(),
                            limits: unique_data.limits.clone(),
                        },
                    )
                })
                .collect(),
        )
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

/// WorldData contains all entity data and unique data in the world.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct WorldData {
    pub entities: EntitiesData,
    pub uniques: UniquesData,
}

impl WorldData {
    /// The WorldData becomes empty after clear.
    pub fn clear(&mut self) {
        self.entities.clear();
        self.uniques.clear();
    }

    /// Cut useless data in self before saving to file:
    /// 1. Erase generation value of [EntityId].
    /// 2. Skip read only values.
    pub fn cut(&self) -> WorldData {
        WorldData {
            entities: self.entities.cut(),
            uniques: self.uniques.cut(),
        }
    }
}

/// SceneData is a compressed version of [WorldData].
/// SceneData stores prefabs by their asset ids while [WorldData] stores all data for every entity.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SceneData {
    /// SceneData regard all eneities in world as one prefab.
    pub entities: PrefabData,
    /// All uniques data.
    pub uniques: UniquesData,
    /// All id paths in uniques. unique_name -> data_name -> entity_id_path.
    pub unique_id_paths: DataEntityIdPaths,
}

impl SceneData {
    /// Create [SceneData] from [WorldData].
    pub fn new(
        world_data: &WorldData,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
    ) -> Self {
        Self::new_with_prefab_data_override(world_data, get_prefab_data_fn, None)
    }

    /// See [PrefabData::new_with_prefab_data_override].
    pub fn new_with_prefab_data_override(
        world_data: &WorldData,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
        prefab_data_override: Option<(EntityId, Arc<PrefabData>)>,
    ) -> Self {
        let (prefab_data, prefab_root_entity_to_nested_prefabs_index) =
            PrefabData::new_with_prefab_data_override(
                &world_data.entities,
                get_prefab_data_fn,
                prefab_data_override,
            );
        let mut unique_id_paths = DataEntityIdPaths::default();
        let uniques = world_data
            .uniques
            .map_value(|unique_name, data_name, data_value| {
                PrefabData::convert_value(
                    unique_name,
                    data_name,
                    data_value,
                    &world_data.entities,
                    &prefab_root_entity_to_nested_prefabs_index,
                    &mut unique_id_paths,
                )
            });
        SceneData {
            entities: prefab_data,
            uniques,
            unique_id_paths,
        }
    }

    /// Cut useless data in self before saving to file:
    /// 1. Erase generation value of [EntityId].
    /// 2. Skip read only values.
    pub fn cut(&mut self) {
        self.entities.cut();
        self.uniques = self.uniques.cut();
    }

    /// Convert [SceneData] to [WorldData]. Also return a map that maps every
    /// [EntityIdWithPath] in this [SceneData] to a new [EntityId] in [WorldData].
    pub fn to_world_data(
        &self,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
    ) -> (WorldData, HashMap<EntityIdWithPath, EntityId>) {
        let (entities, entity_map) = self.entities.to_entities_data(get_prefab_data_fn);
        let uniques = self
            .uniques
            .map_value(|unique_name, data_name, data_value| {
                let id_path = self
                    .unique_id_paths
                    .get(unique_name)
                    .and_then(|id_paths| id_paths.get(data_name));
                data_value.map_entity_with_path(id_path, |ep| {
                    entity_map.get(&ep).cloned().unwrap_or_default()
                })
            });
        (WorldData { entities, uniques }, entity_map)
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
