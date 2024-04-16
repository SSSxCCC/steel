pub mod canvas;
pub mod renderer2d;

use std::sync::Arc;
use glam::{UVec2, Vec4};
use shipyard::Unique;
use steel_common::{data::{Data, Limit, Value}, engine::DrawInfo, ext::VulkanoWindowRendererExt};
use vulkano::{command_buffer::allocator::StandardCommandBufferAllocator, device::{Device, Queue}, format::Format, image::ImageViewAbstract, memory::allocator::StandardMemoryAllocator};
use vulkano_util::context::VulkanoContext;
use crate::edit::Edit;
use self::canvas::CanvasRenderContext;

/// Helper struct to define window index constans: WindowIndex::GAME and WindowIndex::SCENE
pub struct WindowIndex;

impl WindowIndex {
    pub const GAME: usize = 0;
    pub const SCENE: usize = 1;
}

/// FrameRenderInfo is a temporary unique that carries render data of this frame.
/// FrameRenderInfo is added to World at the start of Engine::draw, and is removed from World at the end of Engine::draw
#[derive(Unique)]
pub struct FrameRenderInfo {
    /// WindowIndex::GAME or WindowIndex::SCENE
    pub window_index: usize,
    pub window_size: UVec2,
    pub image_count: usize,
    pub image_index: usize,
    pub image: Arc<dyn ImageViewAbstract>, // the image we will draw
    pub format: Format,
}

impl FrameRenderInfo {
    pub fn from(info: &DrawInfo) -> Self {
        FrameRenderInfo {
            window_index: if info.editor_info.is_some() { WindowIndex::SCENE } else { WindowIndex::GAME },
            window_size: info.window_size,
            image_count: std::cmp::max(info.renderer.image_count(), info.renderer.image_index() as usize + 1), // TODO: only use info.renderer.image_count() when it returns right value
            image_index: info.renderer.image_index() as usize,
            image: info.image.clone(),
            format: info.renderer.swapchain_format()
        }
    }
}

/// RenderContext stores many render objects that exist in the whole lifetime of application
pub struct RenderContext {
    pub device: Arc<Device>,
    pub graphics_queue: Arc<Queue>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    pub command_buffer_allocator: StandardCommandBufferAllocator,
}

/// RenderManager contains many render context objects and render parameters
#[derive(Unique)]
pub struct RenderManager {
    pub context: RenderContext,
    pub canvas_context: Option<CanvasRenderContext>,

    pub clear_color: Vec4,
}

impl RenderManager {
    pub fn new(context: &VulkanoContext) -> Self {
        Self {
            context: RenderContext {
                device: context.device().clone(),
                graphics_queue: context.graphics_queue().clone(),
                memory_allocator: context.memory_allocator().clone(),
                command_buffer_allocator: StandardCommandBufferAllocator::new(context.device().clone(), Default::default()),
            },
            canvas_context: None,
            clear_color: Vec4::ZERO
        }
    }

    pub fn update(&mut self, info: &FrameRenderInfo) {
        self.canvas_context.get_or_insert_with(|| CanvasRenderContext::new(&self.context, info)).update(&self.context, info);
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
