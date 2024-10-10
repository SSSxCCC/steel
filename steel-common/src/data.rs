use crate::asset::AssetId;
use glam::{IVec2, IVec3, IVec4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use shipyard::EntityId;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ops::RangeInclusive,
    sync::Arc,
};

/// Limit Value in a range or in several enum, mainly used in Edit::get_data.
#[derive(Debug, Clone)]
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
    /// Limit each element in a Vec2/Vec3/Vec4 to a different range.
    VecRange(Vec<Option<RangeInclusive<f32>>>),
    /// Limit each element in a IVec2/IVec3/IVec4 to a different range.
    IVecRange(Vec<Option<RangeInclusive<i32>>>),
    /// Limit each element in a UVec2/UVec3/UVec4 to a different range.
    UVecRange(Vec<Option<RangeInclusive<u32>>>),
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
}

/// Data contains all Value with Limit in a component or unique.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
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
        Self::default()
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

    /// Cut useless data in self before saving to file:
    /// 1. Erase generation value of EntityId.
    /// 2. Skip read only values if cut_read_only is true.
    pub fn cut(&self, cut_read_only: bool) -> Self {
        let mut data_cut = Data::new();
        for (name, value) in &self.values {
            if !(cut_read_only && matches!(self.limits.get(name), Some(Limit::ReadOnly))) {
                let value = match value {
                    Value::Entity(e) => Value::Entity(Self::erase_generation(*e)),
                    Value::VecEntity(v) => {
                        Value::VecEntity(v.iter().map(|e| Self::erase_generation(*e)).collect())
                    }
                    _ => value.clone(),
                };
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

/// EntityData contains all component Data in a entity.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EntityData {
    pub components: IndexMap<String, Data>,
}

impl EntityData {
    /// Get the name in Name component.
    /// Returns None if it does not exist.
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
    /// Returns EntityId::dead() if this entity is at top layer.
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
    /// Returns None if there are no children.
    pub fn children(&self) -> Option<&Vec<EntityId>> {
        self.components
            .get("Children")
            .map(|children| match children.get("unnamed-0") {
                Some(Value::VecEntity(v)) => v,
                _ => panic!("Children entity vector not found in Children component: {children:?}"),
            })
    }

    /// Get prefab_asset, prefab_entity_index, prefab_entity_path, and prefab_root_entity at the same time.
    /// Returns None if this entity is not created from a prefab.
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
    /// Returns None if this entity is not in a prefab.
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
    /// Returns None if this entity is not in a prefab.
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
    /// Returns None if this entity is not in a prefab.
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
    /// Returns None if this entity is not in a prefab.
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
    fn update_prefab_info(
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
    /// 1. Erase generation value of EntityId.
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

/// A collection of EntityData. This is a wrapper of IndexMap<EntityId, EntityData>.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EntitiesData(
    #[serde(with = "vectorize")] // TODO: #[serde_as(as = "Vec<(_, _)>")]
    pub  IndexMap<EntityId, EntityData>,
);

impl EntitiesData {
    /// Cut useless data in EntitiesData before saving to file:
    /// 1. Erase generation value of EntityId.
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
    fn check_attached_to_first_entity(&self) -> HashMap<EntityId, bool> {
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
    /// 1. Erase generation value of EntityId.
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

    /// For every data_value in self, call `f` with (unique_name, data_name, data_value), to create a new UniquesData.
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

/// WorldData contains all EntityData and UniqueData in the world.
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
/// SceneData stores prefabs by their asset ids while WorldData stores all data for every entity.
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
    /// 1. Erase generation value of EntityId.
    /// 2. Skip read only values.
    pub fn cut(&mut self) {
        self.entities.cut();
        self.uniques = self.uniques.cut();
    }

    /// Convert SceneData to WorldData. Also return a map that maps every
    /// EntityIdWithPath in this SceneData to a new EntityId in WorldData.
    pub fn to_world_data(
        &self,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
    ) -> (WorldData, HashMap<EntityIdWithPath, EntityId>) {
        let (entities, entity_map) = self.entities.to_entities_data(get_prefab_data_fn);
        let uniques = self
            .uniques
            .map_value(|unique_name, data_name, data_value| {
                if let Some(id_path) = self
                    .unique_id_paths
                    .get(unique_name)
                    .and_then(|id_paths| id_paths.get(data_name))
                {
                    if let (Value::Entity(e), EntityIdPathInValue::EntityId(p)) =
                        (data_value, id_path)
                    {
                        return Value::Entity(
                            entity_map
                                .get(&EntityIdWithPath(*e, p.clone()))
                                .cloned()
                                .unwrap_or_default(),
                        );
                    } else if let (Value::VecEntity(es), EntityIdPathInValue::EntityVec(ps)) =
                        (data_value, id_path)
                    {
                        return Value::VecEntity(
                            es.iter()
                                .enumerate()
                                .map(|(i, e)| {
                                    ps.get(&(i as u64))
                                        .and_then(|p| {
                                            entity_map.get(&EntityIdWithPath(*e, p.clone()))
                                        })
                                        .cloned()
                                        .unwrap_or_default()
                                })
                                .collect(),
                        );
                    }
                }
                data_value.clone()
            });
        (WorldData { entities, uniques }, entity_map)
    }
}

/// Prefab is a collection of entities, which stored in an asset, to be used to
/// create entities as template. A prefab can have many nested prefabs.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PrefabData {
    /// All entities with id paths in self prefab and all modifications from the original prefab in nested prefabs.
    /// for keys whose path is empty, it means the entity is in self prefab.
    /// The first entity should be the root entity of the prefab.
    #[serde(with = "vectorize")] // TODO: #[serde_as(as = "Vec<(_, _)>")]
    pub entities: IndexMap<EntityIdWithPath, EntityDataWithIdPaths>,
    /// All nested prefabs.
    pub nested_prefabs: Vec<AssetId>,
    /// Deleted entities and components in nested prefabs.
    /// Value is a set that contains all the components deleted in the key entity,
    /// if it is an empty set, it means this key entity is deleted.
    #[serde(with = "vectorize")] // TODO: #[serde_as(as = "Vec<(_, _)>")]
    pub delete: IndexMap<EntityIdWithPath, IndexSet<String>>,
}

impl PrefabData {
    /// Create a new prefab data from entities, also return prefab_root_entity_to_nested_prefabs_index map.
    /// The first entity of input entities must be the root, so that after creating the prefab,
    /// we can regard the first entity as the root entity.
    pub fn new(
        entities: &EntitiesData,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
    ) -> (Self, HashMap<EntityId, u64>) {
        Self::new_with_prefab_data_override(entities, get_prefab_data_fn, None)
    }

    /// This function has an extra parameter 'prefab_data_override' compared to [Self::new].
    /// prefab_data_override defines a different [PrefabData] for an prefab root entity, this is used for updating a prefab.
    pub fn new_with_prefab_data_override(
        entities: &EntitiesData,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
        prefab_data_override: Option<(EntityId, Arc<PrefabData>)>,
    ) -> (Self, HashMap<EntityId, u64>) {
        // prepare entity maps for values comparison between entities and prefabs,
        // and prepare prefab_root_entity to nested_prefabs_index map for id path generation
        let mut entity_maps: HashMap<_, HashMap<_, _>> = HashMap::new(); // prefab_root_entity_id in entities -> entity_id_with_path in prefab -> entity_id in entities
        let mut prefab_root_entity_to_nested_prefabs_index = HashMap::new();
        let mut nested_prefabs = Vec::new();
        for (&id, entity_data) in entities {
            if let Some((
                prefab_asset,
                prefab_entity_index,
                prefab_entity_path,
                prefab_root_entity,
            )) = entity_data.prefab_info()
            {
                if entities.contains_key(&prefab_root_entity) {
                    entity_maps.entry(prefab_root_entity).or_default().insert(
                        EntityIdWithPath(
                            EntityId::new_from_index_and_gen(prefab_entity_index, 0),
                            prefab_entity_path.clone(),
                        ),
                        id,
                    );
                    prefab_root_entity_to_nested_prefabs_index
                        .entry(prefab_root_entity)
                        .or_insert_with(|| {
                            nested_prefabs.push(prefab_asset);
                            nested_prefabs.len() as u64 - 1
                        });
                }
            }
        }

        // calculate prefab contents
        let mut prefab_entities = IndexMap::new();
        let mut prefab_delete = IndexMap::<_, IndexSet<_>>::new();
        let mut existing_entities_in_prefabs = HashSet::new();
        for (&id, entity_data) in entities {
            if let Some((
                prefab_asset,
                prefab_entity_index,
                prefab_entity_path,
                prefab_root_entity,
            )) = entity_data.prefab_info()
            {
                // we only consider this entity to belong to a nested prefab if the root entity of this entity's prefab is also in entities
                if entities.contains_key(&prefab_root_entity) {
                    // get the prefab data of this entity
                    if let Some(prefab_data) = &prefab_data_override
                        .as_ref()
                        .filter(|(root_entity, _)| *root_entity == prefab_root_entity)
                        .map(|(_, prefab_data)| Some(prefab_data.clone()))
                        .unwrap_or_else(|| get_prefab_data_fn(prefab_asset))
                    {
                        // get the entity data in this entity's prefab
                        let prefab_entity =
                            EntityId::new_from_index_and_gen(prefab_entity_index, 0);
                        if let Some(prefab_entity_data) = prefab_data.get_entity_data(
                            prefab_entity,
                            prefab_entity_path,
                            entity_maps
                                .get(&prefab_root_entity)
                                .as_ref()
                                .expect("All prefab root entity should be in entity_maps!"),
                            get_prefab_data_fn,
                        ) {
                            // get the index in nested_prefabs of this entity
                            let nested_prefab_index = *prefab_root_entity_to_nested_prefabs_index
                                .get(&prefab_root_entity)
                                .expect("prefab_root_entity_to_nested_prefabs_index should have been initialized!");

                            // generate EntityIdWithPath in new prefab
                            let mut path = vec![nested_prefab_index];
                            path.extend(prefab_entity_path);
                            let entity_id_with_path = EntityIdWithPath(prefab_entity, path);

                            // compare entity_data and prefab_entity_data and record differences
                            for (component_name, component_data) in &entity_data.components {
                                if component_name == "Prefab" {
                                    continue; // not store Prefab component because it is noly used at runtime
                                }

                                if let Some(prefab_component_data) =
                                    prefab_entity_data.components.get(component_name)
                                {
                                    // this component is in both entity_data and prefab_entity_data, compare them
                                    for (data_name, data_value) in &component_data.values {
                                        if let Some(prefab_data_value) =
                                            prefab_component_data.values.get(data_name)
                                        {
                                            // this value is in both component_data and prefab_component_data, record difference if not equal
                                            if data_value != prefab_data_value {
                                                Self::record_value(
                                                    entity_id_with_path.clone(),
                                                    component_name,
                                                    data_name,
                                                    data_value,
                                                    component_data.limits.get(data_name),
                                                    entities,
                                                    &prefab_root_entity_to_nested_prefabs_index,
                                                    &mut prefab_entities,
                                                );
                                            }
                                        } else {
                                            // this value is only in component_data, record difference
                                            Self::record_value(
                                                entity_id_with_path.clone(),
                                                component_name,
                                                data_name,
                                                data_value,
                                                component_data.limits.get(data_name),
                                                entities,
                                                &prefab_root_entity_to_nested_prefabs_index,
                                                &mut prefab_entities,
                                            );
                                        }
                                    }
                                } else {
                                    // this component is not in the prefab, record this component

                                    // it is important to insert component here because there will be no chance for empty data component to be inserted
                                    prefab_entities
                                        .entry(entity_id_with_path.clone())
                                        .or_default()
                                        .0
                                        .components
                                        .entry(component_name.clone())
                                        .or_default();

                                    // record data values in this component
                                    for (data_name, data_value) in &component_data.values {
                                        Self::record_value(
                                            entity_id_with_path.clone(),
                                            component_name,
                                            data_name,
                                            data_value,
                                            component_data.limits.get(data_name),
                                            entities,
                                            &prefab_root_entity_to_nested_prefabs_index,
                                            &mut prefab_entities,
                                        );
                                    }
                                }
                            }

                            // record deleted components in this entity
                            let delete_components = prefab_entity_data
                                .components
                                .keys()
                                .filter(|&c| !entity_data.components.contains_key(c))
                                .cloned()
                                .collect::<IndexSet<_>>();
                            if !delete_components.is_empty() {
                                prefab_delete
                                    .entry(entity_id_with_path.clone())
                                    .or_default()
                                    .extend(delete_components);
                            }

                            // record that this entity is not deleted
                            existing_entities_in_prefabs.insert(entity_id_with_path);

                            // current entity is a prefab and has been saved as nested prefab
                            continue;
                        }
                    }
                }
            }
            // current entity is not a prefab
            for (component_name, component_data) in &entity_data.components {
                if component_name == "Prefab" {
                    continue; // not store Prefab component because it is noly used at runtime
                }

                // entity id path of entity not in prefab is empty
                let entity_id_with_path = EntityIdWithPath(id, EntityIdPath::new());

                // it is important to insert component here because there will be no chance for empty data component to be inserted
                prefab_entities
                    .entry(entity_id_with_path.clone())
                    .or_default()
                    .0
                    .components
                    .entry(component_name.clone())
                    .or_default();

                // record data values in this component
                for (data_name, data_value) in &component_data.values {
                    Self::record_value(
                        entity_id_with_path.clone(),
                        component_name,
                        data_name,
                        data_value,
                        component_data.limits.get(data_name),
                        entities,
                        &prefab_root_entity_to_nested_prefabs_index,
                        &mut prefab_entities,
                    );
                }
            }
        }

        // record deleted entities.
        for (i, &nested_prefab) in nested_prefabs.iter().enumerate() {
            if let Some(prefab_data) = get_prefab_data_fn(nested_prefab) {
                // we use prepare_entity_map to get all entities in this nested prefab, the value of this map is useless for us
                let nested_entity_map = prefab_data.prepare_entity_map(get_prefab_data_fn);
                for (EntityIdWithPath(e, p), _) in nested_entity_map {
                    // generate EntityIdWithPath in new prefab
                    let mut path = vec![i as u64];
                    path.extend(p);
                    let entity_id_with_path = EntityIdWithPath(e, path);

                    // if this entity exists in prefab but not in entities data, it means that this entity was deleted
                    if !existing_entities_in_prefabs.contains(&entity_id_with_path) {
                        // empty value means that this entity is deleted
                        prefab_delete.insert(entity_id_with_path, Default::default());
                    }
                }
            }
        }

        (
            PrefabData {
                entities: prefab_entities,
                nested_prefabs,
                delete: prefab_delete,
            },
            prefab_root_entity_to_nested_prefabs_index,
        )
    }

    /// Record value in prefab_entities.
    fn record_value(
        entity_id_with_path: EntityIdWithPath,
        component_name: &String,
        data_name: &String,
        data_value: &Value,
        data_limit: Option<&Limit>,
        entities: &EntitiesData,
        prefab_root_entity_to_nested_prefabs_index: &HashMap<EntityId, u64>,
        prefab_entities: &mut IndexMap<EntityIdWithPath, EntityDataWithIdPaths>,
    ) {
        let entity_data_override = prefab_entities.entry(entity_id_with_path).or_default();
        let component_data_override = entity_data_override
            .0
            .components
            .entry(component_name.clone())
            .or_default();

        // for EntityId value, we need to also store its id path,
        // or set them to Entity::dead() if it's not in the prefab_entities
        let data_value = Self::convert_value(
            component_name,
            data_name,
            data_value,
            entities,
            prefab_root_entity_to_nested_prefabs_index,
            &mut entity_data_override.1,
        );

        if let Some(data_limit) = data_limit {
            // also record the limit if exists, because PrefabData will be cut
            // before saving to file, and the cutting process depends on the limit
            component_data_override.add_value_with_limit(data_name, data_value, data_limit.clone());
        } else {
            component_data_override.add_value(data_name, data_value);
        }
    }

    /// Convert entity ids in [Value] to entity id with path in prefab.
    fn convert_value(
        component_or_unique_name: &String,
        data_name: &String,
        data_value: &Value,
        entities: &EntitiesData,
        prefab_root_entity_to_nested_prefabs_index: &HashMap<EntityId, u64>,
        id_paths: &mut DataEntityIdPaths,
    ) -> Value {
        match data_value {
            Value::Entity(e) => {
                let (entity_id, id_path) =
                    Self::convert_entity(e, entities, prefab_root_entity_to_nested_prefabs_index);
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
                        let (entity_id, id_path) = Self::convert_entity(
                            e,
                            entities,
                            prefab_root_entity_to_nested_prefabs_index,
                        );
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
            _ => data_value.clone(),
        }
    }

    /// Convert entity id in entities to entity id with path in prefab.
    fn convert_entity(
        e: &EntityId,
        entities: &EntitiesData,
        prefab_root_entity_to_nested_prefabs_index: &HashMap<EntityId, u64>,
    ) -> (EntityId, Option<EntityIdPath>) {
        if let Some(entity_data) = entities.get(e) {
            if let Some((prefab_entity_index, id_path)) = entity_data.prefab_info().and_then(
                |(_, prefab_entity_index, prefab_entity_path, prefab_root_entity)| {
                    prefab_root_entity_to_nested_prefabs_index
                        .get(&prefab_root_entity)
                        .map(|&nested_prefab_index| {
                            let mut id_path = vec![nested_prefab_index];
                            id_path.extend(prefab_entity_path);
                            (prefab_entity_index, id_path)
                        })
                },
            ) {
                // this entity is in the nested prefab, we need to store it's id_path
                (
                    EntityId::new_from_index_and_gen(prefab_entity_index, 0),
                    Some(id_path),
                )
            } else {
                // this entity is not in the nested prefab, we store it's id_path as empty
                (*e, Some(EntityIdPath::new()))
            }
        } else {
            // this entity is not in the prefab_entities, set it to EntityId::dead()
            (EntityId::dead(), None)
        }
    }

    /// Get EntityData of entity_id with id_path.
    fn get_entity_data(
        &self,
        id: EntityId,
        id_path: &EntityIdPath,
        entity_map: &HashMap<EntityIdWithPath, EntityId>,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>>,
    ) -> Option<EntityData> {
        self.get_entity_data_recursive(
            id,
            id_path,
            0,
            entity_map,
            Default::default(),
            &mut HashSet::new(),
            get_prefab_data_fn,
        )
    }

    fn get_entity_data_recursive(
        &self,
        id: EntityId,
        id_path: &EntityIdPath,
        path_index: usize,
        entity_map: &HashMap<EntityIdWithPath, EntityId>,
        mut data_override: EntityDataWithIdPaths,
        deleted_components: &mut HashSet<String>,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>>,
    ) -> Option<EntityData> {
        if path_index < id_path.len() {
            // find entity in nested prefab
            if (id_path[path_index] as usize) < self.nested_prefabs.len() {
                if let Some(nested_prefab_data) =
                    get_prefab_data_fn(self.nested_prefabs[id_path[path_index] as usize])
                {
                    // get entity id with path in self prefab
                    let entity_id_with_path =
                        EntityIdWithPath(id, id_path[path_index..].iter().cloned().collect());

                    // get the components or entity that are deleted in self prefab
                    if let Some(self_deleted_components) = self.delete.get(&entity_id_with_path) {
                        if self_deleted_components.is_empty() {
                            // empty set means that this entity is deleted
                            return None;
                        }
                        // record deleted components in self prefab
                        deleted_components.extend(self_deleted_components.clone());
                    }

                    // get override entity data for this entity in sef prefab
                    if let Some(current_data_overide) = self.entities.get(&entity_id_with_path) {
                        data_override.merge(
                            current_data_overide,
                            deleted_components,
                            &id_path[..path_index],
                        );
                    }

                    // get entity data from nested prefab
                    return nested_prefab_data.get_entity_data_recursive(
                        id,
                        id_path,
                        path_index + 1,
                        entity_map,
                        data_override,
                        deleted_components,
                        get_prefab_data_fn,
                    );
                }
            }
            None
        } else {
            // find entity in self prefab
            self.entities
                .get(&EntityIdWithPath(id, Vec::new()))
                .map(|entity_data| {
                    data_override.merge(entity_data, deleted_components, id_path);
                    data_override.map(entity_map)
                })
        }
    }

    /// Cut useless data in self before saving to file:
    /// 1. Erase generation value of EntityId.
    /// 2. Skip read only values.
    pub fn cut(&mut self) {
        let entities = std::mem::take(&mut self.entities);
        for (mut id_with_path, mut entity_data_with_paths) in entities {
            id_with_path.0 = Data::erase_generation(id_with_path.0);
            entity_data_with_paths.0 = entity_data_with_paths.0.cut();
            self.entities.insert(id_with_path, entity_data_with_paths);
        }
    }

    /// Convert PrefabData to EntitiesData. Also return a map that maps every
    /// EntityIdWithPath in this PrefabData to a new EntityId in EntitiesData.
    pub fn to_entities_data(
        &self,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
    ) -> (EntitiesData, HashMap<EntityIdWithPath, EntityId>) {
        let entity_index_map = self.prepare_entity_map(get_prefab_data_fn);
        let entity_map = entity_index_map
            .iter()
            .map(|(k, &v)| (k.clone(), v))
            .collect();
        let mut entities_data = EntitiesData::default();
        for (EntityIdWithPath(id, path), &entity_id) in &entity_index_map {
            if let Some(entity_data) =
                self.get_entity_data(*id, path, &entity_map, get_prefab_data_fn)
            {
                entities_data.insert(entity_id, entity_data);
            }
        }
        (entities_data, entity_map)
    }

    /// Create a map that maps every EntityIdWithPath in self prefab to a new EntityId.
    /// This returns a IndexMap for a stable order of the entities to be stored.
    /// You can use .into_iter().collect() to convert to a HashMap.
    fn prepare_entity_map(
        &self,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
    ) -> IndexMap<EntityIdWithPath, EntityId> {
        let mut entity_map = IndexMap::new();
        self.prepare_entity_map_recursive(
            get_prefab_data_fn,
            &mut Vec::new(),
            &mut 0,
            &mut HashSet::new(),
            &mut entity_map,
        );
        entity_map
    }

    fn prepare_entity_map_recursive(
        &self,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
        id_path_prefix: &mut Vec<u64>,
        entity_id_index: &mut u64,
        deleted_entities: &mut HashSet<EntityIdWithPath>,
        entity_map: &mut IndexMap<EntityIdWithPath, EntityId>,
    ) {
        for (EntityIdWithPath(id, path), deleted_components) in &self.delete {
            // empty components list means that this entity is deleted.
            if deleted_components.is_empty() {
                let mut id_path = id_path_prefix.clone();
                id_path.extend(path);
                deleted_entities.insert(EntityIdWithPath(*id, id_path));
            }
        }
        for EntityIdWithPath(id, path) in self.entities.keys() {
            let mut id_path = id_path_prefix.clone();
            id_path.extend(path);
            let entity_id_with_path = EntityIdWithPath(*id, id_path);
            if !deleted_entities.contains(&entity_id_with_path) {
                entity_map.entry(entity_id_with_path).or_insert_with(|| {
                    *entity_id_index += 1;
                    EntityId::new_from_index_and_gen(*entity_id_index - 1, 0)
                });
            }
        }
        for (i, &asset_id) in self.nested_prefabs.iter().enumerate() {
            if let Some(prefab_data) = get_prefab_data_fn(asset_id) {
                id_path_prefix.push(i as u64);
                Self::prepare_entity_map_recursive(
                    &prefab_data,
                    get_prefab_data_fn,
                    id_path_prefix,
                    entity_id_index,
                    deleted_entities,
                    entity_map,
                );
                id_path_prefix.pop();
            }
        }
    }

    /// Update prefab data according to the input entities data which created from an exisiting prefab,
    /// also return entity_id_to_prefab_entity_id_with_path map. The first entity of input entities must be the root.
    pub fn update(
        entities: EntitiesData,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
    ) -> Result<(Self, HashMap<EntityId, EntityIdWithPath>), String> {
        // get self prefab asset id and old prefab data
        let (&root_entity, root_entity_data) = entities
            .get_index(0)
            .expect("PrefabData::update: no entity found");
        let (self_prefab_asset, _, _, self_prefab_root_entity) = root_entity_data
            .prefab_info()
            .expect("PrefabData::update: no prefab info in root entity");
        assert!(
            root_entity == self_prefab_root_entity,
            "PrefabData::update: first entity is not a prefab root"
        );
        let old_prefab_data =
            get_prefab_data_fn(self_prefab_asset).ok_or("PrefabData::update: prefab not found")?;

        // remove other entities that are or nest the same prefab to prevent prefabs from nesting themselves
        let mut prefab_asset_to_nested_self = HashMap::new();
        let entities = EntitiesData(
            entities
                .into_iter()
                .filter(|(_, entity_data)| {
                    if let Some((prefab_asset, _, _, prefab_root_entity)) =
                        entity_data.prefab_info()
                    {
                        if prefab_root_entity != root_entity {
                            if prefab_asset == self_prefab_asset {
                                return false;
                            } else {
                                if !prefab_asset_to_nested_self.contains_key(&prefab_asset) {
                                    prefab_asset_to_nested_self.insert(
                                        prefab_asset,
                                        get_prefab_data_fn(prefab_asset)
                                            .map(|prefab_data| {
                                                prefab_data
                                                    .all_nested_prefabs(get_prefab_data_fn)
                                                    .contains(&self_prefab_asset)
                                            })
                                            .unwrap_or(false),
                                    );
                                }
                                return !prefab_asset_to_nested_self[&prefab_asset];
                            }
                        }
                    }
                    true
                })
                .collect(),
        );

        // remove all entities that were detached due to previous deletions
        let attached_map = entities.check_attached_to_first_entity();
        let entities = EntitiesData(
            entities
                .into_iter()
                .filter(|(e, _)| attached_map[e])
                .collect(),
        );

        // map entity ids in entities to entity ids in the original prefab data.
        let mut entity_map = HashMap::new();
        let mut current_entity_index = 0;
        let prefab_entity_set = old_prefab_data
            .entities
            .keys()
            .filter(|EntityIdWithPath(_, p)| p.is_empty())
            .map(|EntityIdWithPath(e, _)| *e)
            .collect::<HashSet<_>>();
        for (&e, entity_data) in &entities {
            if let Some((_, prefab_entity_index, prefab_entity_path, prefab_root)) =
                entity_data.prefab_info()
            {
                if prefab_root == root_entity && prefab_entity_path.is_empty() {
                    entity_map.insert(e, EntityId::new_from_index_and_gen(prefab_entity_index, 0));
                    continue;
                }
            }
            loop {
                let new_entity_id = EntityId::new_from_index_and_gen(current_entity_index, 0);
                if !prefab_entity_set.contains(&new_entity_id) {
                    entity_map.insert(e, new_entity_id);
                    current_entity_index += 1;
                    break;
                }
                current_entity_index += 1;
            }
        }
        let mut mapped_entities = EntitiesData::default();
        for (e, mut entity_data) in entities {
            let e = entity_map[&e];
            entity_data.components.iter_mut().for_each(|(_, data)| {
                data.values.iter_mut().for_each(|(_, v)| match v {
                    Value::Entity(e) => *e = entity_map.get(e).cloned().unwrap_or_default(),
                    Value::VecEntity(es) => es
                        .iter_mut()
                        .for_each(|e| *e = entity_map.get(e).cloned().unwrap_or_default()),
                    _ => (),
                })
            });
            mapped_entities.insert(e, entity_data);
        }

        // revert prefab infos in mapped entities
        let mut nested_root_map = vec![None; old_prefab_data.nested_prefabs.len()];
        for (&e, entity_data) in &mapped_entities {
            if let Some((prefab_asset, prefab_entity_index, prefab_entity_path, _)) =
                entity_data.prefab_info()
            {
                // if prefab_asset == self_prefab_asset, this entity must be in self prefab,
                // because we have removed any entities that may cause self nesting
                if prefab_asset == self_prefab_asset && !prefab_entity_path.is_empty() {
                    if let Some(nested_prefab_data) = get_prefab_data_fn(
                        old_prefab_data.nested_prefabs[prefab_entity_path[0] as usize],
                    ) {
                        let EntityIdWithPath(
                            nested_prefab_root_entity,
                            nested_prefab_root_entity_path,
                        ) = nested_prefab_data
                            .entities
                            .get_index(0)
                            .expect("PrefabData::update: noraml prefab should have at least one entity!")
                            .0;
                        if nested_prefab_root_entity.index() == prefab_entity_index
                            && *nested_prefab_root_entity_path == prefab_entity_path[1..]
                        {
                            nested_root_map[prefab_entity_path[0] as usize] = Some(e);
                        }
                    }
                }
            }
        }
        for (_, entity_data) in &mut mapped_entities {
            if let Some((prefab_asset, _, prefab_entity_path, _)) = entity_data.prefab_info() {
                // if prefab_asset == self_prefab_asset, this entity must be in self prefab,
                // because we have removed any entities that may cause self nesting
                if prefab_asset == self_prefab_asset {
                    if prefab_entity_path.is_empty() {
                        entity_data.components.shift_remove("Prefab");
                    } else if let Some(revert_prefab_root_entity) =
                        nested_root_map[prefab_entity_path[0] as usize]
                    {
                        let revert_prefab_asset =
                            old_prefab_data.nested_prefabs[prefab_entity_path[0] as usize];
                        let revert_prefab_entity_path = prefab_entity_path[1..].to_vec();
                        entity_data.update_prefab_info(
                            revert_prefab_asset,
                            revert_prefab_entity_path,
                            revert_prefab_root_entity,
                        );
                    } else {
                        // the root entity of this prefab is not found, maybe it was removed
                        entity_data.components.shift_remove("Prefab");
                    }
                }
            }
        }

        // create new prefab data
        let (new_prefab_data, prefab_root_entity_to_nested_prefabs_index) =
            PrefabData::new(&mapped_entities, get_prefab_data_fn);

        // create entity_id to prefab_entity_id_with_path map for updating prefab
        let mut entity_id_to_prefab_entity_id_with_path = HashMap::new();
        // map back entity ids in entities.
        let entity_map = entity_map
            .into_iter()
            .map(|(k, v)| (v, k))
            .collect::<HashMap<_, _>>();
        for (e, entity_data) in &mapped_entities {
            if let Some((_, prefab_entity_index, prefab_entity_path, prefab_root_entity)) =
                entity_data.prefab_info()
            {
                let mut entity_path =
                    vec![prefab_root_entity_to_nested_prefabs_index[&prefab_root_entity]];
                entity_path.extend(prefab_entity_path);
                entity_id_to_prefab_entity_id_with_path.insert(
                    entity_map[e],
                    EntityIdWithPath(
                        EntityId::new_from_index_and_gen(prefab_entity_index, 0),
                        entity_path,
                    ),
                );
            } else {
                entity_id_to_prefab_entity_id_with_path
                    .insert(entity_map[e], EntityIdWithPath(*e, EntityIdPath::default()));
            }
        }

        // return the updated prefab data
        Ok((new_prefab_data, entity_id_to_prefab_entity_id_with_path))
    }

    /// Get all nested prefab asset ids in this prefab, including nested prefabs of nested prefabs.
    fn all_nested_prefabs(
        &self,
        get_prefab_data_fn: impl Fn(AssetId) -> Option<Arc<PrefabData>> + Copy,
    ) -> HashSet<AssetId> {
        let mut all_nested_prefabs = HashSet::new();
        let mut prefabs_to_search = self.nested_prefabs.clone();
        while let Some(prefab) = prefabs_to_search.pop() {
            if let Some(prefab_data) = get_prefab_data_fn(prefab) {
                all_nested_prefabs.insert(prefab);
                prefabs_to_search.extend(
                    prefab_data
                        .nested_prefabs
                        .iter()
                        .filter(|prefab| !all_nested_prefabs.contains(prefab))
                        .collect::<Vec<_>>(),
                );
            }
        }
        all_nested_prefabs
    }
}

/// EntityIdWithPath contains the entity id and its path in the prefab.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq, Hash)]
pub struct EntityIdWithPath(pub EntityId, pub EntityIdPath);

/// EntityDataWithIdPaths containts enitity data and all entity id paths in it.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EntityDataWithIdPaths(pub EntityData, pub DataEntityIdPaths);

impl EntityDataWithIdPaths {
    /// Merge other into self, leaving already existing data in self unchanged,
    /// insert front_id_path in front of all id paths in other before merging.
    pub fn merge(
        &mut self,
        other: &Self,
        exclude_components: &HashSet<String>,
        front_id_path: &[u64],
    ) {
        let vec_insert_front_fn = |p: &[u64]| {
            let mut r = front_id_path.iter().cloned().collect::<Vec<_>>();
            r.extend_from_slice(p);
            r
        };
        let insert_front_fn = |path: &EntityIdPathInValue| match path {
            EntityIdPathInValue::EntityId(p) => {
                EntityIdPathInValue::EntityId(vec_insert_front_fn(p))
            }
            EntityIdPathInValue::EntityVec(ps) => EntityIdPathInValue::EntityVec(
                ps.iter()
                    .map(|(&i, p)| (i, vec_insert_front_fn(p)))
                    .collect(),
            ),
        };
        for (component_name, component_data) in &other.0.components {
            if exclude_components.contains(component_name) {
                continue;
            }
            if let Some(self_component_data) = self.0.components.get_mut(component_name) {
                for (data_name, data_value) in &component_data.values {
                    if !self_component_data.values.contains_key(data_name) {
                        self_component_data
                            .values
                            .insert(data_name.clone(), data_value.clone());
                        if let Some(id_paths) = other
                            .1
                            .get(component_name)
                            .and_then(|id_paths| id_paths.get(data_name))
                        {
                            let id_paths = if front_id_path.is_empty() {
                                id_paths.clone()
                            } else {
                                // insert front id path to existing id path, so it is important to
                                // save empty id path as empty vector rather than not saving it.
                                insert_front_fn(id_paths)
                            };
                            self.1
                                .entry(component_name.clone())
                                .or_default()
                                .insert(data_name.clone(), id_paths);
                        }
                    }
                }
            } else {
                self.0
                    .components
                    .insert(component_name.clone(), component_data.clone());
                if let Some(id_paths) = other.1.get(component_name) {
                    let mut id_paths = id_paths.clone();
                    if !front_id_path.is_empty() {
                        // insert front id path to each existing id paths, so it is important to
                        // save empty id path as empty vector rather than not saving it.
                        id_paths.iter_mut().for_each(|(_, id_paths)| {
                            *id_paths = insert_front_fn(id_paths);
                        });
                    }
                    self.1.insert(component_name.clone(), id_paths);
                }
            }
        }
    }

    /// Convert all EntityIdWithPath to EntityId by entity_map.
    pub fn map(mut self, entity_map: &HashMap<EntityIdWithPath, EntityId>) -> EntityData {
        self.0.components.iter_mut().for_each(|(component_name, component_data)| {
            component_data.values.iter_mut().for_each(|(data_name, data_value)| {
                match data_value {
                    Value::Entity(e) => {
                        let id_path = if let Some(EntityIdPathInValue::EntityId(id_path)) = self.1.get(component_name).and_then(|id_paths| id_paths.get(data_name)) {
                            id_path.clone()
                        } else {
                            Vec::new()
                        };
                        let entity_id_with_path = EntityIdWithPath(*e, id_path);
                        *e = if let Some(e) = entity_map.get(&entity_id_with_path) {
                            *e
                        } else {
                            if *e != EntityId::dead() {
                                log::warn!("EntityDataWithIdPaths::map: {entity_id_with_path:?} not found.");
                            }
                            EntityId::dead()
                        };
                    }
                    Value::VecEntity(es) => {
                        let id_paths = if let Some(EntityIdPathInValue::EntityVec(id_paths)) = self.1.get(component_name).and_then(|id_paths| id_paths.get(data_name)) {
                            Cow::Borrowed(id_paths)
                        } else {
                            Cow::Owned(Default::default())
                        };
                        for (i, e) in es.iter_mut().enumerate() {
                            let entity_id_with_path = EntityIdWithPath(*e, id_paths.get(&(i as u64)).cloned().unwrap_or_default());
                            *e = if let Some(e) = entity_map.get(&entity_id_with_path) {
                                *e
                            } else {
                                log::warn!("EntityDataWithIdPaths::map: {entity_id_with_path:?} not found.");
                                EntityId::dead()
                            };
                        }
                    }
                    _ => (),
                }
            });
        });
        self.0
    }
}

/// EntityIdPath is the path to an entity in a prefab. Examples:
/// 1. \[\] means EntityId is in the current Prefab. (empty id path should store as empty vector rather than not storing it)
/// 2. \[a\] means EntityId is in the nested prefab at index a.
/// 3. \[a, b\] means the EntityId is in the nested prefab at index b of the nested prefab at index a.
pub type EntityIdPath = Vec<u64>;

/// All [EntityIdPath] in [Data]. component_name/unique_name -> data_name -> entity_id_path.
pub type DataEntityIdPaths = IndexMap<String, IndexMap<String, EntityIdPathInValue>>;

/// The [EntityIdPath] in [Value].
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EntityIdPathInValue {
    /// The [EntityIdPath] for [Value::Entity].
    EntityId(EntityIdPath),
    /// All [EntityIdPath] for [Value::VecEntity]. Key is the index of entity vector, there is no entry for EntityId::dead().
    EntityVec(IndexMap<u64, EntityIdPath>),
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
