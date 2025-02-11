use super::{
    image::ImageAssets,
    mesh::{self, Mesh, MeshAssets, MeshData},
    model::ModelAssets,
    pipeline::{
        rasterization::RasterizationPipeline,
        raytracing::{material::Material, RayTracingPipeline},
    },
    texture::{Texture, TextureAssets, TextureData},
    FrameRenderInfo, RenderContext, RenderSettings,
};
use crate::{asset::AssetManager, camera::CameraInfo, hierarchy::Parent, transform::Transform};
use glam::{Affine3A, IVec2, Vec3, Vec4};
use shipyard::{EntityId, Get, IntoIter, IntoWithId, Unique, UniqueView, UniqueViewMut, View};
use std::{collections::HashMap, sync::Arc};
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
/// You can use this unique to draw points, lines, meshs, circles, spheres, etc. on the screen.
/// All the drawing data requires an [EntityId] for screen object picking.
#[derive(Unique, Default)]
pub struct Canvas {
    /// 1 vertex: (position, color, eid)
    pub(crate) points: Vec<(Vec3, Vec4, EntityId)>,
    /// 2 vertex: (position, color, eid)
    pub(crate) lines: Vec<[(Vec3, Vec4, EntityId); 2]>,
    /// (mesh, color, texture, material, model matrix, eid)
    pub(crate) meshs: Vec<(
        Arc<MeshData>,
        Vec4,
        Option<TextureData>,
        Material,
        Affine3A,
        EntityId,
    )>,
    /// (color, texture, material, model matrix, eid)
    pub(crate) circles: Vec<(Vec4, Option<TextureData>, Material, Affine3A, EntityId)>,
    /// (color, texture, material, model matrix, eid)
    pub(crate) spheres: Vec<(Vec4, Option<TextureData>, Material, Affine3A, EntityId)>,
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

    /// Draw a mesh with mesh data, color, texture, material, model matrix, and [EntityId].
    pub fn mesh(
        &mut self,
        mesh: Arc<MeshData>,
        color: Vec4,
        texture: Option<TextureData>,
        material: Material,
        model: Affine3A,
        eid: EntityId,
    ) {
        self.meshs
            .push((mesh, color, texture, material, model, eid));
    }

    /// Draw a circle with color, texture, material, model matrix, and [EntityId].
    /// Model matrix are the center and radius of the circle. The circle is drawn on the xy plane.
    /// Rasterization rendering pipeline uses [mesh::RECTANGLE] and custom vertex & fragment shaders to render a perfect circle.
    /// Raytracing rendering pipeline uses an intersection shader to render a perfect circle.
    pub fn circle(
        &mut self,
        color: Vec4,
        texture: Option<TextureData>,
        material: Material,
        model: Affine3A,
        eid: EntityId,
    ) {
        self.circles.push((color, texture, material, model, eid));
    }

    /// Draw a sphere with color, texture, material, model matrix, and [EntityId].
    /// Rasterization rendering pipeline uses [mesh::SPHERE] to render a sphere.
    /// Raytracing rendering pipeline uses an intersection shader to render a perfect sphere.
    pub fn sphere(
        &mut self,
        color: Vec4,
        texture: Option<TextureData>,
        material: Material,
        model: Affine3A,
        eid: EntityId,
    ) {
        self.spheres.push((color, texture, material, model, eid));
    }

    /// Clear all drawing data.
    pub fn clear(&mut self) {
        self.points.clear();
        self.lines.clear();
        self.meshs.clear();
        self.circles.clear();
        self.spheres.clear();
    }
}

/// Clear the canvas.
pub fn canvas_clear_system(mut canvas: UniqueViewMut<Canvas>) {
    canvas.clear();
}

