mod shader;

use crate::{
    asset::AssetManager,
    camera::CameraInfo,
    render::{
        canvas::Canvas, image::ImageAssets, mesh, model::ModelAssets, texture::TextureAssets,
        FrameRenderInfo, RenderContext,
    },
};
use glam::{Affine3A, Vec3, Vec4};
use shipyard::EntityId;
use std::{collections::HashMap, iter::zip, sync::Arc};
use steel_common::{asset::AssetId, platform::Platform};
use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer,
        RenderPassBeginInfo, SubpassBeginInfo, SubpassContents,
    },
    descriptor_set::{layout::DescriptorBindingFlags, PersistentDescriptorSet, WriteDescriptorSet},
    device::Device,
    format::Format,
    image::{view::ImageView, Image, ImageCreateInfo, ImageUsage},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{
        graphics::{
            color_blend::{AttachmentBlend, ColorBlendAttachmentState, ColorBlendState},
            depth_stencil::{DepthState, DepthStencilState},
            input_assembly::{InputAssemblyState, PrimitiveTopology},
            multisample::MultisampleState,
            rasterization::{CullMode, PolygonMode, RasterizationState},
            vertex_input::{Vertex, VertexDefinition},
            viewport::{Viewport, ViewportState},
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
        PipelineShaderStageCreateInfo,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    shader::{EntryPoint, ShaderModule},
    Validated, VulkanError,
};

/// RasterizationPipeline stores many render objects that exist between frames.
pub(crate) struct RasterizationPipeline {
    /// The image vectors whose index at WindowIndex::GAME and WindowIndex::SCENE are for game window and scene window.
    depth_stencil_images: [Vec<Arc<ImageView>>; 2],
    render_pass: Arc<RenderPass>,
    pipeline_point: Arc<GraphicsPipeline>,
    pipeline_line: Arc<GraphicsPipeline>,
    pipeline_triangle: Arc<GraphicsPipeline>,
    /// Used to draw 2d shapes, like rectangle.
    pipeline_shape2d: Arc<GraphicsPipeline>,
    /// Used to draw 3d shapes, like cuboid. The only difference to
    /// [CanvasRenderContext::pipeline_shape2d] is that this has culling.
    pipeline_shape: Arc<GraphicsPipeline>,
    pipeline_circle: Arc<GraphicsPipeline>,
    pipeline_texture: Arc<GraphicsPipeline>,
    pipeline_model: Arc<GraphicsPipeline>,
}

impl RasterizationPipeline {
    pub fn new(context: &RenderContext, info: &FrameRenderInfo) -> Self {
        let render_pass = Self::create_render_pass(context, info.format);
        let (
            pipeline_point,
            pipeline_line,
            pipeline_triangle,
            pipeline_shape2d,
            pipeline_shape,
            pipeline_circle,
            pipeline_texture,
            pipeline_model,
        ) = Self::create_pipelines(context, render_pass.clone());
        RasterizationPipeline {
            depth_stencil_images: [Vec::new(), Vec::new()],
            render_pass,
            pipeline_point,
            pipeline_line,
            pipeline_triangle,
            pipeline_shape2d,
            pipeline_shape,
            pipeline_circle,
            pipeline_texture,
            pipeline_model,
        }
    }

    pub fn update(&mut self, context: &RenderContext, info: &FrameRenderInfo) {
        self.update_depth_stencil_images(context, info);
    }

    fn update_depth_stencil_images(&mut self, context: &RenderContext, info: &FrameRenderInfo) {
        let depth_stencil_images = &mut self.depth_stencil_images[info.window_index];
        if depth_stencil_images.len() >= info.image_count {
            // TODO: use == instead of >= when we can get right image count
            let [width, height, _] = depth_stencil_images[0].image().extent();
            if info.window_size.x == width && info.window_size.y == height {
                return;
            }
        }
        log::trace!(
            "Create depth stencil images, image_count={}",
            info.image_count
        );
        *depth_stencil_images = (0..info.image_count)
            .map(|_| {
                let image = Image::new(
                    context.memory_allocator.clone(),
                    ImageCreateInfo {
                        format: Format::D32_SFLOAT,
                        extent: [info.window_size.x, info.window_size.y, 1],
                        usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT,
                        ..Default::default()
                    },
                    AllocationCreateInfo::default(),
                )
                .unwrap();
                ImageView::new_default(image).unwrap()
            })
            .collect();
    }

    fn create_render_pass(context: &RenderContext, format: Format) -> Arc<RenderPass> {
        vulkano::single_pass_renderpass!(
            context.device.clone(),
            attachments: {
                color: { format: format, samples: 1, load_op: Clear, store_op: Store },
                eid: { format: Format::R32G32_UINT, samples: 1, load_op: Clear, store_op: Store },
                depth_stencil: { format: Format::D32_SFLOAT, samples: 1, load_op: Clear, store_op: DontCare },
            },
            pass: {
                color: [ color, eid ],
                depth_stencil: { depth_stencil },
            },
        ).unwrap()
    }

    fn create_pipelines(
        context: &RenderContext,
        render_pass: Arc<RenderPass>,
    ) -> (
        Arc<GraphicsPipeline>,
        Arc<GraphicsPipeline>,
        Arc<GraphicsPipeline>,
        Arc<GraphicsPipeline>,
        Arc<GraphicsPipeline>,
        Arc<GraphicsPipeline>,
        Arc<GraphicsPipeline>,
        Arc<GraphicsPipeline>,
    ) {
        let vs = Self::load_entry_point(context.device.clone(), shader::vertex::vs::load);
        let fs = Self::load_entry_point(context.device.clone(), shader::vertex::fs::load);

        let pipeline_point = Self::create_pipeline(
            context,
            render_pass.clone(),
            &shader::vertex::VertexData::per_vertex(),
            PrimitiveTopology::PointList,
            PolygonMode::Point,
            CullMode::None,
            vs.clone(),
            fs.clone(),
            |_| {},
        );

        let pipeline_line = Self::create_pipeline(
            context,
            render_pass.clone(),
            &shader::vertex::VertexData::per_vertex(),
            PrimitiveTopology::LineList,
            PolygonMode::Line,
            CullMode::None,
            vs.clone(),
            fs.clone(),
            |_| {},
        );

        let pipeline_triangle = Self::create_pipeline(
            context,
            render_pass.clone(),
            &shader::vertex::VertexData::per_vertex(),
            PrimitiveTopology::TriangleList,
            PolygonMode::Fill,
            CullMode::None,
            vs.clone(),
            fs.clone(),
            |_| {},
        );

        let pipeline_shape2d = Self::create_pipeline(
            context,
            render_pass.clone(),
            &[
                shader::shape::VertexData::per_vertex(),
                shader::shape::InstanceData::per_instance(),
            ],
            PrimitiveTopology::TriangleList,
            PolygonMode::Fill,
            CullMode::None,
            Self::load_entry_point(context.device.clone(), shader::shape::vs::load),
            Self::load_entry_point(context.device.clone(), shader::shape::fs::load),
            |_| {},
        );

        let pipeline_shape = Self::create_pipeline(
            context,
            render_pass.clone(),
            &[
                shader::shape::VertexData::per_vertex(),
                shader::shape::InstanceData::per_instance(),
            ],
            PrimitiveTopology::TriangleList,
            PolygonMode::Fill,
            CullMode::Back,
            Self::load_entry_point(context.device.clone(), shader::shape::vs::load),
            Self::load_entry_point(context.device.clone(), shader::shape::fs::load),
            |_| {},
        );

        let pipeline_circle = Self::create_pipeline(
            context,
            render_pass.clone(),
            &[
                shader::shape::VertexData::per_vertex(),
                shader::shape::InstanceData::per_instance(),
            ],
            PrimitiveTopology::TriangleList,
            PolygonMode::Fill,
            CullMode::None,
            Self::load_entry_point(context.device.clone(), shader::circle::vs::load),
            Self::load_entry_point(context.device.clone(), shader::circle::fs::load),
            |_| {},
        );

        let properties = context.device.physical_device().properties();
        let max_descriptor_count = properties
            .max_per_stage_descriptor_samplers
            .min(properties.max_per_stage_descriptor_sampled_images);

        let pipeline_texture = Self::create_pipeline(
            context,
            render_pass.clone(),
            &[
                shader::shape::VertexData::per_vertex(),
                shader::texture::InstanceData::per_instance(),
            ],
            PrimitiveTopology::TriangleList,
            PolygonMode::Fill,
            CullMode::None,
            Self::load_entry_point(context.device.clone(), shader::texture::vs::load),
            Self::load_entry_point(context.device.clone(), shader::texture::fs::load),
            |create_info| {
                let binding = create_info.set_layouts[0].bindings.get_mut(&0).unwrap();
                binding.binding_flags |= DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT;
                binding.descriptor_count = max_descriptor_count;
            },
        );

        let pipeline_model = Self::create_pipeline(
            context,
            render_pass.clone(),
            &[
                shader::model::VertexData::per_vertex(),
                shader::texture::InstanceData::per_instance(),
            ],
            PrimitiveTopology::TriangleList,
            PolygonMode::Fill,
            CullMode::Back,
            Self::load_entry_point(context.device.clone(), shader::model::vs::load),
            Self::load_entry_point(context.device.clone(), shader::model::fs::load),
            |create_info| {
                let binding = create_info.set_layouts[0].bindings.get_mut(&0).unwrap();
                binding.binding_flags |= DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT;
                binding.descriptor_count = max_descriptor_count;
            },
        );

        (
            pipeline_point,
            pipeline_line,
            pipeline_triangle,
            pipeline_shape2d,
            pipeline_shape,
            pipeline_circle,
            pipeline_texture,
            pipeline_model,
        )
    }

    fn create_pipeline(
        context: &RenderContext,
        render_pass: Arc<RenderPass>,
        vertex_definition: &impl VertexDefinition,
        topology: PrimitiveTopology,
        polygon_mode: PolygonMode,
        cull_mode: CullMode,
        vs: EntryPoint,
        fs: EntryPoint,
        pipeline_descriptor_set_layout_create_info_modify: impl FnOnce(
            &mut PipelineDescriptorSetLayoutCreateInfo,
        ),
    ) -> Arc<GraphicsPipeline> {
        let vertex_input_state = vertex_definition
            .definition(&vs.info().input_interface)
            .unwrap();
        let stages = [
            PipelineShaderStageCreateInfo::new(vs),
            PipelineShaderStageCreateInfo::new(fs),
        ];
        let mut pipeline_descriptor_set_layout_create_info =
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages);
        pipeline_descriptor_set_layout_create_info_modify(
            &mut pipeline_descriptor_set_layout_create_info,
        );
        let layout = PipelineLayout::new(
            context.device.clone(),
            pipeline_descriptor_set_layout_create_info
                .into_pipeline_layout_create_info(context.device.clone())
                .unwrap(),
        )
        .unwrap();
        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        GraphicsPipeline::new(
            context.device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState {
                    topology,
                    ..Default::default()
                }),
                rasterization_state: Some(RasterizationState {
                    polygon_mode,
                    cull_mode,
                    ..Default::default()
                }),
                multisample_state: Some(MultisampleState::default()),
                depth_stencil_state: Some(DepthStencilState {
                    depth: Some(DepthState::simple()),
                    ..Default::default()
                }),
                color_blend_state: Some(ColorBlendState {
                    attachments: vec![
                        ColorBlendAttachmentState {
                            blend: Some(AttachmentBlend::alpha()),
                            ..Default::default()
                        },
                        ColorBlendAttachmentState::default(),
                    ],
                    ..Default::default()
                }),
                viewport_state: Some(ViewportState::default()),
                dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )
        .unwrap()
    }

    fn load_entry_point(
        device: Arc<Device>,
        load_fn: impl Fn(Arc<Device>) -> Result<Arc<ShaderModule>, Validated<VulkanError>>,
    ) -> EntryPoint {
        load_fn(device).unwrap().entry_point("main").unwrap()
    }

    /// Send all canvas drawing data to the gpu to draw.
    pub fn draw(
        &self,
        context: &RenderContext,
        info: &FrameRenderInfo,
        camera: &CameraInfo,
        canvas: &Canvas,
        model_assets: &mut ModelAssets,
        texture_assets: &mut TextureAssets,
        image_assets: &mut ImageAssets,
        asset_manager: &mut AssetManager,
        platform: &Platform,
        clear_color: Vec4,
        eid_image: Arc<ImageView>,
    ) -> Arc<PrimaryAutoCommandBuffer> {
        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: info.window_size.as_vec2().to_array(),
            depth_range: 0.0..=1.0,
        };

        let framebuffer = Framebuffer::new(
            // TODO: pre-create framebuffers when we can get swapchain image views from VulkanoWindowRenderer
            self.render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![
                    info.image.clone(),
                    eid_image,
                    self.depth_stencil_images[info.window_index][info.image_index].clone(),
                ],
                ..Default::default()
            },
        )
        .unwrap();

        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            &context.command_buffer_allocator,
            context.graphics_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();
        command_buffer_builder
            .set_viewport(0, [viewport].into_iter().collect())
            .unwrap()
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![
                        Some(clear_color.to_array().into()),
                        Some(crate::render::canvas::eid_to_u32_array(EntityId::dead()).into()),
                        Some(1.0.into()),
                    ],
                    ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                },
                SubpassBeginInfo {
                    contents: SubpassContents::Inline,
                    ..Default::default()
                },
            )
            .unwrap();

        let projection_view = camera.projection_view(&info.window_size);
        let push_constants = shader::vertex::vs::PushConstants {
            projection_view: projection_view.to_cols_array_2d(),
        };

        draw_points(
            &canvas.points,
            self.pipeline_point.clone(),
            context.memory_allocator.clone(),
            &mut command_buffer_builder,
            push_constants,
        );
        draw_lines(
            &canvas.lines,
            self.pipeline_line.clone(),
            context.memory_allocator.clone(),
            &mut command_buffer_builder,
            push_constants,
        );
        draw_triangles(
            &canvas.triangles,
            self.pipeline_triangle.clone(),
            context.memory_allocator.clone(),
            &mut command_buffer_builder,
            push_constants,
        );
        draw_shapes(
            &canvas.rectangles,
            self.pipeline_shape2d.clone(),
            context.memory_allocator.clone(),
            &mut command_buffer_builder,
            push_constants,
            mesh::RECTANGLE_VERTICES.to_vec(),
            mesh::RECTANGLE_INDICES.to_vec(),
        );
        draw_shapes(
            &canvas.cicles,
            self.pipeline_circle.clone(),
            context.memory_allocator.clone(),
            &mut command_buffer_builder,
            push_constants,
            mesh::RECTANGLE_VERTICES.to_vec(),
            mesh::RECTANGLE_INDICES.to_vec(),
        );
        draw_textures(
            &canvas.textures,
            self.pipeline_texture.clone(),
            &mut command_buffer_builder,
            push_constants,
            context,
            texture_assets,
            image_assets,
            asset_manager,
            platform,
        );
        draw_shapes(
            &canvas.cuboids,
            self.pipeline_shape.clone(),
            context.memory_allocator.clone(),
            &mut command_buffer_builder,
            push_constants,
            mesh::CUBOID_VERTICES.to_vec(),
            mesh::CUBOID_INDICES.to_vec(),
        );
        draw_shapes(
            &canvas
                .spheres
                .iter()
                .map(|(model, color, _, eid)| (*model, *color, *eid))
                .collect(),
            self.pipeline_shape.clone(),
            context.memory_allocator.clone(),
            &mut command_buffer_builder,
            push_constants,
            mesh::SPHERE_VERTICES.to_vec(),
            mesh::SPHERE_INDICES.to_vec(),
        );
        draw_models(
            &canvas.models,
            self.pipeline_model.clone(),
            &mut command_buffer_builder,
            push_constants,
            context,
            model_assets,
            texture_assets,
            image_assets,
            asset_manager,
            platform,
        );

        command_buffer_builder
            .end_render_pass(Default::default())
            .unwrap();
        command_buffer_builder.build().unwrap()
        // There is a strange bug here that command buffer build will return an error with message "unsolvable resource conflict".
        // This is caused by hashing wrong key in the HashMap of vulkano::command_buffer::auto::builder::AutoSyncState::images.
        // We can work around this by changing HashMap<K, V> to Vec<(K, V)> and searching key by Arc::ptr_eq when traversing the vector.
        // After modifying local vulkano source file in ".cargo" folder, run "cargo clean" to force rebuilding vulkano locally.
        // TODO: fix this bug.
    }
}

