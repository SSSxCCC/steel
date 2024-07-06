use std::sync::Arc;
use glam::{Affine3A, Mat4, Quat, UVec2, Vec3, Vec4};
use vulkano::{buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer}, command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, CopyImageToBufferInfo, PrimaryAutoCommandBuffer, PrimaryCommandBufferAbstract, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents}, format::{ClearValue, Format}, image::{view::ImageView, Image, ImageCreateInfo, ImageUsage}, memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator}, pipeline::{graphics::{color_blend::{AttachmentBlend, ColorBlendAttachmentState, ColorBlendState}, depth_stencil::{DepthState, DepthStencilState}, input_assembly::{InputAssemblyState, PrimitiveTopology}, multisample::MultisampleState, rasterization::{PolygonMode, RasterizationState}, vertex_input::{Vertex, VertexBufferDescription, VertexDefinition}, viewport::{Viewport, ViewportState}, GraphicsPipelineCreateInfo}, layout::PipelineDescriptorSetLayoutCreateInfo, DynamicState, GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo}, render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass}, shader::{EntryPoint, ShaderModule}, sync::GpuFuture};
use shipyard::{EntityId, Unique, UniqueView, UniqueViewMut};
use crate::camera::CameraInfo;
use super::{FrameRenderInfo, RenderContext, RenderManager};

/// Canvas contains current frame's drawing data, which will be converted to vertex data, and send to gpu to draw.
/// You can use this unique to draw points, lines, triangles, rectangles, cicles to the screen.
#[derive(Unique)]
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
}

impl Canvas {
    /// Create an empty Canvas.
    pub fn new() -> Self {
        Canvas { points: Vec::new(), lines: Vec::new(), triangles: Vec::new(), rectangles: Vec::new(), cicles: Vec::new() }
    }

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
        self.triangles.push([(p1, color, eid), (p2, color, eid), (p3, color, eid)]);
    }

    /// Draw a rectangle with vertices p1, p2, p3, p4 (indices 0, 1, 2, 2, 3, 0), color, and EntityId eid. The EntityId is used for screen object picking.
    pub fn rectangle(&mut self, p1: Vec3, p2: Vec3, p3: Vec3, p4: Vec3, color: Vec4, eid: EntityId) {
        self.rectangles.push([(p1, color, eid), (p2, color, eid), (p3, color, eid), (p4, color, eid)]);
    }

    /// Draw a circle with center, rotation, radius, color, and EntityId eid. The EntityId is used for screen object picking.
    pub fn circle(&mut self, center: Vec3, rotation: Quat, radius: f32, color: Vec4, eid: EntityId) {
        self.cicles.push((center, rotation, radius, color, eid));
    }

    /// Clear all drawing data.
    pub fn clear(&mut self) {
        self.points.clear();
        self.lines.clear();
        self.triangles.clear();
        self.rectangles.clear();
        self.cicles.clear();
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
    pub eid_image_futures: [Vec<Box<dyn GpuFuture + Send + Sync>>; 2],
    pub render_pass: Arc<RenderPass>,
    pub pipeline_point: Arc<GraphicsPipeline>,
    pub pipeline_line: Arc<GraphicsPipeline>,
    pub pipeline_triangle: Arc<GraphicsPipeline>,
    pub pipeline_circle: Arc<GraphicsPipeline>,
    pub render_pass_eid: Arc<RenderPass>,
    pub pipeline_point_eid: Arc<GraphicsPipeline>,
    pub pipeline_line_eid: Arc<GraphicsPipeline>,
    pub pipeline_triangle_eid: Arc<GraphicsPipeline>,
    pub pipeline_circle_eid: Arc<GraphicsPipeline>,
}