/// Collect render data from [Mesh], [Texture], and [Material] components to [Canvas].
pub fn canvas_update_system(
    meshs: View<Mesh>,
    textures: View<Texture>,
    materials: View<Material>,
    transforms: View<Transform>,
    parents: View<Parent>,
    mut canvas: UniqueViewMut<Canvas>,
    context: UniqueView<RenderContext>,
    platform: UniqueView<Platform>,
    (mut mesh_assets, mut model_assets, mut texture_assets, mut image_assets, mut asset_manager): (
        UniqueViewMut<MeshAssets>,
        UniqueViewMut<ModelAssets>,
        UniqueViewMut<TextureAssets>,
        UniqueViewMut<ImageAssets>,
        UniqueViewMut<AssetManager>,
    ),
) {
    let mut model_cache = Some(HashMap::new());
    let mut scale_cache = Some(HashMap::new());
    for (eid, mesh) in meshs.iter().with_id() {
        let scale = Transform::entity_final_scale(eid, &parents, &transforms, &mut scale_cache)
            .unwrap_or(Vec3::ONE);
        let model_without_scale = Transform::entity_final_model_without_scale(
            eid,
            &parents,
            &transforms,
            &mut model_cache,
        )
        .unwrap_or_default();
        let model = model_without_scale * Affine3A::from_scale(scale);

        let (texture_asset, color) = if let Ok(texture) = textures.get(eid) {
            (texture.asset, texture.color)
        } else {
            (AssetId::INVALID, Vec4::ONE)
        };
        let texture_data = texture_assets.get_texture(
            texture_asset,
            &mut image_assets,
            &mut asset_manager,
            &platform,
            &context,
        );

        let material = materials.get(eid).cloned().unwrap_or_default();

        match mesh {
            Mesh::Asset(model_asset) => {
                if let Some(mesh_data) = mesh_assets.get_mesh(
                    *model_asset,
                    &mut model_assets,
                    &mut asset_manager,
                    &platform,
                ) {
                    canvas.mesh(mesh_data, color, texture_data, material, model, eid);
                }
            }
            Mesh::Shape2D(shape2d) => match shape2d.shape_type() {
                parry2d::shape::ShapeType::Ball => {
                    let scale = shape2d.as_ball().unwrap().radius / 0.5
                        * std::cmp::max_by(scale.x.abs(), scale.y.abs(), |x, y| {
                            x.partial_cmp(y).unwrap()
                        });
                    let model =
                        model_without_scale * Affine3A::from_scale(Vec3::new(scale, scale, 1.0));
                    canvas.circle(color, texture_data, material, model, eid);
                }
                parry2d::shape::ShapeType::Cuboid => {
                    let shape = shape2d.as_cuboid().unwrap();
                    let scale = Vec3::new(
                        scale.x * shape.half_extents.x * 2.0,
                        scale.y * shape.half_extents.y * 2.0,
                        scale.z,
                    );
                    let model = model_without_scale * Affine3A::from_scale(scale);
                    canvas.mesh(
                        mesh::RECTANGLE.clone(),
                        color,
                        texture_data,
                        material,
                        model,
                        eid,
                    );
                }
                _ => (),
            },
            Mesh::Shape3D(shape3d) => match shape3d.shape_type() {
                parry3d::shape::ShapeType::Ball => {
                    let scale = shape3d.as_ball().unwrap().radius / 0.5
                        * [scale.x.abs(), scale.y.abs(), scale.z.abs()]
                            .into_iter()
                            .fold(f32::NEG_INFINITY, |max, val| max.max(val));
                    let model =
                        model_without_scale * Affine3A::from_scale(Vec3::new(scale, scale, scale));
                    canvas.sphere(color, texture_data, material, model, eid);
                }
                parry3d::shape::ShapeType::Cuboid => {
                    let shape = shape3d.as_cuboid().unwrap();
                    let scale = Vec3::new(
                        scale.x * shape.half_extents.x * 2.0,
                        scale.y * shape.half_extents.y * 2.0,
                        scale.z * shape.half_extents.z * 2.0,
                    );
                    let model = model_without_scale * Affine3A::from_scale(scale);
                    canvas.mesh(
                        mesh::CUBOID.clone(),
                        color,
                        texture_data,
                        material,
                        model,
                        eid,
                    );
                }
                _ => (),
            },
        }
    }

    for (eid, (texture, _)) in (&textures, !&meshs).iter().with_id() {
        if let Some(texture_data) = texture_assets.get_texture(
            texture.asset,
            &mut image_assets,
            &mut asset_manager,
            &platform,
            &context,
        ) {
            let scale = Transform::entity_final_scale(eid, &parents, &transforms, &mut scale_cache)
                .unwrap_or(Vec3::ONE);
            let model_without_scale = Transform::entity_final_model_without_scale(
                eid,
                &parents,
                &transforms,
                &mut model_cache,
            )
            .unwrap_or_default();
            let model = model_without_scale
                * Affine3A::from_scale(scale)
                * Affine3A::from_scale(Vec3::new(
                    texture_data.image_view.image().extent()[0] as f32 / 100.0,
                    texture_data.image_view.image().extent()[1] as f32 / 100.0,
                    1.0,
                ));

            let material = materials.get(eid).cloned().unwrap_or_default();

            canvas.mesh(
                mesh::RECTANGLE.clone(),
                texture.color,
                Some(texture_data),
                material,
                model,
                eid,
            );
        }
    }
}

