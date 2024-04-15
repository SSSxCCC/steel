use std::{path::PathBuf, sync::Arc};
use glam::{UVec2, Vec3};
use vulkano::{sync::GpuFuture, image::ImageViewAbstract};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};
use shipyard::EntityId;
use winit_input_helper::WinitInputHelper;
use crate::{data::WorldData, platform::Platform};

pub trait Engine {
    fn init(&mut self, info: InitInfo);
    fn maintain(&mut self, info: &UpdateInfo);
    fn update(&mut self, info: &UpdateInfo);
    fn finish(&mut self, info: &UpdateInfo);
    fn draw(&mut self, info: DrawInfo) -> Box<dyn GpuFuture>;
    fn command(&mut self, cmd: Command);
}

pub struct InitInfo<'a> {
    pub platform: Platform,
    pub context: &'a VulkanoContext,
    pub scene: Option<PathBuf>,
}

pub struct UpdateInfo<'a> {
    pub input: &'a WinitInputHelper,
    pub ctx: &'a egui::Context,
}

pub struct DrawInfo<'a> {
    pub before_future: Box<dyn GpuFuture>,
    pub context: &'a VulkanoContext,
    pub renderer: &'a VulkanoWindowRenderer,
    /// the image we will draw
    pub image: Arc<dyn ImageViewAbstract>,
    pub window_size: UVec2,
    /// if editor_info is some, we are drawing for the editor window
    pub editor_info: Option<EditorInfo<'a>>,
}

pub struct EditorInfo<'a> {
    pub camera: &'a EditorCamera
}

/// Camera info for editor window
pub struct EditorCamera {
    pub position: Vec3,
    pub height: f32,
}

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
}