impl CanvasRenderContext {
    pub fn new(context: &RenderContext, info: &FrameRenderInfo) -> Self {
        let render_pass = Self::create_render_pass(context, info.format);
        let (pipeline_point, pipeline_line, pipeline_triangle, pipeline_circle) = Self::create_pipelines(context, render_pass.clone(), &MyVertex::per_vertex(), Some(AttachmentBlend::alpha()),
            vs::load(context.device.clone()).unwrap(), fs::load(context.device.clone()).unwrap(),
            circle::vs::load(context.device.clone()).unwrap(), circle::fs::load(context.device.clone()).unwrap());
        let render_pass_eid = Self::create_render_pass(context, Format::R32G32_UINT);
        let (pipeline_point_eid, pipeline_line_eid, pipeline_triangle_eid, pipeline_circle_eid) = Self::create_pipelines(context, render_pass_eid.clone(), &MyVertexEid::per_vertex(), None,
            eid::vs::load(context.device.clone()).unwrap(), eid::fs::load(context.device.clone()).unwrap(),
            eid::circle::vs::load(context.device.clone()).unwrap(), eid::circle::fs::load(context.device.clone()).unwrap());
        CanvasRenderContext {
            depth_stencil_images: [Vec::new(), Vec::new()], eid_images: [Vec::new(), Vec::new()], eid_image_futures: [Vec::new(), Vec::new()],
            render_pass, pipeline_point, pipeline_line, pipeline_triangle, pipeline_circle,
            render_pass_eid, pipeline_point_eid, pipeline_line_eid, pipeline_triangle_eid, pipeline_circle_eid,
        }
    }

    pub fn update(&mut self, context: &RenderContext, info: &FrameRenderInfo) {
        self.update_depth_stencil_images(context, info);
        self.update_eid_images(context, info);
    }

    fn update_depth_stencil_images(&mut self, context: &RenderContext, info: &FrameRenderInfo) {
        let depth_stencil_images = &mut self.depth_stencil_images[info.window_index];
        if depth_stencil_images.len() >= info.image_count { // TODO: use == instead of >= when we can get right image count
            let [width, height, _] = depth_stencil_images[0].image().extent();
            if info.window_size.x == width && info.window_size.y == height {
                return;
            }
        }
        log::trace!("Create depth stencil images, image_count={}", info.image_count);
        *depth_stencil_images = (0..info.image_count).map(|_| {
            let image = Image::new(
                context.memory_allocator.clone(),
                ImageCreateInfo {
                    format: Format::D32_SFLOAT,
                    extent: [info.window_size.x, info.window_size.y, 1],
                    usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT,
                    ..Default::default()
                },
                AllocationCreateInfo::default(),
            ).unwrap();
            ImageView::new_default(image).unwrap()
        }).collect();
    }

    fn update_eid_images(&mut self, context: &RenderContext, info: &FrameRenderInfo) {
        let eid_images = &mut self.eid_images[info.window_index];
        if eid_images.len() >= info.image_count { // TODO: use == instead of >= when we can get right image count
            let [width, height, _] = eid_images[0].image().extent();
            if info.window_size.x == width && info.window_size.y == height {
                return;
            }
        }
        log::trace!("Create eid images, image_count={}", info.image_count);
        *eid_images = (0..info.image_count).map(|_| {
            let image = Image::new(
                context.memory_allocator.clone(),
                ImageCreateInfo {
                    format: Format::R32G32_UINT,
                    extent: [info.window_size.x, info.window_size.y, 1],
                    usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_SRC,
                    ..Default::default()
                },
                AllocationCreateInfo::default(),
            ).unwrap();
            ImageView::new_default(image).unwrap()
        }).collect();
        self.eid_image_futures[info.window_index] = (0..info.image_count)
            .map(|_| vulkano::sync::now(context.device.clone()).boxed_send_sync())
            .collect();
    }

    fn create_render_pass(context: &RenderContext, format: Format) -> Arc<RenderPass> {
        vulkano::single_pass_renderpass!(
            context.device.clone(),
            attachments: {
                color: { format: format, samples: 1, load_op: Clear, store_op: Store },
                depth_stencil: { format: Format::D32_SFLOAT, samples: 1, load_op: Clear, store_op: DontCare },
            },
            pass: {
                color: [ color ],
                depth_stencil: { depth_stencil },
            },
        ).unwrap()
    }

