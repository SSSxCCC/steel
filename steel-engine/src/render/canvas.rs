use super::{
    image::ImageAssets,
    model::ModelAssets,
    pipeline::{
        rasterization::RasterizationPipeline,
        raytracing::{material::Material, RayTracingPipeline},
    },
    texture::TextureAssets,
    FrameRenderInfo, RenderContext, RenderManager,
};
use crate::{asset::AssetManager, camera::CameraInfo};
use glam::{Affine3A, UVec2, Vec3, Vec4};
use shipyard::{EntityId, Unique, UniqueView, UniqueViewMut};
use std::sync::Arc;
use steel_common::{asset::AssetId, platform::Platform};
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, CopyImageToBufferInfo,
        PrimaryAutoCommandBuffer, PrimaryCommandBufferAbstract,
    },
    format::Format,
    image::{view::ImageView, Image, ImageCreateInfo, ImageUsage},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    sync::GpuFuture,
};

/// Canvas contains current frame's drawing data, which will be converted to vertex data, and send to gpu to draw.
/// You can use this unique to draw points, lines, triangles, rectangles, cicles, etc. on the screen.
/// All the drawing data requires an [EntityId] for screen object picking.
#[derive(Unique, Default)]
pub struct Canvas {
    /// 1 vertex: (position, color, eid)
    pub(crate) points: Vec<(Vec3, Vec4, EntityId)>,
    /// 2 vertex: (position, color, eid)
    pub(crate) lines: Vec<[(Vec3, Vec4, EntityId); 2]>,
    /// 3 vertex: (position, color, eid)
    pub(crate) triangles: Vec<[(Vec3, Vec4, EntityId); 3]>,
    /// (model matrix, color, eid)
    pub(crate) rectangles: Vec<(Affine3A, Vec4, EntityId)>,
    /// (model matrix, color, eid)
    pub(crate) cicles: Vec<(Affine3A, Vec4, EntityId)>,
    /// (texture asset, model matrix, color, eid)
    pub(crate) textures: Vec<(AssetId, Affine3A, Vec4, EntityId)>,
    /// (model matrix, color, eid)
    pub(crate) cuboids: Vec<(Affine3A, Vec4, EntityId)>,
    /// (model matrix, color, material, eid)
    pub(crate) spheres: Vec<(Affine3A, Vec4, Material, EntityId)>,
    /// (model asset, texture asset, model matrix, color, eid)
    pub(crate) models: Vec<(AssetId, AssetId, Affine3A, Vec4, EntityId)>,
}

impl Canvas {
    /// Draw a point with position p, color, and [EntityId].
    pub fn point(&mut self, p: Vec3, color: Vec4, eid: EntityId) {
        self.points.push((p, color, eid));
    }

    /// Draw a line from p1 to p2 with color and [EntityId].
    pub fn line(&mut self, p1: Vec3, p2: Vec3, color: Vec4, eid: EntityId) {
        self.lines.push([(p1, color, eid), (p2, color, eid)]);
    }

    /// Draw a triangle with vertices p1, p2, p3, color, and [EntityId].
    pub fn triangle(&mut self, p1: Vec3, p2: Vec3, p3: Vec3, color: Vec4, eid: EntityId) {
        self.triangles
            .push([(p1, color, eid), (p2, color, eid), (p3, color, eid)]);
    }

    /// Draw a rectangle with model matrix, color, and [EntityId].
    pub fn rectangle(&mut self, model: Affine3A, color: Vec4, eid: EntityId) {
        self.rectangles.push((model, color, eid));
    }

    /// Draw a circle with model matrix, color, and [EntityId].
    /// are the center and radius of the circle. The eid is used for screen object picking.
    pub fn circle(&mut self, model: Affine3A, color: Vec4, eid: EntityId) {
        self.cicles.push((model, color, eid));
    }

    /// Draw a texture with texture asset, model matrix, color, and [EntityId].
    pub fn texture(&mut self, asset: AssetId, model: Affine3A, color: Vec4, eid: EntityId) {
        self.textures.push((asset, model, color, eid));
    }

    /// Draw a cuboid with model matrix, color, and [EntityId].
    pub fn cuboid(&mut self, model: Affine3A, color: Vec4, eid: EntityId) {
        self.cuboids.push((model, color, eid));
    }

    /// Draw a sphere with model matrix, color, material, and [EntityId].
    pub fn sphere(&mut self, model: Affine3A, color: Vec4, material: Material, eid: EntityId) {
        self.spheres.push((model, color, material, eid));
    }

    /// Draw a model with model asset, texture asset, model matrix, color, and [EntityId].
    pub fn model(
        &mut self,
        model_asset: AssetId,
        texture_asset: AssetId,
        model: Affine3A,
        color: Vec4,
        eid: EntityId,
    ) {
        self.models
            .push((model_asset, texture_asset, model, color, eid));
    }

    /// Clear all drawing data.
    pub fn clear(&mut self) {
        self.points.clear();
        self.lines.clear();
        self.triangles.clear();
        self.rectangles.clear();
        self.cicles.clear();
        self.textures.clear();
        self.cuboids.clear();
        self.spheres.clear();
        self.models.clear();
    }
}

/// Clear the canvas.
pub fn canvas_clear_system(mut canvas: UniqueViewMut<Canvas>) {
    canvas.clear();
}

/// CanvasRenderContext stores many render objects that exist between frames.
pub(crate) struct CanvasRenderContext {
    pub eid_images: [Vec<Arc<ImageView>>; 2],
    pub rasterization: RasterizationPipeline,
    pub ray_tracing: Option<RayTracingPipeline>,
}

