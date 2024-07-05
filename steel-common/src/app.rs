use std::{path::PathBuf, sync::Arc};
use glam::{UVec2, Vec3};
use vulkano::{sync::GpuFuture, image::view::ImageView};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};
use shipyard::EntityId;
use winit::event::WindowEvent;
use crate::{data::WorldData, platform::Platform};

/// The App trait defines many functions called by steel-editor or steel-client to control the running of steel application.
/// You usually do not need to manually implement this trait, just use steel::app::SteelApp.
pub trait App {
    fn init(&mut self, info: InitInfo);
    fn update(&mut self, info: UpdateInfo);
    fn draw(&mut self, info: DrawInfo) -> Box<dyn GpuFuture>;
    fn command(&mut self, cmd: Command);
}

/// The InitInfo contains some initialization data, and is passed to [App::init].
pub struct InitInfo<'a> {
    pub platform: Platform,
    pub context: &'a VulkanoContext,
    pub scene: Option<PathBuf>,
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
    pub camera: &'a EditorCamera
}

/// Camera info for editor window.
pub struct EditorCamera {
    pub position: Vec3,
    pub height: f32,
}

/// Command is sent by editor through [App::command] method to modify the game world.
pub enum Command<'a> {
    Save(&'a mut WorldData),
    Load(&'a WorldData),
    Reload(&'a WorldData),
    SetCurrentScene(Option<PathBuf>),

    CreateEntity,
    DestroyEntity(EntityId),
    ClearEntity,

    GetComponents(&'a mut Vec<&'static str>),
    CreateComponent(EntityId, &'static str),
    DestroyComponent(EntityId, &'a String),

    UpdateInput(&'a Vec<WindowEvent<'static>>),

    /// window_index (WindowIndex::GAME or WindowIndex::SCENE), screen_position, out_eid.
    GetEntityAtScreen(usize, UVec2, &'a mut EntityId),

    ResetTime,

    // attached_entity, parent, before
    AttachEntity(EntityId, EntityId, EntityId),
}

/// Helper struct to define window index constants: WindowIndex::GAME and WindowIndex::SCENE.
pub struct WindowIndex;

impl WindowIndex {
    /// The game window in editor or client.
    pub const GAME: usize = 0;
    /// The scene window in editor.
    pub const SCENE: usize = 1;
}
