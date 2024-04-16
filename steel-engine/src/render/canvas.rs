use std::sync::Arc;
use glam::{Mat4, Quat, Vec3, Vec4, Vec4Swizzles};
use vulkano::{buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer}, command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassContents}, format::Format, image::{view::ImageView, ImageAccess, ImageDimensions, ImageUsage, StorageImage}, memory::allocator::{AllocationCreateInfo, MemoryUsage, StandardMemoryAllocator}, pipeline::{graphics::{color_blend::ColorBlendState, depth_stencil::DepthStencilState, input_assembly::{InputAssemblyState, PrimitiveTopology}, rasterization::{PolygonMode, RasterizationState}, vertex_input::Vertex, viewport::{Viewport, ViewportState}}, GraphicsPipeline, Pipeline}, render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass}};
use shipyard::{Unique, UniqueView, UniqueViewMut};
use crate::camera::CameraInfo;
use super::{FrameRenderInfo, RenderContext, RenderManager};

#[derive(Unique)]
pub struct Canvas {
    /// 1 vertex: (position, color)
    pub points: Vec<(Vec3, Vec4)>,
    /// 2 vertex: (position, color)
    pub lines: Vec<[(Vec3, Vec4); 2]>,
    /// 3 vertex: (position, color)
    pub triangles: Vec<[(Vec3, Vec4); 3]>,
    /// 4 vertex: (position, color), index: 0, 1, 2, 2, 3, 0
    pub rectangles: Vec<[(Vec3, Vec4); 4]>,
    /// (center, rotation, radius, color)
    pub cicles: Vec<(Vec3, Quat, f32, Vec4)>,
}

impl Canvas {
    pub fn new() -> Self {
        Canvas { points: Vec::new(), lines: Vec::new(), triangles: Vec::new(), rectangles: Vec::new(), cicles: Vec::new() }
    }

    pub fn clear(&mut self) {
        self.points.clear();
        self.lines.clear();
        self.triangles.clear();
        self.rectangles.clear();
        self.cicles.clear();
    }
}

pub fn canvas_clear_system(mut canvas: UniqueViewMut<Canvas>) {
    canvas.clear();
}

/// CanvasRenderContext stores many render objects that exist between frames
pub struct CanvasRenderContext {
    /// The image vectors whose index at WindowIndex::GAME and WindowIndex::SCENE are for game window and scene window
    pub depth_stencil_images: [Vec<Arc<ImageView<StorageImage>>>; 2],
    pub render_pass: Arc<RenderPass>,
    pub pipeline_point: Arc<GraphicsPipeline>,
    pub pipeline_line: Arc<GraphicsPipeline>,
    pub pipeline_triangle: Arc<GraphicsPipeline>,
    pub pipeline_circle: Arc<GraphicsPipeline>,
}

impl CanvasRenderContext {
    pub fn new(context: &RenderContext, info: &FrameRenderInfo) -> Self {
        let render_pass = Self::create_render_pass(context, info);
        let (pipeline_point, pipeline_line, pipeline_triangle, pipeline_circle) = Self::create_pipelines(context, render_pass.clone());
        CanvasRenderContext {
            depth_stencil_images: [Vec::new(), Vec::new()],
            render_pass, pipeline_point, pipeline_line, pipeline_triangle, pipeline_circle,
        }
    }

    pub fn update(&mut self, context: &RenderContext, info: &FrameRenderInfo) {
        self.update_depth_stencil_images(context, info);
    }

    fn update_depth_stencil_images(&mut self, context: &RenderContext, info: &FrameRenderInfo) {
        let depth_stencil_images: &mut Vec<Arc<ImageView<StorageImage>>> = &mut self.depth_stencil_images[info.window_index];
        if depth_stencil_images.len() >= info.image_count { // TODO: use == instead of >= when we can get right image count
            if let ImageDimensions::Dim2d { width, height, .. } = depth_stencil_images[0].image().dimensions() {
                if info.window_size.x == width && info.window_size.y == height {
                    return;
                }
            }
        }
        log::debug!("Create depth stencil images, image_count={}", info.image_count);
        *depth_stencil_images = (0..info.image_count).map(|_| StorageImage::general_purpose_image_view(
            context.memory_allocator.as_ref(),
            context.graphics_queue.clone(),
            info.window_size.to_array(),
            Format::D32_SFLOAT,
            ImageUsage::DEPTH_STENCIL_ATTACHMENT,
        ).unwrap()).collect();
    }

    fn create_render_pass(context: &RenderContext, info: &FrameRenderInfo) -> Arc<RenderPass> {
        vulkano::single_pass_renderpass!(
            context.device.clone(),
            attachments: {
                color: { load: Clear, store: Store, format: info.format, samples: 1 },
                depth_stencil: { load: Clear, store: DontCare, format: Format::D32_SFLOAT, samples: 1 },
            },
            pass: {
                color: [ color ],
                depth_stencil: { depth_stencil },
            },
        ).unwrap()
    }