fn draw_points(
    points: &Vec<(Vec3, Vec4, EntityId)>,
    pipeline: Arc<GraphicsPipeline>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    push_constants: shader::vertex::vs::PushConstants,
) {
    if points.is_empty() {
        return;
    }

    let vertices = points
        .iter()
        .map(|(v, c, e)| shader::vertex::VertexData::new(*v, *c, *e))
        .collect::<Vec<_>>();

    draw_vertices(
        vertices,
        pipeline,
        memory_allocator,
        command_buffer_builder,
        push_constants,
    );
}

fn draw_lines(
    lines: &Vec<[(Vec3, Vec4, EntityId); 2]>,
    pipeline: Arc<GraphicsPipeline>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    push_constants: shader::vertex::vs::PushConstants,
) {
    if lines.is_empty() {
        return;
    }

    let vertices = lines
        .iter()
        .flatten()
        .map(|(v, c, e)| shader::vertex::VertexData::new(*v, *c, *e))
        .collect::<Vec<_>>();

    draw_vertices(
        vertices,
        pipeline,
        memory_allocator,
        command_buffer_builder,
        push_constants,
    );
}

fn draw_triangles(
    triangles: &Vec<[(Vec3, Vec4, EntityId); 3]>,
    pipeline: Arc<GraphicsPipeline>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    push_constants: shader::vertex::vs::PushConstants,
) {
    if triangles.is_empty() {
        return;
    }

    let vertices = triangles
        .iter()
        .flatten()
        .map(|(v, c, e)| shader::vertex::VertexData::new(*v, *c, *e))
        .collect::<Vec<_>>();

    draw_vertices(
        vertices,
        pipeline,
        memory_allocator,
        command_buffer_builder,
        push_constants,
    );
}