    fn create_pipelines(context: &RenderContext, render_pass: Arc<RenderPass>, vertex_buffer_description: &VertexBufferDescription, blend: Option<AttachmentBlend>,
            vs: Arc<ShaderModule>, fs: Arc<ShaderModule>, vs_circle: Arc<ShaderModule>, fs_circle: Arc<ShaderModule>)
            -> (Arc<GraphicsPipeline>, Arc<GraphicsPipeline>, Arc<GraphicsPipeline>, Arc<GraphicsPipeline>) {
        let vs = vs.entry_point("main").unwrap();
        let fs = fs.entry_point("main").unwrap();

        let pipeline_point = Self::create_pipeline(context, render_pass.clone(),
            vertex_buffer_description, blend, PrimitiveTopology::PointList, PolygonMode::Point,
            vs.clone(), fs.clone());

        let pipeline_line = Self::create_pipeline(context, render_pass.clone(),
            vertex_buffer_description, blend, PrimitiveTopology::LineList, PolygonMode::Line,
            vs.clone(), fs.clone());

        let pipeline_triangle = Self::create_pipeline(context, render_pass.clone(),
            vertex_buffer_description, blend, PrimitiveTopology::TriangleList, PolygonMode::Fill,
            vs.clone(), fs.clone());

        let pipeline_circle = Self::create_pipeline(context, render_pass.clone(),
            vertex_buffer_description, blend, PrimitiveTopology::TriangleList, PolygonMode::Fill,
            vs_circle.entry_point("main").unwrap(), fs_circle.entry_point("main").unwrap());

        (pipeline_point, pipeline_line, pipeline_triangle, pipeline_circle)
    }

    fn create_pipeline(context: &RenderContext, render_pass: Arc<RenderPass>, vertex_buffer_description: &VertexBufferDescription,
            blend: Option<AttachmentBlend>, topology: PrimitiveTopology, polygon_mode: PolygonMode,
            vs: EntryPoint, fs: EntryPoint) -> Arc<GraphicsPipeline> {
        let vertex_input_state = vertex_buffer_description.definition(&vs.info().input_interface).unwrap();
        let stages = [
            PipelineShaderStageCreateInfo::new(vs),
            PipelineShaderStageCreateInfo::new(fs),
        ];
        let layout = PipelineLayout::new(
            context.device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(context.device.clone()).unwrap(),
        ).unwrap();
        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        GraphicsPipeline::new(context.device.clone(), None, GraphicsPipelineCreateInfo {
            stages: stages.into_iter().collect(),
            vertex_input_state: Some(vertex_input_state),
            input_assembly_state: Some(InputAssemblyState { topology, ..Default::default() }),
            rasterization_state: Some(RasterizationState { polygon_mode, ..Default::default() }),
            multisample_state: Some(MultisampleState::default()),
            depth_stencil_state: Some(DepthStencilState { depth: Some(DepthState::simple()), ..Default::default() }),
            color_blend_state: Some(ColorBlendState::with_attachment_states(
                subpass.num_color_attachments(),
                ColorBlendAttachmentState { blend, ..Default::default() })),
            viewport_state: Some(ViewportState::default()),
            dynamic_state: [DynamicState::Viewport].into_iter().collect(),
            subpass: Some(subpass.into()),
            ..GraphicsPipelineCreateInfo::layout(layout)
        }).unwrap()
    }
}

