pub mod canvas;
pub mod image;
pub mod mesh;
pub mod model;
pub mod pipeline;
pub mod texture;

use crate::edit::Edit;
use glam::UVec2;
use pipeline::{rasterization::RasterizationSettings, raytracing::RayTracingSettings};
use shipyard::Unique;
use std::sync::Arc;
use steel_common::{
    app::{DrawInfo, WindowIndex},
    data::{Data, Value},
};
use vulkano::{
    command_buffer::allocator::{CommandBufferAllocator, StandardCommandBufferAllocator},
    descriptor_set::allocator::{DescriptorSetAllocator, StandardDescriptorSetAllocator},
    device::{Device, Queue},
    format::Format,
    image::{
        sampler::{Sampler, SamplerCreateInfo},
        view::ImageView,
        Image, ImageCreateInfo, ImageUsage,
    },
    memory::allocator::{AllocationCreateInfo, MemoryAllocator},
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
            image_count: info.renderer.swapchain_image_views().len(),
            image_index: info.renderer.image_index() as usize,
            image: info.image.clone(),
            format: info.renderer.swapchain_format(),
        }
    }
}

/// RenderContext stores many render objects that exist in the whole lifetime of application.
#[derive(Unique)]
pub struct RenderContext {
    pub(crate) device: Arc<Device>,
    pub(crate) graphics_queue: Arc<Queue>,
    pub(crate) memory_allocator: Arc<dyn MemoryAllocator>,
    pub(crate) command_buffer_allocator: Arc<dyn CommandBufferAllocator>,
    pub(crate) descriptor_set_allocator: Arc<dyn DescriptorSetAllocator>,

    /// If current device supports ray tracing.
    ray_tracing_supported: bool,

    /// The image index at [WindowIndex::GAME] and [WindowIndex::SCENE] are for game window and scene window.
    pub(crate) image_index: [usize; 2],

    /// Avoid a vulkano bug "unsolvable resource conflict".
    /// TODO: remove this when vulkano bug is fixed.
    pub(crate) temp_image_view_and_sampler: (Arc<ImageView>, Arc<Sampler>),
}

impl RenderContext {
    /// Create a [RenderContext] from [VulkanoContext].
    pub(crate) fn new(context: &VulkanoContext, ray_tracing_supported: bool) -> Self {
        RenderContext {
            device: context.device().clone(),
            graphics_queue: context.graphics_queue().clone(),
            memory_allocator: context.memory_allocator().clone(),
            command_buffer_allocator: Arc::new(StandardCommandBufferAllocator::new(
                context.device().clone(),
                Default::default(),
            )),
            descriptor_set_allocator: Arc::new(StandardDescriptorSetAllocator::new(
                context.device().clone(),
                Default::default(),
            )),
            ray_tracing_supported,
            image_index: [0, 0],
            temp_image_view_and_sampler: (
                ImageView::new_default(
                    Image::new(
                        context.memory_allocator().clone(),
                        ImageCreateInfo {
                            format: Format::R8G8B8A8_UNORM,
                            extent: [1, 1, 1],
                            usage: ImageUsage::SAMPLED | ImageUsage::COLOR_ATTACHMENT,
                            ..Default::default()
                        },
                        AllocationCreateInfo::default(),
                    )
                    .unwrap(),
                )
                .unwrap(),
                Sampler::new(
                    context.device().clone(),
                    SamplerCreateInfo::simple_repeat_linear_no_mipmap(),
                )
                .unwrap(),
            ),
        }
    }

    /// If current device supports ray tracing.
    pub fn ray_tracing_supported(&self) -> bool {
        self.ray_tracing_supported
    }
}

/// Settings for rendering pipelines.
#[derive(Unique)]
pub struct RenderSettings {
    /// True means rendering with ray tracing pipeline, false means rendering with rasterization pipeline.
    /// If [RenderContext::ray_tracing_supported()] is false, always rendering with rasterization pipeline.
    pub ray_tracing: bool,

    // TODO: move pipeline settings to Camera component
    pub rasterization_settings: RasterizationSettings,
    pub ray_tracing_settings: RayTracingSettings,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            ray_tracing: false,
            rasterization_settings: RasterizationSettings::default(),
            ray_tracing_settings: RayTracingSettings::default(),
        }
    }
}

impl Edit for RenderSettings {
    fn name() -> &'static str {
        "RenderSettings"
    }

    fn get_data(&self, data: &mut Data) {
        data.insert("ray_tracing", Value::Bool(self.ray_tracing));
        if self.ray_tracing {
            self.ray_tracing_settings.get_data(data);
        } else {
            self.rasterization_settings.get_data(data);
        }
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Bool(v)) = data.get("ray_tracing") {
            self.ray_tracing = *v;
        }
        self.ray_tracing_settings.set_data(data);
        self.rasterization_settings.set_data(data);
    }
}