fn draw_vertices(
    vertices: Vec<shader::vertex::VertexData>,
    pipeline: Arc<GraphicsPipeline>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    push_constants: shader::vertex::vs::PushConstants,
) {
    let vertex_buffer = create_buffer(vertices, &memory_allocator, BufferUsage::VERTEX_BUFFER);

    command_buffer_builder
        .bind_pipeline_graphics(pipeline.clone())
        .unwrap()
        .push_constants(pipeline.layout().clone(), 0, push_constants)
        .unwrap()
        .bind_vertex_buffers(0, vertex_buffer.clone())
        .unwrap()
        .draw(vertex_buffer.len() as u32, 1, 0, 0)
        .unwrap();
}

fn draw_shapes(
    shapes: &Vec<(Affine3A, Vec4, EntityId)>,
    pipeline: Arc<GraphicsPipeline>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    push_constants: shader::vertex::vs::PushConstants,
    vertices: Vec<Vec3>,
    indices: Vec<u16>,
) {
    if shapes.is_empty() {
        return;
    }

    let vertices = vertices
        .into_iter()
        .map(|v| shader::shape::VertexData::new(v));
    let instances = shapes
        .iter()
        .map(|(model, color, eid)| shader::shape::InstanceData::new(*color, *eid, *model))
        .collect::<Vec<_>>();

    let vertex_buffer = create_buffer(vertices, &memory_allocator, BufferUsage::VERTEX_BUFFER);
    let index_buffer = create_buffer(indices, &memory_allocator, BufferUsage::INDEX_BUFFER);
    let instance_buffer = create_buffer(instances, &memory_allocator, BufferUsage::VERTEX_BUFFER);

    command_buffer_builder
        .bind_pipeline_graphics(pipeline.clone())
        .unwrap()
        .push_constants(pipeline.layout().clone(), 0, push_constants)
        .unwrap()
        .bind_vertex_buffers(0, (vertex_buffer.clone(), instance_buffer.clone()))
        .unwrap()
        .bind_index_buffer(index_buffer.clone())
        .unwrap()
        .draw_indexed(
            index_buffer.len() as u32,
            instance_buffer.len() as u32,
            0,
            0,
            0,
        )
        .unwrap();
}

