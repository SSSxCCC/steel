use std::{path::PathBuf, sync::Arc};
use glam::{UVec2, Vec3};
use vulkano::{sync::GpuFuture, image::view::ImageView};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};
use shipyard::EntityId;
use winit::event::WindowEvent;
use crate::{data::WorldData, platform::Platform};

/// The engine trait defines many game lifecycle methods so that different game can implement different logic.
/// # Lifecycle
/// The steel game engine has three lifecyles: Engine::init, Engine::frame and Engine::draw.
/// ## Engine::init
/// Engine::init is called once when the application started, you can:
/// * Register components or uniques that should be edited in editor
/// * Add uniques to the ecs world
/// ## Engine::frame
/// Engine::frame is called three times every frame for three stages: FrameStage::Maintain, FrameStage::Update and FrameStage::Finish.
/// You can get current stage from info.stage. Engine::frame is mainly used to run your systems which need to run in each frame.
/// ### FrameStage::Maintain
/// This is the first stage in a frame. This stage is also called in editor even when game is not running,
/// so that the logic you write here will also affect editor every frame.
/// For example the physics maintain system should run in FrameStage::Maintain
/// so that the physics world is updated immediately when you add or remove physics components in editor.
/// ### FrameStage::Update
/// This is the second stage in a frame. This stage is skipped in editor when game is not running,
/// so that you can implement your game logic which should not affect editor.
/// For example the physics update system should run in FrameStage::Update
/// so that the physics bodies is not pulled by gravity when the game is not running in editor.
/// ### FrameStage::Finish
/// This is the third stage in a frame. This stage is also called in editor even when game is not running.
/// For example the render system can run in FrameStage::Finish to collect vertex data
/// after all entitiesa and compnents are updated.
/// ## Engine::draw
/// Engine::draw is called every frame after Engine::frame, and is called two times in editor for scene window and game window.
/// If info.editor_info is some, we are drawing for the scene window.
/// ## Engine::command
/// Engine::command is not belong to the lifecyles. Engine::command is used by editor to modify the game world.
/// You usually do not need to implement this, just call the EngineImpl::command.
/// # Examples
/// ## For empty project
/// You do not need to impl Engine, just return an EngineImpl:
/// ```rust
/// #[no_mangle]
/// pub fn create() -> Box<dyn Engine> {
///     Box::new(EngineImpl::new())
/// }
/// ```
/// ## Write EngineWrapper
/// You can write an EngineWrapper before writing your components, uniques or systems:
/// ```rust
/// #[no_mangle]
/// pub fn create() -> Box<dyn Engine> {
///     Box::new(EngineWrapper { inner: EngineImpl::new() })
/// }
///
/// struct EngineWrapper {
///     inner: EngineImpl,
/// }
///
/// impl Engine for EngineWrapper {
///     fn init(&mut self, info: InitInfo) {
///         self.inner.init(info);
///     }
///
///     fn frame(&mut self, info: &FrameInfo) {
///         self.inner.frame(info);
///         match info.stage {
///             FrameStage::Maintain => (),
///             FrameStage::Update => (),
///             FrameStage::Finish => (),
///         }
///     }
///
///     fn draw(&mut self, info: DrawInfo) -> Box<dyn GpuFuture> {
///         self.inner.draw(info)
///     }
///
///     fn command(&mut self, cmd: Command) {
///         self.inner.command(cmd);
///     }
/// }
/// ```
/// ## Write your component
/// ```
/// #[derive(Component, Edit, Default)]
/// struct Player;
///
/// impl Engine for EngineWrapper {
///     fn init(&mut self, info: InitInfo) {
///         self.inner.init(info);
///         self.inner.register_component::<Player>();
///     }
///     ...
/// }
/// ```
/// ## Write your system and run it every frame
/// ```rust
/// fn player_control_system(player: ViewMut<Player>, input: UniqueView<Input>) {
///     ...
/// }
///
/// impl Engine for EngineWrapper {
///     ...
///     fn frame(&mut self, info: &FrameInfo) {
///         self.inner.frame(info);
///         match info.stage {
///             FrameStage::Maintain => (),
///             FrameStage::Update => {
///                 self.inner.world.run(player_control_system);
///             },
///             FrameStage::Finish => (),
///         }
///     }
///     ...
/// }
/// ```
pub trait Engine {
    fn init(&mut self, info: InitInfo);
    fn frame(&mut self, info: &FrameInfo);
    fn draw(&mut self, info: DrawInfo) -> Box<dyn GpuFuture>;
    fn command(&mut self, cmd: Command);
}

/// The InitInfo contains some initialization data, and is passed to Engine:init.
pub struct InitInfo<'a> {
    pub platform: Platform,
    pub context: &'a VulkanoContext,
    pub scene: Option<PathBuf>,
}

/// The FrameInfo contains some data about current frame, and is passed to Engine:frame every frame.
/// You can get current frame stage from FrameInfo::stage.
pub struct FrameInfo<'a> {
    pub stage: FrameStage,
    pub ctx: &'a egui::Context,
}

/// Every frame has three stages: FrameStage::Maintain, FrameStage::Update and FrameStage::Finish.
pub enum FrameStage {
    /// This is the first stage in a frame. This stage is also called in editor even when game is not running,
    /// so that the logic you write here will also affect editor every frame.
    /// For example the physics maintain system should run in FrameStage::Maintain
    /// so that the physics world is updated immediately when you add or remove physics components in editor.
    Maintain,
    /// This is the second stage in a frame. This stage is skipped in editor when game is not running,
    /// so that you can implement your game logic which should not affect editor.
    /// For example the physics update system should run in FrameStage::Update
    /// so that the physics bodies is not pulled by gravity when the game is not running in editor.
    Update,
    /// This is the third stage in a frame. This stage is also called in editor even when game is not running.
    /// For example the render system can run in FrameStage::Finish to collect vertex data
    /// after all entitiesa and compnents are updated.
    Finish,
}

/// The DrawInfo contains some drawing data, and is passed to Engine:draw every frame.
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
/// and is contained in DrawInfo, which is passed to Engine:draw every frame.
pub struct EditorInfo<'a> {
    pub camera: &'a EditorCamera
}

/// Camera info for editor window.
pub struct EditorCamera {
    pub position: Vec3,
    pub height: f32,
}

/// Command is sent by editor through Engine::command method to modify the game world.
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
