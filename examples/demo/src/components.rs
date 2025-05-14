use glam::{IVec2, IVec3, IVec4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};
use shipyard::{Component, EntityId};
use steel::{
    asset::AssetId,
    data::{Data, Limit, Value},
    edit::Edit,
};

/// This is a tag component. It doesn't have any data, but it can be used to mark entities.
#[derive(Component, Edit, Default)]
pub struct TagComponent;

/// This is a demo component. It has various types of data, including primitive types, strings, and vectors.
#[derive(Component, Edit, Default)]
pub struct DemoComponent {
    bool: bool,
    int32: i32,
    uint32: u32,
    float32: f32,
    string: String,
    entity: EntityId,
    asset: AssetId,
    inner: InnerStruct,
    vec2: Vec2,
    vec3: Vec3,
    vec4: Vec4,
    vec_bool: Vec<bool>,
    vec_i32: Vec<i32>,
    vec_i64: Vec<i64>,
    vec_u32: Vec<u32>,
    vec_u64: Vec<u64>,
    vec_f32: Vec<f32>,
    vec_f64: Vec<f64>,
    vec_string: Vec<String>,
    vec_entity: Vec<EntityId>,
    vec_asset: Vec<AssetId>,
    vec_struct: Vec<InnerStruct>,
    #[edit(ignore)]
    _no_edit: NoEditStruct,
}

/// This is a nested struct used in the DemoComponent. It contains various value types.
#[derive(Edit, Default, Clone)]
struct InnerStruct {
    ivec2: IVec2,
    #[edit(limit = "Limit::ReadOnly")]
    ivec3: IVec3,
    inner2: InnerStruct2,
    ivec4: IVec4,
    entity: EntityId,
    vec_entity: Vec<EntityId>,
}

/// This is another nested struct used in the InnerStruct. It contains unsigned vector types.
#[derive(Edit, Default, Clone)]
struct InnerStruct2 {
    uvec2: UVec2,
    uvec3: UVec3,
    uvec4: UVec4,
}

/// This is a struct that doesn't not implement Edit.
#[derive(Default)]
struct NoEditStruct;