fn draw_textures(
    textures: &Vec<(AssetId, Affine3A, Vec4, EntityId)>,
    pipeline: Arc<GraphicsPipeline>,
    command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    push_constants: shader::vertex::vs::PushConstants,
    render_context: &RenderContext,
    texture_assets: &mut TextureAssets,
    image_assets: &mut ImageAssets,
    asset_manager: &mut AssetManager,
    platform: &Platform,
) {
    if textures.is_empty() {
        return;
    }

    let mut instances = Vec::new();
    let mut image_view_samplers = Vec::new();
    let mut image_to_index = HashMap::new();
    for (asset, model, color, eid) in textures {
        if let Some((image_view, sampler)) = texture_assets.get_texture(
            *asset,
            image_assets,
            asset_manager,
            platform,
            render_context,
        ) {
            let model = *model
                * Affine3A::from_scale(Vec3::new(
                    image_view.image().extent()[0] as f32 / 100.0,
                    image_view.image().extent()[1] as f32 / 100.0,
                    1.0,
                ));
            let index = *image_to_index.entry(image_view.clone()).or_insert_with(|| {
                image_view_samplers.push((image_view, sampler));
                image_view_samplers.len() - 1
            });
            instances.push(shader::texture::InstanceData::new(
                *color, *eid, index, model,
            ));
        }
    }

    if instances.is_empty() {
        return;
    }

    let vertices = [
        Vec3::new(-0.5, -0.5, 0.0),
        Vec3::new(-0.5, 0.5, 0.0),
        Vec3::new(0.5, 0.5, 0.0),
        Vec3::new(0.5, -0.5, 0.0),
    ]
    .map(|v| shader::shape::VertexData::new(v));
    let indices = [0u16, 1, 2, 2, 3, 0];

    let vertex_buffer = create_buffer(
        vertices,
        &render_context.memory_allocator,
        BufferUsage::VERTEX_BUFFER,
    );
    let index_buffer = create_buffer(
        indices,
        &render_context.memory_allocator,
        BufferUsage::INDEX_BUFFER,
    );
    let instance_buffer = create_buffer(
        instances,
        &render_context.memory_allocator,
        BufferUsage::VERTEX_BUFFER,
    );
    let descriptor_set = PersistentDescriptorSet::new_variable(
        &render_context.descriptor_set_allocator,
        pipeline.layout().set_layouts()[0].clone(),
        image_view_samplers.len() as u32,
        [WriteDescriptorSet::image_view_sampler_array(
            0,
            0,
            image_view_samplers,
        )],
        [],
    )
    .unwrap();

    command_buffer_builder
        .bind_pipeline_graphics(pipeline.clone())
        .unwrap()
        .push_constants(pipeline.layout().clone(), 0, push_constants)
        .unwrap()
        .bind_vertex_buffers(0, (vertex_buffer.clone(), instance_buffer.clone()))
        .unwrap()
        .bind_index_buffer(index_buffer.clone())
        .unwrap()
        .bind_descriptor_sets(
            PipelineBindPoint::Graphics,
            pipeline.layout().clone(),
            0,
            descriptor_set,
        )
        .unwrap()
        .draw_indexed(
            index_buffer.len() as u32,
            instance_buffer.len() as u32,
            0,
            0,
            0,
        )
        .unwrap();
}