/// CanvasRenderContext stores many render objects that exist between frames.
#[derive(Unique)]
pub(crate) struct CanvasRenderContext {
    pub eid_images: [Vec<Arc<ImageView>>; 2],
    /// Rasterization pipeline will be created in the first [crate::app::App::draw].
    pub rasterization: Option<RasterizationPipeline>,
    /// Ray tracing pipeline is None if [RenderContext::ray_tracing_supported()] is false.
    pub ray_tracing: Option<RayTracingPipeline>,
}

impl CanvasRenderContext {
    pub fn new(context: &RenderContext) -> Self {
        CanvasRenderContext {
            eid_images: [Vec::new(), Vec::new()],
            rasterization: None,
            ray_tracing: if context.ray_tracing_supported() {
                Some(RayTracingPipeline::new(context))
            } else {
                None
            },
        }
    }

    pub fn update(&mut self, context: &RenderContext, info: &FrameRenderInfo) {
        self.update_eid_images(context, info);
        self.rasterization
            .get_or_insert_with(|| RasterizationPipeline::new(context, info))
            .update(context, info); // TODO: not update unused pipeline
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
                        usage: ImageUsage::COLOR_ATTACHMENT
                            | ImageUsage::STORAGE
                            | ImageUsage::TRANSFER_SRC,
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
pub(crate) fn canvas_render_system(
    info: UniqueView<FrameRenderInfo>,
    camera: UniqueView<CameraInfo>,
    canvas: UniqueView<Canvas>,
    mut render_settings: UniqueViewMut<RenderSettings>,
    mut context: UniqueViewMut<RenderContext>,
    mut canvas_context: UniqueViewMut<CanvasRenderContext>,
) -> (Box<dyn GpuFuture>, Arc<PrimaryAutoCommandBuffer>) {
    let render_settings = render_settings.as_mut();
    context.image_index[info.window_index] = info.image_index;
    canvas_context.update(&context, &info);
    let eid_image = canvas_context.eid_images[info.window_index][info.image_index].clone();
    if context.ray_tracing_supported() && render_settings.ray_tracing {
        canvas_context.ray_tracing.as_mut().unwrap().draw(
            &context,
            &info,
            &camera,
            &render_settings.ray_tracing_settings,
            &canvas,
            eid_image,
        )
    } else {
        (
            vulkano::sync::now(context.device.clone()).boxed(),
            canvas_context.rasterization.as_mut().unwrap().draw(
                &context,
                &info,
                &camera,
                &render_settings.rasterization_settings,
                &canvas,
                eid_image,
            ),
        )
    }
}

/// Parameters for [get_entity_at_screen_system].
#[derive(Unique)]
pub(crate) struct GetEntityAtScreenParam {
    pub window_index: usize,
    pub screen_position: IVec2,
}

/// Screen object picking system.
pub(crate) fn get_entity_at_screen_system(
    context: UniqueView<RenderContext>,
    canvas_context: UniqueView<CanvasRenderContext>,
    param: UniqueView<GetEntityAtScreenParam>,
) -> EntityId {
    let image_index = context.image_index[param.window_index];
    if canvas_context.eid_images[param.window_index].len() > image_index {
        let eid_image = &canvas_context.eid_images[param.window_index][image_index];
        let [width, height, _] = eid_image.image().extent().map(|i| i as i32);
        if param.screen_position.x < 0
            || param.screen_position.x >= width
            || param.screen_position.y < 0
            || param.screen_position.y >= height
        {
            return EntityId::dead();
        }
        let buffer = Buffer::from_iter(
            context.memory_allocator.clone(),
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
            &context.command_buffer_allocator,
            context.graphics_queue.queue_family_index(),
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
            .execute(context.graphics_queue.clone())
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