    fn create_pipelines(context: &RenderContext, render_pass: Arc<RenderPass>) -> (Arc<GraphicsPipeline>, Arc<GraphicsPipeline>, Arc<GraphicsPipeline>, Arc<GraphicsPipeline>) {
        let base_pipeline_builder = GraphicsPipeline::start()
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .vertex_input_state(MyVertex::per_vertex())
            .render_pass(Subpass::from(render_pass, 0).unwrap())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .color_blend_state(ColorBlendState::default().blend_alpha()); // TODO: implement order independent transparency

        let vs = vs::load(context.device.clone()).expect("failed to create shader module");
        let fs = fs::load(context.device.clone()).expect("failed to create shader module");
        let pipeline_builder = base_pipeline_builder.clone()
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            .fragment_shader(fs.entry_point("main").unwrap(), ());

        let pipeline_point = pipeline_builder.clone()
            .input_assembly_state(InputAssemblyState::new().topology(PrimitiveTopology::PointList))
            .rasterization_state(RasterizationState::new().polygon_mode(PolygonMode::Point))
            .build(context.device.clone())
            .unwrap();

        let pipeline_line = pipeline_builder.clone()
            .input_assembly_state(InputAssemblyState::new().topology(PrimitiveTopology::LineList))
            .rasterization_state(RasterizationState::new().polygon_mode(PolygonMode::Line))
            .build(context.device.clone())
            .unwrap();

        let pipeline_triangle = pipeline_builder.clone()
            .build(context.device.clone())
            .unwrap();

        let vs = circle::vs::load(context.device.clone()).expect("failed to create shader module");
        let fs = circle::fs::load(context.device.clone()).expect("failed to create shader module");
        let pipeline_circle = base_pipeline_builder.clone()
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            .fragment_shader(fs.entry_point("main").unwrap(), ())
            .build(context.device.clone())
            .unwrap();

        (pipeline_point, pipeline_line, pipeline_triangle, pipeline_circle)
    }
}

pub fn canvas_render_system(info: UniqueView<FrameRenderInfo>, camera: UniqueView<CameraInfo>, canvas: UniqueView<Canvas>, mut render_manager: UniqueViewMut<RenderManager>) -> PrimaryAutoCommandBuffer {
    render_manager.update(&info);
    let context = &render_manager.context;
    let canvas_context = render_manager.canvas_context.as_ref().unwrap();

    let depth_stencil_image = canvas_context.depth_stencil_images[info.window_index][info.image_index].clone();
    let framebuffer = Framebuffer::new( // TODO: pre-create framebuffers when we can get swapchain image views from VulkanoWindowRenderer
        canvas_context.render_pass.clone(),
        FramebufferCreateInfo {
            attachments: vec![info.image.clone(), depth_stencil_image],
            ..Default::default()
        },
    ).unwrap();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: info.window_size.as_vec2().to_array(),
        depth_range: 0.0..1.0,
    };

    let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
        &context.command_buffer_allocator,
        context.graphics_queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    ).unwrap();
    command_buffer_builder
        .set_viewport(0, [viewport])
        .begin_render_pass(RenderPassBeginInfo {
            clear_values: vec![Some(render_manager.clear_color.to_array().into()), Some(1.0.into())],
            ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
        }, SubpassContents::Inline)
        .unwrap();

    let projection_view = camera.projection_view(&info.window_size);
    let push_constants = vs::PushConstants { projection_view: projection_view.to_cols_array_2d() };

    draw_points(&canvas.points, canvas_context.pipeline_point.clone(), context.memory_allocator.clone(), &mut command_buffer_builder, push_constants);
    draw_lines(&canvas.lines, canvas_context.pipeline_line.clone(), context.memory_allocator.clone(), &mut command_buffer_builder, push_constants);
    draw_triangles(&canvas.triangles, canvas_context.pipeline_triangle.clone(), context.memory_allocator.clone(), &mut command_buffer_builder, push_constants);
    draw_rectangles(&canvas.rectangles, canvas_context.pipeline_triangle.clone(), context.memory_allocator.clone(), &mut command_buffer_builder, push_constants);
    draw_circles(&canvas.cicles, canvas_context.pipeline_circle.clone(), context.memory_allocator.clone(), &mut command_buffer_builder, &projection_view);

    command_buffer_builder.end_render_pass().unwrap();
    command_buffer_builder.build().unwrap()
}

fn draw_points(points: &Vec<(Vec3, Vec4)>, pipeline: Arc<GraphicsPipeline>, memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, push_constants: vs::PushConstants) {
    if points.is_empty() {
        return;
    }

    let vertices = points.iter()
        .map(|(v, c)| MyVertex { position: v.to_array(), color: c.to_array() })
        .collect::<Vec<_>>();

    draw_vertices(vertices, pipeline, memory_allocator, command_buffer_builder, push_constants);
}