fn draw_models(
    models: &Vec<(AssetId, AssetId, Affine3A, Vec4, EntityId)>,
    pipeline: Arc<GraphicsPipeline>,
    command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    push_constants: shader::vertex::vs::PushConstants,
    render_context: &RenderContext,
    model_assets: &mut ModelAssets,
    texture_assets: &mut TextureAssets,
    image_assets: &mut ImageAssets,
    asset_manager: &mut AssetManager,
    platform: &Platform,
) {
    if models.is_empty() {
        return;
    }

    let mut vertex_buffers = Vec::new();
    let mut index_buffers = Vec::new();
    let mut instances = Vec::new();
    let mut model_to_index = HashMap::new();
    let mut image_view_samplers = Vec::new();
    let mut image_to_index = HashMap::new();
    for (model_asset, texture_asset, model_matrix, color, eid) in models {
        if let Some(model) = model_assets.get_model(*model_asset, asset_manager, platform) {
            let index = *model_to_index.entry(*model_asset).or_insert_with(|| {
                let vertices = model.vertices.iter().map(|v| shader::model::VertexData {
                    position: v.position,
                    // the OBJ format assumes a coordinate system where a vertical coordinate of 0 means the bottom of the image
                    tex_coord: [v.texture[0], 1.0 - v.texture[1]],
                });
                let vertex_buffer = create_buffer(
                    vertices,
                    &render_context.memory_allocator,
                    BufferUsage::VERTEX_BUFFER,
                );
                let index_buffer = create_buffer(
                    model.indices.clone(),
                    &render_context.memory_allocator,
                    BufferUsage::INDEX_BUFFER,
                );
                vertex_buffers.push(vertex_buffer);
                index_buffers.push(index_buffer);
                instances.push(Vec::new());
                instances.len() - 1
            });
            let texture_index = if let Some((image_view, sampler)) = texture_assets.get_texture(
                *texture_asset,
                image_assets,
                asset_manager,
                platform,
                render_context,
            ) {
                *image_to_index.entry(image_view.clone()).or_insert_with(|| {
                    image_view_samplers.push((image_view, sampler));
                    image_view_samplers.len() - 1
                })
            } else {
                u32::MAX as usize
            };
            instances[index].push(shader::texture::InstanceData::new(
                *color,
                *eid,
                texture_index,
                *model_matrix,
            ));
        }
    }

    if instances.is_empty() {
        return;
    }

    let descriptor_set = PersistentDescriptorSet::new_variable(
        &render_context.descriptor_set_allocator,
        pipeline.layout().set_layouts()[0].clone(),
        image_view_samplers.len() as u32,
        if image_view_samplers.is_empty() {
            vec![]
        } else {
            vec![WriteDescriptorSet::image_view_sampler_array(
                0,
                0,
                image_view_samplers,
            )]
        },
        [],
    )
    .unwrap();

    command_buffer_builder
        .bind_pipeline_graphics(pipeline.clone())
        .unwrap()
        .push_constants(pipeline.layout().clone(), 0, push_constants)
        .unwrap()
        .bind_descriptor_sets(
            PipelineBindPoint::Graphics,
            pipeline.layout().clone(),
            0,
            descriptor_set,
        )
        .unwrap();

    for (instances, (vertex_buffer, index_buffer)) in
        zip(instances, zip(vertex_buffers, index_buffers))
    {
        let instance_buffer = create_buffer(
            instances,
            &render_context.memory_allocator,
            BufferUsage::VERTEX_BUFFER,
        );
        command_buffer_builder
            .bind_vertex_buffers(0, (vertex_buffer.clone(), instance_buffer.clone()))
            .unwrap()
            .bind_index_buffer(index_buffer.clone())
            .unwrap()
            .draw_indexed(
                index_buffer.len() as u32,
                instance_buffer.len() as u32,
                0,
                0,
                0,
            )
            .unwrap();
    }
}

/// Helper function to create a buffer from a list of data.
/// This can be used to create vertex buffer, index buffer, or instance buffer.
fn create_buffer<T: BufferContents>(
    data: impl IntoIterator<Item = T, IntoIter: ExactSizeIterator>,
    memory_allocator: &Arc<StandardMemoryAllocator>,
    usage: BufferUsage,
) -> Subbuffer<[T]> {
    Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        data.into_iter(),
    )
    .unwrap()
}