impl CanvasRenderContext {
    pub fn new(
        context: &RenderContext,
        info: &FrameRenderInfo,
        ray_tracing_supported: bool,
    ) -> Self {
        CanvasRenderContext {
            eid_images: [Vec::new(), Vec::new()],
            rasterization: RasterizationPipeline::new(context, info),
            ray_tracing: if ray_tracing_supported {
                Some(RayTracingPipeline::new(context))
            } else {
                None
            },
        }
    }

    pub fn update(&mut self, context: &RenderContext, info: &FrameRenderInfo) {
        self.update_eid_images(context, info);
        self.rasterization.update(context, info); // TODO: not update unused pipeline
    }

    fn update_eid_images(&mut self, context: &RenderContext, info: &FrameRenderInfo) {
        let eid_images = &mut self.eid_images[info.window_index];
        if eid_images.len() >= info.image_count {
            // TODO: use == instead of >= when we can get right image count
            let [width, height, _] = eid_images[0].image().extent();
            if info.window_size.x == width && info.window_size.y == height {
                return;
            }
        }
        log::trace!("Create eid images, image_count={}", info.image_count);
        *eid_images = (0..info.image_count)
            .map(|_| {
                let image = Image::new(
                    context.memory_allocator.clone(),
                    ImageCreateInfo {
                        format: Format::R32G32_UINT,
                        extent: [info.window_size.x, info.window_size.y, 1],
                        usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_SRC,
                        ..Default::default()
                    },
                    AllocationCreateInfo::default(),
                )
                .unwrap();
                ImageView::new_default(image).unwrap()
            })
            .collect();
    }
}

/// Send all canvas drawing data to the gpu to draw.
pub fn canvas_render_system(
    info: UniqueView<FrameRenderInfo>,
    camera: UniqueView<CameraInfo>,
    canvas: UniqueView<Canvas>,
    mut render_manager: UniqueViewMut<RenderManager>,
    mut model_assets: UniqueViewMut<ModelAssets>,
    mut texture_assets: UniqueViewMut<TextureAssets>,
    mut image_assets: UniqueViewMut<ImageAssets>,
    mut asset_manager: UniqueViewMut<AssetManager>,
    platform: UniqueView<Platform>,
) -> (Box<dyn GpuFuture>, Arc<PrimaryAutoCommandBuffer>) {
    let render_manager = render_manager.as_mut();
    render_manager.update(&info, render_manager.ray_tracing_supported());
    let context = &render_manager.context;
    let canvas_context = render_manager.canvas_context.as_mut().unwrap();
    let eid_image = canvas_context.eid_images[info.window_index][info.image_index].clone();
    if render_manager.ray_tracing {
        canvas_context.ray_tracing.as_mut().unwrap().draw(
            context,
            &info,
            &camera,
            &render_manager.ray_tracing_settings,
            &canvas,
            eid_image,
        )
    } else {
        (
            vulkano::sync::now(context.device.clone()).boxed(),
            canvas_context.rasterization.draw(
                context,
                &info,
                &camera,
                &render_manager.rasterization_settings,
                &canvas,
                &mut model_assets,
                &mut texture_assets,
                &mut image_assets,
                &mut asset_manager,
                &platform,
                eid_image,
            ),
        )
    }
}

/// Parameters for [get_entity_at_screen_system].
#[derive(Unique)]
pub(crate) struct GetEntityAtScreenParam {
    pub window_index: usize,
    pub screen_position: UVec2,
}

/// Screen object picking system.
pub(crate) fn get_entity_at_screen_system(
    mut render_manager: UniqueViewMut<RenderManager>,
    param: UniqueView<GetEntityAtScreenParam>,
) -> EntityId {
    let render_manager = render_manager.as_mut();
    if let Some(canvas_contex) = render_manager.canvas_context.as_mut() {
        let image_index = render_manager.image_index[param.window_index];
        if canvas_contex.eid_images[param.window_index].len() > image_index {
            let eid_image = &canvas_contex.eid_images[param.window_index][image_index];
            let [width, height, _] = eid_image.image().extent();
            let buffer = Buffer::from_iter(
                render_manager.context.memory_allocator.clone(),
                BufferCreateInfo {
                    usage: BufferUsage::TRANSFER_DST,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_HOST
                        | MemoryTypeFilter::HOST_RANDOM_ACCESS,
                    ..Default::default()
                },
                (0..width * height * 2).map(|_| 0u32),
            )
            .unwrap();
            let mut builder = AutoCommandBufferBuilder::primary(
                &render_manager.context.command_buffer_allocator,
                render_manager.context.graphics_queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap();
            builder
                .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(
                    eid_image.image().clone(),
                    buffer.clone(),
                ))
                .unwrap();
            builder
                .build()
                .unwrap()
                // no need to execute after previous drawing future because they are excuting on the same vk queue
                .execute(render_manager.context.graphics_queue.clone())
                .unwrap()
                .then_signal_fence_and_flush()
                .unwrap()
                .wait(None)
                .unwrap();
            let index = ((param.screen_position.x + param.screen_position.y * width) * 2) as usize;
            let buffer_read = buffer.read().unwrap();
            if index + 1 < buffer_read.len() {
                let eid_array = [buffer_read[index], buffer_read[index + 1]];
                return u32_array_to_eid(eid_array);
            }
        }
    }
    EntityId::dead()
}

/// Helper function to convert [EntityId] to [[u32; 2]].
pub fn eid_to_u32_array(eid: EntityId) -> [u32; 2] {
    let eid = eid.inner();
    [eid as u32, (eid >> 32) as u32]
}

/// Helper function to convert [[u32; 2]] back into [EntityId].
pub fn u32_array_to_eid(arr: [u32; 2]) -> EntityId {
    let eid = ((arr[1] as u64) << 32) | (arr[0] as u64);
    EntityId::from_inner(eid).unwrap_or_default()
}
