use std::sync::Arc;
use glam::{Mat4, Quat, UVec2, Vec3, Vec4, Vec4Swizzles};
use steel_common::engine::DrawInfo;
use vulkano::{buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer}, command_buffer::{allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassContents}, device::{Device, Queue}, format::Format, image::{ImageUsage, ImageViewAbstract, StorageImage}, memory::allocator::{AllocationCreateInfo, MemoryUsage, StandardMemoryAllocator}, pipeline::{graphics::{color_blend::ColorBlendState, depth_stencil::DepthStencilState, input_assembly::{InputAssemblyState, PrimitiveTopology}, rasterization::{PolygonMode, RasterizationState}, vertex_input::Vertex, viewport::{Viewport, ViewportState}}, GraphicsPipeline, Pipeline}, render_pass::{Framebuffer, FramebufferCreateInfo, Subpass}};
use shipyard::{Unique, UniqueView, UniqueViewMut};
use crate::camera::CameraInfo;
use super::RenderManager;

#[derive(Unique)]
pub struct RenderInfo {
    device: Arc<Device>,
    graphics_queue: Arc<Queue>,
    memory_allocator: Arc<StandardMemoryAllocator>,

    window_size: UVec2,
    image: Arc<dyn ImageViewAbstract>, // the image we will draw
    format: Format,
}

impl RenderInfo {
    pub fn from(info: &DrawInfo) -> Self {
        RenderInfo {
            device: info.context.device().clone(),
            graphics_queue: info.context.graphics_queue().clone(),
            memory_allocator: info.context.memory_allocator().clone(),
            window_size: info.window_size,
            image: info.image.clone(),
            format: info.renderer.swapchain_format()
        }
    }
}

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

pub fn canvas_render_system(info: UniqueView<RenderInfo>, camera: UniqueView<CameraInfo>, canvas: UniqueView<Canvas>, render_manager: UniqueView<RenderManager>) -> PrimaryAutoCommandBuffer {
    let depth_stencil_image = StorageImage::general_purpose_image_view(
        info.memory_allocator.as_ref(),
        info.graphics_queue.clone(),
        info.window_size.to_array(),
        Format::D32_SFLOAT,
        ImageUsage::DEPTH_STENCIL_ATTACHMENT,
    ).unwrap();

    let render_pass = vulkano::single_pass_renderpass!(
        info.device.clone(),
        attachments: {
            color: { load: Clear, store: Store, format: info.format, samples: 1 },
            depth_stencil: { load: Clear, store: DontCare, format: Format::D32_SFLOAT, samples: 1 },
        },
        pass: {
            color: [ color ],
            depth_stencil: { depth_stencil },
        },
    ).unwrap();

    let framebuffer = Framebuffer::new(
        render_pass.clone(),
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

    let command_buffer_allocator = StandardCommandBufferAllocator::new(info.device.clone(), Default::default());

    let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
        &command_buffer_allocator,
        info.graphics_queue.queue_family_index(),
        CommandBufferUsage::MultipleSubmit,
    ).unwrap();

    command_buffer_builder.begin_render_pass(
        RenderPassBeginInfo {
            clear_values: vec![Some(render_manager.clear_color.to_array().into()), Some(1.0.into())],
            ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
        },
        SubpassContents::Inline,
    ).unwrap();

    let projection_view = camera.projection_view(&info.window_size);
    let push_constants = vs::PushConstants { projection_view: projection_view.to_cols_array_2d() };

    let base_pipeline_builder = GraphicsPipeline::start()
        .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
        .vertex_input_state(MyVertex::per_vertex())
        .render_pass(Subpass::from(render_pass, 0).unwrap())
        .depth_stencil_state(DepthStencilState::simple_depth_test())
        .color_blend_state(ColorBlendState::default().blend_alpha()); // TODO: implement order independent transparency

    let vs = vs::load(info.device.clone()).expect("failed to create shader module");
    let fs = fs::load(info.device.clone()).expect("failed to create shader module");
    let pipeline_builder = base_pipeline_builder.clone()
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .fragment_shader(fs.entry_point("main").unwrap(), ());

    let pipeline = pipeline_builder.clone()
        .input_assembly_state(InputAssemblyState::new().topology(PrimitiveTopology::PointList))
        .rasterization_state(RasterizationState::new().polygon_mode(PolygonMode::Point))
        .build(info.device.clone())
        .unwrap();
    draw_points(&canvas.points, pipeline, info.memory_allocator.clone(), &mut command_buffer_builder, push_constants);

    let pipeline = pipeline_builder.clone()
        .input_assembly_state(InputAssemblyState::new().topology(PrimitiveTopology::LineList))
        .rasterization_state(RasterizationState::new().polygon_mode(PolygonMode::Line))
        .build(info.device.clone())
        .unwrap();
    draw_lines(&canvas.lines, pipeline, info.memory_allocator.clone(), &mut command_buffer_builder, push_constants);

    let pipeline = pipeline_builder.clone()
        .build(info.device.clone())
        .unwrap();
    draw_triangles(&canvas.triangles, pipeline.clone(), info.memory_allocator.clone(), &mut command_buffer_builder, push_constants);
    draw_rectangles(&canvas.rectangles, pipeline, info.memory_allocator.clone(), &mut command_buffer_builder, push_constants);

    let vs = circle::vs::load(info.device.clone()).expect("failed to create shader module");
    let fs = circle::fs::load(info.device.clone()).expect("failed to create shader module");
    let pipeline = base_pipeline_builder.clone()
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        .build(info.device.clone())
        .unwrap();
    draw_circles(&canvas.cicles, pipeline, info.memory_allocator.clone(), &mut command_buffer_builder, &projection_view);

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
