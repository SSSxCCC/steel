use std::{path::PathBuf, sync::Arc};
use glam::{Vec2, Vec3};
use vulkano::{sync::GpuFuture, image::ImageViewAbstract};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};
use shipyard::EntityId;
use crate::{data::WorldData, platform::Platform};

pub trait Engine {
    fn init(&mut self, platform: Platform, scene: Option<PathBuf>);
    fn maintain(&mut self);
    fn update(&mut self);
    fn draw(&mut self);
    fn draw_game(&mut self, info: DrawInfo) -> Box<dyn GpuFuture>;
    fn draw_editor(&mut self, info: DrawInfo, camera: &EditorCamera) -> Box<dyn GpuFuture>;
    fn command(&mut self, cmd: Command);
}

pub enum Command<'a> {
    Save(&'a mut WorldData),
    Load(&'a WorldData),
    Relaod(&'a WorldData),

    CreateEntity,
    DestroyEntity(EntityId),
    ClearEntity,

    GetComponents(&'a mut Vec<&'static str>),
    CreateComponent(EntityId, &'static str),
    DestroyComponent(EntityId, &'a String),
}

pub struct DrawInfo<'a> {
    pub before_future: Box<dyn GpuFuture>,
    pub context: &'a VulkanoContext,
    pub renderer: &'a VulkanoWindowRenderer,
    pub image: Arc<dyn ImageViewAbstract>, // the image we will draw
    pub window_size: Vec2,
}

/// Camera info for editor window
pub struct EditorCamera {
    pub position: Vec3,
    pub height: f32,
}