/// Send all canvas drawing data to the gpu to draw.
pub fn canvas_render_system(info: UniqueView<FrameRenderInfo>, camera: UniqueView<CameraInfo>,
        canvas: UniqueView<Canvas>, mut render_manager: UniqueViewMut<RenderManager>) -> Arc<PrimaryAutoCommandBuffer> {
    render_manager.update(&info);
    let context = &render_manager.context;
    let canvas_context = render_manager.canvas_context.as_ref().unwrap();

    let viewport = Viewport {
        offset: [0.0, 0.0],
        extent: info.window_size.as_vec2().to_array(),
        depth_range: 0.0..=1.0,
    };
    let projection_view = camera.projection_view(&info.window_size);

    let command_buffer = draw_image::<MyVertex>(context, canvas_context, canvas_context.render_pass.clone(), info.image.clone(), info.window_index, info.image_index,
        viewport.clone(), render_manager.clear_color.to_array().into(), projection_view, canvas.as_ref(),
        canvas_context.pipeline_point.clone(), canvas_context.pipeline_line.clone(), canvas_context.pipeline_triangle.clone(), canvas_context.pipeline_circle.clone());

    // draw eid image
    let command_buffer_eid = draw_image::<MyVertexEid>(context, canvas_context, canvas_context.render_pass_eid.clone(), canvas_context.eid_images[info.window_index][info.image_index].clone(), info.window_index, info.image_index,
    viewport, [0u32, 0u32].into(), projection_view, canvas.as_ref(),
    canvas_context.pipeline_point_eid.clone(), canvas_context.pipeline_line_eid.clone(), canvas_context.pipeline_triangle_eid.clone(), canvas_context.pipeline_circle_eid.clone());

    // TODO: should we execute after the image has drawn since image and eid_image use the same depth_stencil_image?
    let eid_image_future = command_buffer_eid
        .execute(context.graphics_queue.clone()).unwrap()
        .boxed_send_sync();
    render_manager.canvas_context.as_mut().unwrap().eid_image_futures[info.window_index][info.image_index] = eid_image_future;

    return command_buffer;
}

fn draw_image<V: IntoVertex>(context: &RenderContext, canvas_context: &CanvasRenderContext, render_pass: Arc<RenderPass>, image: Arc<ImageView>,
        window_index: usize, image_index: usize, viewport: Viewport, clear_value: ClearValue, projection_view: Mat4, canvas: &Canvas,
        pipeline_point: Arc<GraphicsPipeline>, pipeline_line: Arc<GraphicsPipeline>, pipeline_triangle: Arc<GraphicsPipeline>, pipeline_circle: Arc<GraphicsPipeline>)
        -> Arc<PrimaryAutoCommandBuffer> {
    let depth_stencil_image = canvas_context.depth_stencil_images[window_index][image_index].clone();
    let framebuffer = Framebuffer::new( // TODO: pre-create framebuffers when we can get swapchain image views from VulkanoWindowRenderer
        render_pass,
        FramebufferCreateInfo {
            attachments: vec![image, depth_stencil_image],
            ..Default::default()
        },
    ).unwrap();

    let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
        &context.command_buffer_allocator,
        context.graphics_queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    ).unwrap();
    command_buffer_builder
        .set_viewport(0, [viewport].into_iter().collect()).unwrap()
        .begin_render_pass(
            RenderPassBeginInfo {
                clear_values: vec![Some(clear_value), Some(1.0.into())],
                ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
            },
            SubpassBeginInfo {
                contents: SubpassContents::Inline,
                ..Default::default()
            },
        ).unwrap();

    let push_constants = vs::PushConstants { projection_view: projection_view.to_cols_array_2d() };

    draw_points::<V>(&canvas.points, pipeline_point, context.memory_allocator.clone(), &mut command_buffer_builder, push_constants);
    draw_lines::<V>(&canvas.lines, pipeline_line, context.memory_allocator.clone(), &mut command_buffer_builder, push_constants);
    draw_triangles::<V>(&canvas.triangles, pipeline_triangle.clone(), context.memory_allocator.clone(), &mut command_buffer_builder, push_constants);
    draw_rectangles::<V>(&canvas.rectangles, pipeline_triangle, context.memory_allocator.clone(), &mut command_buffer_builder, push_constants);
    draw_circles::<V>(&canvas.cicles, pipeline_circle, context.memory_allocator.clone(), &mut command_buffer_builder, &projection_view);

    command_buffer_builder.end_render_pass(Default::default()).unwrap();
    command_buffer_builder.build().unwrap()
}

fn draw_points<V: IntoVertex>(points: &Vec<(Vec3, Vec4, EntityId)>, pipeline: Arc<GraphicsPipeline>, memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, push_constants: vs::PushConstants) {
    if points.is_empty() {
        return;
    }

    let vertices = points.iter()
        .map(|(v, c, e)| V::new(*v, *c, *e))
        .collect::<Vec<_>>();

    draw_vertices(vertices, pipeline, memory_allocator, command_buffer_builder, push_constants);
}

