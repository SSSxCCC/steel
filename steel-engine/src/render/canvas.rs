use super::{shader, texture::TextureAssets, FrameRenderInfo, RenderContext, RenderManager};
use crate::{
    asset::{AssetManager, ImageAssets},
    camera::CameraInfo,
};
use glam::{Affine3A, Mat4, Quat, UVec2, Vec3, Vec4};
use shipyard::{EntityId, Unique, UniqueView, UniqueViewMut};
use std::sync::Arc;
use steel_common::{asset::AssetId, platform::Platform};
use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, CopyImageToBufferInfo,
        PrimaryAutoCommandBuffer, PrimaryCommandBufferAbstract, RenderPassBeginInfo,
        SubpassBeginInfo, SubpassContents,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    format::Format,
    image::{view::ImageView, Image, ImageCreateInfo, ImageUsage},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{
        graphics::{
            color_blend::{AttachmentBlend, ColorBlendAttachmentState, ColorBlendState},
            depth_stencil::{DepthState, DepthStencilState},
            input_assembly::{InputAssemblyState, PrimitiveTopology},
            multisample::MultisampleState,
            rasterization::{PolygonMode, RasterizationState},
            vertex_input::{Vertex, VertexBufferDescription, VertexDefinition},
            viewport::{Viewport, ViewportState},
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
        PipelineShaderStageCreateInfo,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    shader::EntryPoint,
    sync::GpuFuture,
};

/// Canvas contains current frame's drawing data, which will be converted to vertex data, and send to gpu to draw.
/// You can use this unique to draw points, lines, triangles, rectangles, cicles, and textures to the screen.
#[derive(Unique, Default)]
pub struct Canvas {
    /// 1 vertex: (position, color, eid)
    points: Vec<(Vec3, Vec4, EntityId)>,
    /// 2 vertex: (position, color, eid)
    lines: Vec<[(Vec3, Vec4, EntityId); 2]>,
    /// 3 vertex: (position, color, eid)
    triangles: Vec<[(Vec3, Vec4, EntityId); 3]>,
    /// 4 vertex: (position, color, eid), index: 0, 1, 2, 2, 3, 0
    rectangles: Vec<[(Vec3, Vec4, EntityId); 4]>,
    /// (center, rotation, radius, color, eid)
    cicles: Vec<(Vec3, Quat, f32, Vec4, EntityId)>,
    /// (texture, model, color, eid)
    textures: Vec<(AssetId, Affine3A, Vec4, EntityId)>,
}

impl Canvas {
    /// Draw a point with position p, color, and EntityId eid. The EntityId is used for screen object picking.
    pub fn point(&mut self, p: Vec3, color: Vec4, eid: EntityId) {
        self.points.push((p, color, eid));
    }

    /// Draw a line from p1 to p2 with color and EntityId eid. The EntityId is used for screen object picking.
    pub fn line(&mut self, p1: Vec3, p2: Vec3, color: Vec4, eid: EntityId) {
        self.lines.push([(p1, color, eid), (p2, color, eid)]);
    }

    /// Draw a triangle with vertices p1, p2, p3, color, and EntityId eid. The EntityId is used for screen object picking.
    pub fn triangle(&mut self, p1: Vec3, p2: Vec3, p3: Vec3, color: Vec4, eid: EntityId) {
        self.triangles
            .push([(p1, color, eid), (p2, color, eid), (p3, color, eid)]);
    }

    /// Draw a rectangle with vertices p1, p2, p3, p4 (indices 0, 1, 2, 2, 3, 0), color, and EntityId eid. The EntityId is used for screen object picking.
    pub fn rectangle(
        &mut self,
        p1: Vec3,
        p2: Vec3,
        p3: Vec3,
        p4: Vec3,
        color: Vec4,
        eid: EntityId,
    ) {
        self.rectangles.push([
            (p1, color, eid),
            (p2, color, eid),
            (p3, color, eid),
            (p4, color, eid),
        ]);
    }

    /// Draw a circle with center, rotation, radius, color, and EntityId eid. The EntityId is used for screen object picking.
    pub fn circle(
        &mut self,
        center: Vec3,
        rotation: Quat,
        radius: f32,
        color: Vec4,
        eid: EntityId,
    ) {
        self.cicles.push((center, rotation, radius, color, eid));
    }

    pub fn texture(&mut self, asset: AssetId, color: Vec4, model: Affine3A, eid: EntityId) {
        self.textures.push((asset, model, color, eid));
    }

    /// Clear all drawing data.
    pub fn clear(&mut self) {
        self.points.clear();
        self.lines.clear();
        self.triangles.clear();
        self.rectangles.clear();
        self.cicles.clear();
        self.textures.clear();
    }
}

/// Clear the canvas.
pub fn canvas_clear_system(mut canvas: UniqueViewMut<Canvas>) {
    canvas.clear();
}

/// CanvasRenderContext stores many render objects that exist between frames.
pub struct CanvasRenderContext {
    /// The image vectors whose index at WindowIndex::GAME and WindowIndex::SCENE are for game window and scene window.
    pub depth_stencil_images: [Vec<Arc<ImageView>>; 2],
    pub eid_images: [Vec<Arc<ImageView>>; 2],
    pub render_pass: Arc<RenderPass>,
    pub pipeline_point: Arc<GraphicsPipeline>,
    pub pipeline_line: Arc<GraphicsPipeline>,
    pub pipeline_triangle: Arc<GraphicsPipeline>,
    pub pipeline_circle: Arc<GraphicsPipeline>,
    pub pipeline_texture: Arc<GraphicsPipeline>,
}

impl CanvasRenderContext {
    pub fn new(context: &RenderContext, info: &FrameRenderInfo) -> Self {
        let render_pass = Self::create_render_pass(context, info.format);
        let (pipeline_point, pipeline_line, pipeline_triangle, pipeline_circle, pipeline_texture) =
            Self::create_pipelines(context, render_pass.clone());
        CanvasRenderContext {
            depth_stencil_images: [Vec::new(), Vec::new()],
            eid_images: [Vec::new(), Vec::new()],
            render_pass,
            pipeline_point,
            pipeline_line,
            pipeline_triangle,
            pipeline_circle,
            pipeline_texture,
        }
    }

    pub fn update(&mut self, context: &RenderContext, info: &FrameRenderInfo) {
        self.update_depth_stencil_images(context, info);
        self.update_eid_images(context, info);
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
    ) {
        let vs = shader::vs::load(context.device.clone())
            .unwrap()
            .entry_point("main")
            .unwrap();
        let fs = shader::fs::load(context.device.clone())
            .unwrap()
            .entry_point("main")
            .unwrap();

        let pipeline_point = Self::create_pipeline(
            context,
            render_pass.clone(),
            &MyVertex::per_vertex(),
            PrimitiveTopology::PointList,
            PolygonMode::Point,
            vs.clone(),
            fs.clone(),
        );

        let pipeline_line = Self::create_pipeline(
            context,
            render_pass.clone(),
            &MyVertex::per_vertex(),
            PrimitiveTopology::LineList,
            PolygonMode::Line,
            vs.clone(),
            fs.clone(),
        );

        let pipeline_triangle = Self::create_pipeline(
            context,
            render_pass.clone(),
            &MyVertex::per_vertex(),
            PrimitiveTopology::TriangleList,
            PolygonMode::Fill,
            vs.clone(),
            fs.clone(),
        );

        let pipeline_circle = Self::create_pipeline(
            context,
            render_pass.clone(),
            &MyVertex::per_vertex(),
            PrimitiveTopology::TriangleList,
            PolygonMode::Fill,
            shader::circle::vs::load(context.device.clone())
                .unwrap()
                .entry_point("main")
                .unwrap(),
            shader::circle::fs::load(context.device.clone())
                .unwrap()
                .entry_point("main")
                .unwrap(),
        );

        let pipeline_texture = Self::create_pipeline(
            context,
            render_pass.clone(),
            &MyVertex::per_vertex(),
            PrimitiveTopology::TriangleList,
            PolygonMode::Fill,
            shader::texture::vs::load(context.device.clone())
                .unwrap()
                .entry_point("main")
                .unwrap(),
            shader::texture::fs::load(context.device.clone())
                .unwrap()
                .entry_point("main")
                .unwrap(),
        );

        (
            pipeline_point,
            pipeline_line,
            pipeline_triangle,
            pipeline_circle,
            pipeline_texture,
        )
    }

    fn create_pipeline(
        context: &RenderContext,
        render_pass: Arc<RenderPass>,
        vertex_buffer_description: &VertexBufferDescription,
        topology: PrimitiveTopology,
        polygon_mode: PolygonMode,
        vs: EntryPoint,
        fs: EntryPoint,
    ) -> Arc<GraphicsPipeline> {
        let vertex_input_state = vertex_buffer_description
            .definition(&vs.info().input_interface)
            .unwrap();
        let stages = [
            PipelineShaderStageCreateInfo::new(vs),
            PipelineShaderStageCreateInfo::new(fs),
        ];
        let layout = PipelineLayout::new(
            context.device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
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
}

/// Send all canvas drawing data to the gpu to draw.
pub fn canvas_render_system(
    info: UniqueView<FrameRenderInfo>,
    camera: UniqueView<CameraInfo>,
    canvas: UniqueView<Canvas>,
    mut render_manager: UniqueViewMut<RenderManager>,
    mut texture_assets: UniqueViewMut<TextureAssets>,
    mut image_assets: UniqueViewMut<ImageAssets>,
    mut asset_manager: UniqueViewMut<AssetManager>,
    platform: UniqueView<Platform>,
) -> Arc<PrimaryAutoCommandBuffer> {
    render_manager.update(&info);
    let context = &render_manager.context;
    let canvas_context = render_manager.canvas_context.as_ref().unwrap();

    let viewport = Viewport {
        offset: [0.0, 0.0],
        extent: info.window_size.as_vec2().to_array(),
        depth_range: 0.0..=1.0,
    };

    let framebuffer = Framebuffer::new(
        // TODO: pre-create framebuffers when we can get swapchain image views from VulkanoWindowRenderer
        canvas_context.render_pass.clone(),
        FramebufferCreateInfo {
            attachments: vec![
                info.image.clone(),
                canvas_context.eid_images[info.window_index][info.image_index].clone(),
                canvas_context.depth_stencil_images[info.window_index][info.image_index].clone(),
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
                    Some(render_manager.clear_color.to_array().into()),
                    Some(eid_to_u32_array(EntityId::dead()).into()),
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
    let push_constants = shader::vs::PushConstants {
        projection_view: projection_view.to_cols_array_2d(),
    };

    draw_points(
        &canvas.points,
        canvas_context.pipeline_point.clone(),
        context.memory_allocator.clone(),
        &mut command_buffer_builder,
        push_constants,
    );
    draw_lines(
        &canvas.lines,
        canvas_context.pipeline_line.clone(),
        context.memory_allocator.clone(),
        &mut command_buffer_builder,
        push_constants,
    );
    draw_triangles(
        &canvas.triangles,
        canvas_context.pipeline_triangle.clone(),
        context.memory_allocator.clone(),
        &mut command_buffer_builder,
        push_constants,
    );
    draw_rectangles(
        &canvas.rectangles,
        canvas_context.pipeline_triangle.clone(),
        context.memory_allocator.clone(),
        &mut command_buffer_builder,
        push_constants,
    );
    draw_circles(
        &canvas.cicles,
        canvas_context.pipeline_circle.clone(),
        context.memory_allocator.clone(),
        &mut command_buffer_builder,
        &projection_view,
    );
    draw_textures(
        &canvas.textures,
        canvas_context.pipeline_texture.clone(),
        &mut command_buffer_builder,
        &projection_view,
        context,
        texture_assets.as_mut(),
        image_assets.as_mut(),
        asset_manager.as_mut(),
        platform.as_ref(),
    );

    command_buffer_builder
        .end_render_pass(Default::default())
        .unwrap();
    command_buffer_builder.build().unwrap()
}

fn draw_points(
    points: &Vec<(Vec3, Vec4, EntityId)>,
    pipeline: Arc<GraphicsPipeline>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    push_constants: shader::vs::PushConstants,
) {
    if points.is_empty() {
        return;
    }

    let vertices = points
        .iter()
        .map(|(v, c, e)| MyVertex::new(*v, *c, *e))
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
    push_constants: shader::vs::PushConstants,
) {
    if lines.is_empty() {
        return;
    }

    let vertices = lines
        .iter()
        .flatten()
        .map(|(v, c, e)| MyVertex::new(*v, *c, *e))
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
    push_constants: shader::vs::PushConstants,
) {
    if triangles.is_empty() {
        return;
    }

    let vertices = triangles
        .iter()
        .flatten()
        .map(|(v, c, e)| MyVertex::new(*v, *c, *e))
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
    vertices: Vec<MyVertex>,
    pipeline: Arc<GraphicsPipeline>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    push_constants: shader::vs::PushConstants,
) {
    let vertex_buffer = vertex_buffer(vertices, &memory_allocator);

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

fn draw_rectangles(
    rectangles: &Vec<[(Vec3, Vec4, EntityId); 4]>,
    pipeline: Arc<GraphicsPipeline>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    push_constants: shader::vs::PushConstants,
) {
    if rectangles.is_empty() {
        return;
    }

    let vertices = rectangles
        .iter()
        .flatten()
        .map(|(v, c, e)| MyVertex::new(*v, *c, *e))
        .collect::<Vec<_>>();

    let indices = rectangles
        .iter()
        .enumerate()
        .map(|(i, _)| [i * 4, i * 4 + 1, i * 4 + 2, i * 4 + 2, i * 4 + 3, i * 4])
        .flatten()
        .map(|i| i as u16)
        .collect::<Vec<_>>();

    let vertex_buffer = vertex_buffer(vertices, &memory_allocator);
    let index_buffer = index_buffer(indices, &memory_allocator);

    command_buffer_builder
        .bind_pipeline_graphics(pipeline.clone())
        .unwrap()
        .push_constants(pipeline.layout().clone(), 0, push_constants)
        .unwrap()
        .bind_vertex_buffers(0, vertex_buffer.clone())
        .unwrap()
        .bind_index_buffer(index_buffer.clone())
        .unwrap()
        .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
        .unwrap();
}

fn draw_circles(
    cicles: &Vec<(Vec3, Quat, f32, Vec4, EntityId)>,
    pipeline: Arc<GraphicsPipeline>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    projection_view: &Mat4,
) {
    command_buffer_builder
        .bind_pipeline_graphics(pipeline.clone())
        .unwrap();
    for (center, rotation, radius, color, eid) in cicles {
        let radius = *radius;
        let push_constants = shader::circle::vs::PushConstants {
            projection_view: projection_view.to_cols_array_2d(),
            center: center.to_array(),
            radius,
        };
        let model = Affine3A::from_rotation_translation(*rotation, *center);
        let vertex_buffer = vertex_buffer(
            [
                model.transform_point3(Vec3::new(-radius, -radius, 0.0)),
                model.transform_point3(Vec3::new(-radius, radius, 0.0)),
                model.transform_point3(Vec3::new(radius, radius, 0.0)),
                model.transform_point3(Vec3::new(radius, -radius, 0.0)),
            ]
            .map(|v| MyVertex::new(v, *color, *eid))
            .to_vec(),
            &memory_allocator,
        );
        let index_buffer = index_buffer(vec![0u16, 1, 2, 2, 3, 0], &memory_allocator);

        command_buffer_builder
            .push_constants(pipeline.layout().clone(), 0, push_constants)
            .unwrap()
            .bind_vertex_buffers(0, vertex_buffer)
            .unwrap()
            .bind_index_buffer(index_buffer.clone())
            .unwrap()
            .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
            .unwrap();
    }
}

fn draw_textures(
    textures: &Vec<(AssetId, Affine3A, Vec4, EntityId)>,
    pipeline: Arc<GraphicsPipeline>,
    command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    projection_view: &Mat4,
    render_context: &RenderContext,
    texture_assets: &mut TextureAssets,
    image_assets: &mut ImageAssets,
    asset_manager: &mut AssetManager,
    platform: &Platform,
) {
    command_buffer_builder
        .bind_pipeline_graphics(pipeline.clone())
        .unwrap();
    for (asset, model, color, eid) in textures {
        if let Some((image_view, sampler)) = texture_assets.get_texture(
            *asset,
            image_assets,
            asset_manager,
            platform,
            render_context,
        ) {
            let push_constants = shader::texture::vs::PushConstants {
                projection_view_model: (*projection_view * Mat4::from(*model)).to_cols_array_2d(),
            };
            let vertex_buffer = vertex_buffer(
                [
                    Vec3::new(-0.5, -0.5, 0.0),
                    Vec3::new(-0.5, 0.5, 0.0),
                    Vec3::new(0.5, 0.5, 0.0),
                    Vec3::new(0.5, -0.5, 0.0),
                ]
                .map(|v| MyVertex::new(v, *color, *eid))
                .to_vec(),
                &render_context.memory_allocator,
            );
            let index_buffer =
                index_buffer(vec![0u16, 1, 2, 2, 3, 0], &render_context.memory_allocator);
            let descriptor_set = PersistentDescriptorSet::new(
                &render_context.descriptor_set_allocator,
                pipeline.layout().set_layouts()[0].clone(),
                [
                    WriteDescriptorSet::sampler(0, sampler),
                    WriteDescriptorSet::image_view(1, image_view),
                ],
                [],
            )
            .unwrap();

            command_buffer_builder
                .push_constants(pipeline.layout().clone(), 0, push_constants)
                .unwrap()
                .bind_vertex_buffers(0, vertex_buffer)
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
                .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
                .unwrap();
        }
    }
}

fn vertex_buffer<T: BufferContents>(
    vertices: Vec<T>,
    memory_allocator: &Arc<StandardMemoryAllocator>,
) -> Subbuffer<[T]> {
    Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::VERTEX_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        vertices.into_iter(),
    )
    .unwrap()
}

fn index_buffer(
    indices: Vec<u16>,
    memory_allocator: &Arc<StandardMemoryAllocator>,
) -> Subbuffer<[u16]> {
    Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::INDEX_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        indices.into_iter(),
    )
    .unwrap()
}

#[derive(BufferContents, Vertex, Clone)]
#[repr(C)]
struct MyVertex {
    #[format(R32G32B32_SFLOAT)]
    position: [f32; 3],
    #[format(R32G32B32A32_SFLOAT)]
    color: [f32; 4],
    #[format(R32G32_UINT)]
    eid: [u32; 2],
}

impl MyVertex {
    fn new(position: Vec3, color: Vec4, eid: EntityId) -> Self {
        MyVertex {
            position: position.to_array(),
            color: color.to_array(),
            eid: eid_to_u32_array(eid),
        }
    }
}

fn eid_to_u32_array(eid: EntityId) -> [u32; 2] {
    let eid = eid.inner();
    [eid as u32, (eid >> 32) as u32]
}

fn u32_array_to_eid(arr: [u32; 2]) -> EntityId {
    let eid = ((arr[1] as u64) << 32) | (arr[0] as u64);
    EntityId::from_inner(eid).unwrap_or_default()
}

/// Parameters for [get_entity_at_screen_system].
#[derive(Unique)]
pub struct GetEntityAtScreenParam {
    pub window_index: usize,
    pub screen_position: UVec2,
}

/// Screen object picking system.
pub fn get_entity_at_screen_system(
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
