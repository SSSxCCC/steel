pub mod canvas;
pub mod image;
pub mod model;
pub mod pipeline;
pub mod renderer;
pub mod renderer2d;
pub mod texture;

mod mesh;

use self::canvas::CanvasRenderContext;
use crate::edit::Edit;
use glam::{UVec2, Vec4};
use pipeline::raytracing::util::ash::AshContext;
use shipyard::Unique;
use std::sync::Arc;
use steel_common::{
    app::{DrawInfo, WindowIndex},
    data::{Data, Limit, Value},
    ext::VulkanoWindowRendererExt,
};
use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator,
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    device::{Device, Queue},
    format::Format,
    image::view::ImageView,
    memory::allocator::StandardMemoryAllocator,
};
use vulkano_util::context::VulkanoContext;

/// FrameRenderInfo is a temporary unique that carries render data of this frame.
/// FrameRenderInfo is added to World at the start of App::draw, and is removed from World at the end of App::draw.
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
    pub image: Arc<ImageView>,
    /// The image format.
    pub format: Format,
    // We can not store before_future here because VulkanoWindowRenderer::acquire can not return Box<dyn GpuFuture + Send + Sync>.
    // TODO: store before_future here and return after_future in render::canvas::canvas_render_system.
}

impl FrameRenderInfo {
    pub fn from(info: &DrawInfo) -> Self {
        FrameRenderInfo {
            window_index: if info.editor_info.is_some() {
                WindowIndex::SCENE
            } else {
                WindowIndex::GAME
            },
            window_size: info.window_size,
            image_count: std::cmp::max(
                info.renderer.image_count(),
                info.renderer.image_index() as usize + 1,
            ), // TODO: only use info.renderer.image_count() when it returns right value
            image_index: info.renderer.image_index() as usize,
            image: info.image.clone(),
            format: info.renderer.swapchain_format(),
        }
    }
}

/// RenderContext stores many render objects that exist in the whole lifetime of application.
pub struct RenderContext {
    // The fields in this struct are public for convenience, users can only get a immutable reference
    // of this struct from RenderManager::context so that they can not mutate the fields of this struct.
    pub device: Arc<Device>,
    pub graphics_queue: Arc<Queue>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    pub command_buffer_allocator: StandardCommandBufferAllocator,
    pub descriptor_set_allocator: StandardDescriptorSetAllocator,
    pub ash: AshContext,
}

/// RenderManager contains many render context objects and render parameters.
#[derive(Unique)]
pub struct RenderManager {
    context: RenderContext,
    pub(crate) canvas_context: Option<CanvasRenderContext>,

    /// The image index at [WindowIndex::GAME] and [WindowIndex::SCENE] are for game window and scene window.
    pub(crate) image_index: [usize; 2],

    /// If current device supports ray tracing.
    ray_tracing_supported: bool,
    /// True means rendering with ray tracing pipeline, false means rendering with rasterization pipeline.
    ray_tracing: bool,

    /// The color to clear the image before drawing.
    pub clear_color: Vec4,
}

impl RenderManager {
    /// Create a new RenderManager based on VulkanoContext.
    pub(crate) fn new(context: &VulkanoContext, ray_tracing_supported: bool) -> Self {
        Self {
            context: RenderContext {
                device: context.device().clone(),
                graphics_queue: context.graphics_queue().clone(),
                memory_allocator: context.memory_allocator().clone(),
                command_buffer_allocator: StandardCommandBufferAllocator::new(
                    context.device().clone(),
                    Default::default(),
                ),
                descriptor_set_allocator: StandardDescriptorSetAllocator::new(
                    context.device().clone(),
                    Default::default(),
                ),
                ash: AshContext::new(context),
            },
            canvas_context: None,
            image_index: [0, 0],
            ray_tracing_supported,
            ray_tracing: false,
            clear_color: Vec4::ZERO,
        }
    }

    /// Update RenderManager from FrameRenderInfo.
    pub(crate) fn update(&mut self, info: &FrameRenderInfo, ray_tracing_supported: bool) {
        self.image_index[info.window_index] = info.image_index;
        self.canvas_context
            .get_or_insert_with(|| {
                CanvasRenderContext::new(&self.context, info, ray_tracing_supported)
            })
            .update(&self.context, info);
    }

    /// The render context.
    pub fn context(&self) -> &RenderContext {
        &self.context
    }

    /// If current device supports ray tracing.
    pub fn ray_tracing_supported(&self) -> bool {
        self.ray_tracing_supported
    }

    /// True means rendering with ray tracing pipeline, false means rendering with rasterization pipeline.
    pub fn ray_tracing(&self) -> bool {
        self.ray_tracing
    }

    /// Turn ray tracing on or off. If [Self::ray_tracing_supported] is false, this dose nothing.
    pub fn set_ray_tracing(&mut self, on: bool) {
        if self.ray_tracing_supported {
            self.ray_tracing = on;
        }
    }
}

impl Edit for RenderManager {
    fn name() -> &'static str {
        "RenderManager"
    }

    fn get_data(&self) -> Data {
        let mut data = Data::new();
        if self.ray_tracing_supported {
            data.add_value("ray_tracing", Value::Bool(self.ray_tracing));
        } else {
            data.add_value_with_limit(
                "ray_tracing",
                Value::Bool(self.ray_tracing),
                Limit::ReadOnly,
            );
        }

        if self.ray_tracing {
            // ray tracing config
        } else {
            // rasterization config
            data.add_value_with_limit(
                "clear_color",
                Value::Vec4(self.clear_color),
                Limit::Vec4Color,
            )
        }

        data
    }

    fn set_data(&mut self, data: &Data) {
        if self.ray_tracing_supported {
            if let Some(Value::Bool(v)) = data.get("ray_tracing") {
                self.ray_tracing = *v;
            }
        }

        // ray tracing config

        // rasterization config
        if let Some(Value::Vec4(v)) = data.get("clear_color") {
            self.clear_color = *v;
        }
    }
}