fn draw_lines<V: IntoVertex>(lines: &Vec<[(Vec3, Vec4, EntityId); 2]>, pipeline: Arc<GraphicsPipeline>, memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, push_constants: vs::PushConstants) {
    if lines.is_empty() {
        return;
    }

    let vertices = lines.iter()
        .flatten()
        .map(|(v, c, e)| V::new(*v, *c, *e))
        .collect::<Vec<_>>();

    draw_vertices(vertices, pipeline, memory_allocator, command_buffer_builder, push_constants);
}

fn draw_triangles<V: IntoVertex>(triangles: &Vec<[(Vec3, Vec4, EntityId); 3]>, pipeline: Arc<GraphicsPipeline>, memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, push_constants: vs::PushConstants) {
    if triangles.is_empty() {
        return;
    }

    let vertices = triangles.iter()
        .flatten()
        .map(|(v, c, e)| V::new(*v, *c, *e))
        .collect::<Vec<_>>();

    draw_vertices(vertices, pipeline, memory_allocator, command_buffer_builder, push_constants);
}

fn draw_vertices<V: BufferContents>(vertices: Vec<V>, pipeline: Arc<GraphicsPipeline>, memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, push_constants: vs::PushConstants) {
    let vertex_buffer = vertex_buffer(vertices, &memory_allocator);

    command_buffer_builder
        .bind_pipeline_graphics(pipeline.clone()).unwrap()
        .push_constants(pipeline.layout().clone(), 0, push_constants).unwrap()
        .bind_vertex_buffers(0, vertex_buffer.clone()).unwrap()
        .draw(vertex_buffer.len() as u32, 1, 0, 0).unwrap();
}

fn draw_rectangles<V: IntoVertex>(rectangles: &Vec<[(Vec3, Vec4, EntityId); 4]>, pipeline: Arc<GraphicsPipeline>, memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, push_constants: vs::PushConstants) {
    if rectangles.is_empty() {
        return;
    }

    let vertices = rectangles.iter()
        .flatten()
        .map(|(v, c, e)| V::new(*v, *c, *e))
        .collect::<Vec<_>>();

    let indices = rectangles.iter()
        .enumerate()
        .map(|(i, _)| [i * 4, i * 4 + 1, i * 4 + 2, i * 4 + 2, i * 4 + 3, i * 4])
        .flatten()
        .map(|i| i as u16)
        .collect::<Vec<_>>();

    let vertex_buffer = vertex_buffer(vertices, &memory_allocator);
    let index_buffer = index_buffer(indices, &memory_allocator);

    command_buffer_builder
        .bind_pipeline_graphics(pipeline.clone()).unwrap()
        .push_constants(pipeline.layout().clone(), 0, push_constants).unwrap()
        .bind_vertex_buffers(0, vertex_buffer.clone()).unwrap()
        .bind_index_buffer(index_buffer.clone()).unwrap()
        .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0).unwrap();
}

fn draw_circles<V: IntoVertex>(cicles: &Vec<(Vec3, Quat, f32, Vec4, EntityId)>, pipeline: Arc<GraphicsPipeline>, memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, projection_view: &Mat4) {
    command_buffer_builder.bind_pipeline_graphics(pipeline.clone()).unwrap();
    for (center, rotation, radius, color, eid) in cicles {
        let radius = *radius;
        let push_constants = circle::vs::PushConstants {
            projection_view: projection_view.to_cols_array_2d(), center: center.to_array(), radius };

        let model = Affine3A::from_rotation_translation(*rotation, *center);
        let vertex_buffer = vertex_buffer([
            model.transform_point3(Vec3::new(-radius, -radius, 0.0)),
            model.transform_point3(Vec3::new(-radius, radius, 0.0)),
            model.transform_point3(Vec3::new(radius, radius, 0.0)),
            model.transform_point3(Vec3::new(radius, -radius, 0.0)),
        ].map(|v| V::new(v, *color, *eid)).to_vec(), &memory_allocator);
        let index_buffer = index_buffer(vec![0u16, 1, 2, 2, 3, 0], &memory_allocator);

        command_buffer_builder
            .push_constants(pipeline.layout().clone(), 0, push_constants).unwrap()
            .bind_vertex_buffers(0, vertex_buffer).unwrap()
            .bind_index_buffer(index_buffer.clone()).unwrap()
            .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0).unwrap();
    }
}

