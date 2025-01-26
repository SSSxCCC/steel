use crate::{
    asset::AssetId,
    camera::SceneCamera,
    data::{EntitiesData, SceneData, WorldData},
    platform::Platform,
    prefab::{EntityIdWithPath, PrefabData},
};
use glam::UVec2;
use shipyard::EntityId;
use std::{
    collections::HashMap,
    error::Error,
    path::{Path, PathBuf},
    sync::Arc,
};
use vulkano::{image::view::ImageView, sync::GpuFuture};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};
use winit::event::WindowEvent;

/// The App trait defines many functions called by steel-editor or steel-client to control the running of steel application.
/// You usually do not need to manually implement this trait, just use steel::app::SteelApp.
pub trait App {
    fn init(&mut self, info: InitInfo);
    fn update(&mut self, info: UpdateInfo);
    fn draw(&mut self, info: DrawInfo) -> Box<dyn GpuFuture>;
    fn command(&self, cmd: Command);
}

/// The InitInfo contains some initialization data, and is passed to [App::init].
pub struct InitInfo<'a> {
    pub platform: Platform,
    pub context: &'a VulkanoContext,
    pub ray_tracing_supported: bool,
    pub scene: Option<AssetId>,
}

/// The UpdateInfo contains some data about current frame, and is passed to [App::update] every frame.
/// You can get current frame stage from FrameInfo::stage.
pub struct UpdateInfo<'a> {
    /// If true, run PreUpdate, Update, PostUpdate systems,
    /// if false, only run PreUpdate and PostUpdate systems.
    /// This is false in editor when game is not running.
    pub update: bool,
    pub ctx: &'a egui::Context,
}

/// The DrawInfo contains some drawing data, and is passed to [App::draw] every frame.
pub struct DrawInfo<'a> {
    pub before_future: Box<dyn GpuFuture>,
    pub context: &'a VulkanoContext,
    pub renderer: &'a VulkanoWindowRenderer,
    /// The image we will draw.
    pub image: Arc<ImageView>,
    /// The window size, this is the pixel size of the image we will draw.
    pub window_size: UVec2,
    /// If editor_info is some, we are drawing for the editor window.
    pub editor_info: Option<EditorInfo<'a>>,
}

/// The EditorInfo contains some drawing data specific to the editor scene window,
/// and is contained in DrawInfo, which is passed to [App::draw] every frame.
pub struct EditorInfo<'a> {
    pub camera: &'a SceneCamera,
}

/// Command is sent by editor through [App::command] method to read the game world.
pub enum Command<'a> {
    Save(&'a mut WorldData),
    Load(&'a WorldData),
    Reload(&'a SceneData),
    SetCurrentScene(Option<AssetId>),

    CreateEntity,
    /// entities_data, old_id_to_new_id map
    AddEntities(&'a EntitiesData, &'a mut HashMap<EntityId, EntityId>),
    DestroyEntity(EntityId),
    ClearEntity,
    GetEntityCount(&'a mut usize),
    /// window_index (WindowIndex::GAME or WindowIndex::SCENE), screen_position, out_eid.
    GetEntityAtScreen(usize, UVec2, &'a mut EntityId),

    CreateComponent(EntityId, &'static str),
    DestroyComponent(EntityId, &'a String),
    GetComponents(&'a mut Vec<&'static str>),

    // attached_entity, parent, before
    AttachBefore(EntityId, EntityId, EntityId),
    // attached_entity, parent, after
    AttachAfter(EntityId, EntityId, EntityId),

    UpdateInput(&'a Vec<WindowEvent<'static>>),

    ResetTime,

    GetAssetPath(AssetId, &'a mut Option<PathBuf>),
    GetAssetContent(AssetId, &'a mut Option<Arc<Vec<u8>>>),
    AssetIdExists(AssetId, &'a mut bool),
    InsertAsset(AssetId, PathBuf),
    DeleteAsset(AssetId),
    DeleteAssetDir(&'a Path),
    UpdateAssetPath(AssetId, PathBuf),

    GetPrefabData(AssetId, &'a mut Option<Arc<PrefabData>>),
    /// prefab_root_entity, prefab_asset, prefab_root_entity_to_nested_prefabs_index
    CreatePrefab(EntityId, AssetId, HashMap<EntityId, u64>),
    /// prefab_root_entity, prefab_asset, entity_id_to_prefab_entity_id_with_path
    LoadPrefab(EntityId, AssetId, HashMap<EntityId, EntityIdWithPath>),
    /// prefab_asset, prefab_root_entity
    AddEntitiesFromPrefab(AssetId, &'a mut Result<EntityId, Box<dyn Error>>),
}

/// Helper struct to define window index constants: WindowIndex::GAME and WindowIndex::SCENE.
pub struct WindowIndex;

impl WindowIndex {
    /// The game window in editor or client.
    pub const GAME: usize = 0;
    /// The scene window in editor.
    pub const SCENE: usize = 1;
}