fn draw_lines(lines: &Vec<[(Vec3, Vec4); 2]>, pipeline: Arc<GraphicsPipeline>, memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, push_constants: vs::PushConstants) {
    if lines.is_empty() {
        return;
    }

    let vertices = lines.iter()
        .flatten()
        .map(|(v, c)| MyVertex { position: v.to_array(), color: c.to_array() })
        .collect::<Vec<_>>();

    draw_vertices(vertices, pipeline, memory_allocator, command_buffer_builder, push_constants);
}

fn draw_triangles(triangles: &Vec<[(Vec3, Vec4); 3]>, pipeline: Arc<GraphicsPipeline>, memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, push_constants: vs::PushConstants) {
    if triangles.is_empty() {
        return;
    }

    let vertices = triangles.iter()
        .flatten()
        .map(|(v, c)| MyVertex { position: v.to_array(), color: c.to_array() })
        .collect::<Vec<_>>();

    draw_vertices(vertices, pipeline, memory_allocator, command_buffer_builder, push_constants);
}

fn draw_vertices(vertices: Vec<MyVertex>, pipeline: Arc<GraphicsPipeline>, memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, push_constants: vs::PushConstants) {
    let vertex_buffer = vertex_buffer(vertices, &memory_allocator);

    command_buffer_builder
        .bind_pipeline_graphics(pipeline.clone())
        .push_constants(pipeline.layout().clone(), 0, push_constants)
        .bind_vertex_buffers(0, vertex_buffer.clone())
        .draw(vertex_buffer.len() as u32, 1, 0, 0)
        .unwrap();
}

fn draw_rectangles(rectangles: &Vec<[(Vec3, Vec4); 4]>, pipeline: Arc<GraphicsPipeline>, memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, push_constants: vs::PushConstants) {
    if rectangles.is_empty() {
        return;
    }

    let vertices = rectangles.iter()
        .flatten()
        .map(|(v, c)| MyVertex { position: v.to_array(), color: c.to_array() })
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
        .bind_pipeline_graphics(pipeline.clone())
        .push_constants(pipeline.layout().clone(), 0, push_constants)
        .bind_vertex_buffers(0, vertex_buffer.clone())
        .bind_index_buffer(index_buffer.clone())
        .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
        .unwrap();
}

fn draw_circles(cicles: &Vec<(Vec3, Quat, f32, Vec4)>, pipeline: Arc<GraphicsPipeline>, memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, projection_view: &Mat4) {
    command_buffer_builder.bind_pipeline_graphics(pipeline.clone());
    for (center, rotation, radius, color) in cicles {
        let radius = *radius;
        let push_constants = circle::vs::PushConstants {
            projection_view: projection_view.to_cols_array_2d(), center: center.to_array(), radius };

        let model = Mat4::from_rotation_translation(*rotation, *center);
        let vertex_buffer = vertex_buffer([
            ((model * Vec4::new(-radius, -radius, 0.0, 1.0)).xyz(), color),
            ((model * Vec4::new(-radius, radius, 0.0, 1.0)).xyz(), color),
            ((model * Vec4::new(radius, radius, 0.0, 1.0)).xyz(), color),
            ((model * Vec4::new(radius, -radius, 0.0, 1.0)).xyz(), color)
        ].map(|(v, c)| MyVertex { position: v.to_array(), color: c.to_array() }).to_vec(), &memory_allocator);
        let index_buffer = index_buffer(vec![0u16, 1, 2, 2, 3, 0], &memory_allocator);

        command_buffer_builder
            .push_constants(pipeline.layout().clone(), 0, push_constants)
            .bind_vertex_buffers(0, vertex_buffer)
            .bind_index_buffer(index_buffer.clone())
            .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
            .unwrap();
    }
}

fn vertex_buffer(vertices: Vec<MyVertex>, memory_allocator: &Arc<StandardMemoryAllocator>) -> Subbuffer<[MyVertex]> {
    Buffer::from_iter(
        memory_allocator.as_ref(),
        BufferCreateInfo { usage: BufferUsage::VERTEX_BUFFER, ..Default::default() },
        AllocationCreateInfo { usage: MemoryUsage::Upload, ..Default::default() },
        vertices.into_iter()
    ).unwrap()
}

fn index_buffer(indices: Vec<u16>, memory_allocator: &Arc<StandardMemoryAllocator>) -> Subbuffer<[u16]> {
    Buffer::from_iter(
        memory_allocator.as_ref(),
        BufferCreateInfo { usage: BufferUsage::INDEX_BUFFER, ..Default::default() },
        AllocationCreateInfo { usage: MemoryUsage::Upload, ..Default::default() },
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