fn vertex_buffer<T: BufferContents>(vertices: Vec<T>, memory_allocator: &Arc<StandardMemoryAllocator>) -> Subbuffer<[T]> {
    Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo { usage: BufferUsage::VERTEX_BUFFER, ..Default::default() },
        AllocationCreateInfo { memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE, ..Default::default() },
        vertices.into_iter()
    ).unwrap()
}

fn index_buffer(indices: Vec<u16>, memory_allocator: &Arc<StandardMemoryAllocator>) -> Subbuffer<[u16]> {
    Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo { usage: BufferUsage::INDEX_BUFFER, ..Default::default() },
        AllocationCreateInfo { memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE, ..Default::default() },
        indices.into_iter()
    ).unwrap()
}

#[derive(BufferContents, Vertex, Clone)]
#[repr(C)]
struct MyVertex {
    #[format(R32G32B32_SFLOAT)]
    position: [f32; 3],
    #[format(R32G32B32A32_SFLOAT)]
    color: [f32; 4],
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 460

            layout(push_constant) uniform PushConstants {
                mat4 projection_view;
            } pcs;

            layout(location = 0) in vec3 position;
            layout(location = 1) in vec4 color;

            layout(location = 0) out vec4 out_color;

            void main() {
                gl_Position = pcs.projection_view * vec4(position, 1.0);
                out_color = color;
            }
        ",
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 460

            layout(location = 0) in vec4 in_color;

            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = in_color;
            }
        ",
    }
}

mod circle {
    pub mod vs {
        vulkano_shaders::shader! { // TODO: Use different PushConstants in vs and fs
            ty: "vertex",
            src: r"
                #version 460

                layout(push_constant) uniform PushConstants {
                    mat4 projection_view;
                    vec3 center;
                    float radius;
                } pcs;

                layout(location = 0) in vec3 position;
                layout(location = 1) in vec4 color;

                layout(location = 0) out vec3 out_position;
                layout(location = 1) out vec4 out_color;

                void main() {
                    gl_Position = pcs.projection_view * vec4(position, 1.0);
                    out_position = position;
                    out_color = color;
                }
            ",
        }
    }

    pub mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            src: r"
                #version 460

                layout(push_constant) uniform PushConstants {
                    mat4 projection_view;
                    vec3 center;
                    float radius;
                } pcs;

                layout(location = 0) in vec3 in_position;
                layout(location = 1) in vec4 in_color;

                layout(location = 0) out vec4 f_color;

                void main() {
                    if (distance(pcs.center, in_position) > pcs.radius) {
                        discard;
                    }
                    f_color = in_color;
                }
            ",
        }
    }
}

/// This union is used to convert between u64 and two u32.
union Eid {
    u64: u64,
    array_u32: [u32; 2],
}

fn eid_to_u32_array(eid: EntityId) -> [u32; 2] {
    let eid = Eid { u64: eid.inner() };
    unsafe { eid.array_u32 }
}

fn u32_array_to_eid(a: [u32; 2]) -> EntityId {
    let eid = Eid { array_u32: a };
    EntityId::from_inner(unsafe { eid.u64 }).unwrap_or(EntityId::dead())
}

#[derive(BufferContents, Vertex, Clone)]
#[repr(C)]
struct MyVertexEid {
    #[format(R32G32B32_SFLOAT)]
    position: [f32; 3],
    #[format(R32G32_UINT)]
    eid: [u32; 2],
}

mod eid {
    pub mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            src: r"
                #version 460

                layout(push_constant) uniform PushConstants {
                    mat4 projection_view;
                } pcs;

                layout(location = 0) in vec3 position;
                layout(location = 1) in uvec2 eid;

                layout(location = 0) out uvec2 out_eid;

