pub mod canvas;
pub mod renderer2d;

use std::sync::Arc;
use glam::{UVec2, Vec4};
use shipyard::Unique;
use steel_common::{data::{Data, Limit, Value}, engine::{DrawInfo, WindowIndex}, ext::VulkanoWindowRendererExt};
use vulkano::{command_buffer::allocator::StandardCommandBufferAllocator, device::{Device, Queue}, format::Format, image::ImageViewAbstract, memory::allocator::StandardMemoryAllocator};
use vulkano_util::context::VulkanoContext;
use crate::edit::Edit;
use self::canvas::CanvasRenderContext;

/// FrameRenderInfo is a temporary unique that carries render data of this frame.
/// FrameRenderInfo is added to World at the start of Engine::draw, and is removed from World at the end of Engine::draw.
#[derive(Unique)]
pub struct FrameRenderInfo {
    /// WindowIndex::GAME or WindowIndex::SCENE.
    pub window_index: usize,
    /// The window size, this is the pixel size of the image we will draw.
    pub window_size: UVec2,
    /// Number of images for multiple buffering.
    pub image_count: usize,
    /// The index of current image.
    pub image_index: usize,
    /// The image we will draw.
    pub image: Arc<dyn ImageViewAbstract>,
    /// The image format.
    pub format: Format,
    // We can not store before_future here because VulkanoWindowRenderer::acquire can not return Box<dyn GpuFuture + Send + Sync>.
    // TODO: store before_future here and return after_future in render::canvas::canvas_render_system.
}

impl FrameRenderInfo {
    pub fn from(info: &DrawInfo) -> Self {
        FrameRenderInfo {
            window_index: if info.editor_info.is_some() { WindowIndex::SCENE } else { WindowIndex::GAME },
            window_size: info.window_size,
            image_count: std::cmp::max(info.renderer.image_count(), info.renderer.image_index() as usize + 1), // TODO: only use info.renderer.image_count() when it returns right value
            image_index: info.renderer.image_index() as usize,
            image: info.image.clone(),
            format: info.renderer.swapchain_format(),
        }
    }
}

/// RenderContext stores many render objects that exist in the whole lifetime of application.
pub struct RenderContext {
    pub device: Arc<Device>,
    pub graphics_queue: Arc<Queue>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    pub command_buffer_allocator: StandardCommandBufferAllocator,
}

/// RenderManager contains many render context objects and render parameters.
#[derive(Unique, Edit)]
pub struct RenderManager {
    pub context: RenderContext,
    pub canvas_context: Option<CanvasRenderContext>,

    /// The image index at WindowIndex::GAME and WindowIndex::SCENE are for game window and scene window.
    pub image_index: [usize; 2],
    /// The color to clear the image before drawing.
    #[edit(limit = "Limit::Vec4Color")]
    pub clear_color: Vec4,
}

impl RenderManager {
    /// Create a new RenderManager based on VulkanoContext.
    pub fn new(context: &VulkanoContext) -> Self {
        Self {
            context: RenderContext {
                device: context.device().clone(),
                graphics_queue: context.graphics_queue().clone(),
                memory_allocator: context.memory_allocator().clone(),
                command_buffer_allocator: StandardCommandBufferAllocator::new(context.device().clone(), Default::default()),
            },
            canvas_context: None,
            image_index: [0, 0],
            clear_color: Vec4::ZERO
        }
    }

    /// Update RenderManager from FrameRenderInfo.
    pub fn update(&mut self, info: &FrameRenderInfo) {
        self.image_index[info.window_index] = info.image_index;
        self.canvas_context.get_or_insert_with(|| CanvasRenderContext::new(&self.context, info)).update(&self.context, info);
    }
}
