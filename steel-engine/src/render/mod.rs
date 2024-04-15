pub mod canvas;
pub mod renderer2d;

use std::sync::Arc;
use glam::{UVec2, Vec4};
use shipyard::Unique;
use steel_common::{data::{Data, Limit, Value}, engine::DrawInfo};
use vulkano::{device::{Device, Queue}, format::Format, image::ImageViewAbstract, memory::allocator::StandardMemoryAllocator};
use vulkano_util::context::VulkanoContext;
use crate::edit::Edit;

/// FrameRenderInfo is a temporary unique that carries render data of this frame.
/// FrameRenderInfo is added to World at the start of Engine::draw, and is removed from World at the end of Engine::draw
#[derive(Unique)]
pub struct FrameRenderInfo {
    pub window_size: UVec2,
    pub image: Arc<dyn ImageViewAbstract>, // the image we will draw
    pub format: Format,
}

impl FrameRenderInfo {
    pub fn from(info: &DrawInfo) -> Self {
        FrameRenderInfo {
            window_size: info.window_size,
            image: info.image.clone(),
            format: info.renderer.swapchain_format()
        }
    }
}

/// RenderManager contains many render context objects and render parameters
#[derive(Unique)]
pub struct RenderManager {
    pub device: Arc<Device>,
    pub graphics_queue: Arc<Queue>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,

    pub clear_color: Vec4,
}

impl RenderManager {
    pub fn new(context: &VulkanoContext) -> Self {
        Self {
            device: context.device().clone(),
            graphics_queue: context.graphics_queue().clone(),
            memory_allocator: context.memory_allocator().clone(),
            clear_color: Vec4::ZERO
        }
    }
}

impl Edit for RenderManager {
    fn name() -> &'static str { "RenderManager" }

    fn get_data(&self) -> Data {
        let mut data = Data::new();
        data.add("clear_color", Value::Vec4(self.clear_color), Limit::Vec4Color);
        data
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Vec4(v)) = data.values.get("clear_color") { self.clear_color = *v }
    }
}