                void main() {
                    gl_Position = pcs.projection_view * vec4(position, 1.0);
                    out_eid = eid;
                }
            ",
        }
    }

    pub mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            src: r"
                #version 460

                layout(location = 0) flat in uvec2 in_eid;

                layout(location = 0) out uvec2 f_eid;

                void main() {
                    f_eid = in_eid;
                }
            ",
        }
    }

    pub mod circle {
        pub mod vs {
            vulkano_shaders::shader! { // TODO: Use different PushConstants in vs and fs
                ty: "vertex",
                src: r"
                    #version 460

                    layout(push_constant) uniform PushConstants {
                        mat4 projection_view;
                        vec3 center;
                        float radius;
                    } pcs;

                    layout(location = 0) in vec3 position;
                    layout(location = 1) in uvec2 eid;

                    layout(location = 0) out vec3 out_position;
                    layout(location = 1) out uvec2 out_eid;

                    void main() {
                        gl_Position = pcs.projection_view * vec4(position, 1.0);
                        out_position = position;
                        out_eid = eid;
                    }
                ",
            }
        }

        pub mod fs {
            vulkano_shaders::shader! {
                ty: "fragment",
                src: r"
                    #version 460

                    layout(push_constant) uniform PushConstants {
                        mat4 projection_view;
                        vec3 center;
                        float radius;
                    } pcs;

                    layout(location = 0) in vec3 in_position;
                    layout(location = 1) flat in uvec2 in_eid;

                    layout(location = 0) out uvec2 f_eid;

                    void main() {
                        if (distance(pcs.center, in_position) > pcs.radius) {
                            discard;
                        }
                        f_eid = in_eid;
                    }
                ",
            }
        }
    }
}

trait IntoVertex: BufferContents + Clone {
    fn new(position: Vec3, color: Vec4, eid: EntityId) -> Self;
}

impl IntoVertex for MyVertex {
    fn new(position: Vec3, color: Vec4, _eid: EntityId) -> Self {
        MyVertex { position: position.to_array(), color: color.to_array() }
    }
}

impl IntoVertex for MyVertexEid {
    fn new(position: Vec3, _color: Vec4, eid: EntityId) -> Self {
        MyVertexEid { position: position.to_array(), eid: eid_to_u32_array(eid) }
    }
}

#[derive(Unique)]
pub struct GetEntityAtScreenParam {
    pub window_index: usize,
    pub screen_position: UVec2,
}

/// Screen object picking system.
pub fn get_entity_at_screen_system(mut render_manager: UniqueViewMut<RenderManager>, param: UniqueView<GetEntityAtScreenParam>) -> EntityId {
    let render_manager = render_manager.as_mut();
    if let Some(canvas_contex) = render_manager.canvas_context.as_mut() {
        let image_index = render_manager.image_index[param.window_index];
        if canvas_contex.eid_images[param.window_index].len() > image_index {
            let eid_image = &canvas_contex.eid_images[param.window_index][image_index];
            let [width, height, _] = eid_image.image().extent();
            let buffer = Buffer::from_iter(
                render_manager.context.memory_allocator.clone(),
                BufferCreateInfo { usage: BufferUsage::TRANSFER_DST, ..Default::default() },
                AllocationCreateInfo { memory_type_filter: MemoryTypeFilter::PREFER_HOST | MemoryTypeFilter::HOST_RANDOM_ACCESS, ..Default::default() },
                (0..width * height * 2).map(|_| 0u32)).unwrap();
            let mut builder = AutoCommandBufferBuilder::primary(
                &render_manager.context.command_buffer_allocator,
                render_manager.context.graphics_queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit).unwrap();
            builder.copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(eid_image.image().clone(), buffer.clone())).unwrap();
            let future = std::mem::replace(&mut canvas_contex.eid_image_futures[param.window_index][image_index], vulkano::sync::now(render_manager.context.device.clone()).boxed_send_sync());
            let future = builder.build().unwrap()
                .execute_after(future, render_manager.context.graphics_queue.clone()).unwrap()
                .then_signal_fence_and_flush().unwrap();
            future.wait(None).unwrap();
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
