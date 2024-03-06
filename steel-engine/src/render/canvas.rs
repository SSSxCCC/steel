use std::sync::Arc;
use glam::{Mat4, Vec2, Vec3, Vec4};
use vulkano::{buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage}, command_buffer::{allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassContents}, device::{Device, Queue}, format::Format, image::{ImageUsage, ImageViewAbstract, StorageImage}, memory::allocator::{AllocationCreateInfo, MemoryUsage, StandardMemoryAllocator}, pipeline::{graphics::{color_blend::ColorBlendState, depth_stencil::DepthStencilState, input_assembly::{InputAssemblyState, PrimitiveTopology}, rasterization::{PolygonMode, RasterizationState}, vertex_input::Vertex, viewport::{Viewport, ViewportState}}, GraphicsPipeline, Pipeline}, render_pass::{Framebuffer, FramebufferCreateInfo, Subpass}};
use shipyard::{Unique, UniqueView, UniqueViewMut};
use crate::camera::CameraInfo;

#[derive(Unique)]
pub struct RenderInfo {
    device: Arc<Device>,
    graphics_queue: Arc<Queue>,
    memory_allocator: Arc<StandardMemoryAllocator>,

    window_size: Vec2,
    image: Arc<dyn ImageViewAbstract>, // the image we will draw
    format: Format,
}

impl RenderInfo {
    pub fn new(device: Arc<Device>, graphics_queue: Arc<Queue>, memory_allocator: Arc<StandardMemoryAllocator>, window_size: Vec2, image: Arc<dyn ImageViewAbstract>, format: Format) -> Self {
        RenderInfo { device, graphics_queue, memory_allocator, window_size, image, format }
    }
}

#[derive(Unique)]
pub struct Canvas {
    pub points: Vec<(Vec3, Vec4)>,
    pub lines: Vec<[(Vec3, Vec4); 2]>,
    pub triangles: Vec<[(Vec3, Vec4); 3]>,
    /// the index buffer: [0, 1, 2, 2, 3, 0]
    pub rectangles: Vec<[(Vec3, Vec4); 4]>,
}

impl Canvas {
    pub fn new() -> Self {
        Canvas { points: Vec::new(), lines: Vec::new(), triangles: Vec::new(), rectangles: Vec::new() }
    }

    pub fn clear(&mut self) {
        self.points.clear();
        self.lines.clear();
        self.triangles.clear();
        self.rectangles.clear();
    }
}

pub fn canvas_clear_system(mut canvas: UniqueViewMut<Canvas>) {
    canvas.clear();
}

pub fn canvas_render_system(info: UniqueView<RenderInfo>, camera: UniqueView<CameraInfo>, canvas: UniqueView<Canvas>) -> PrimaryAutoCommandBuffer {
    let depth_stencil_image = StorageImage::general_purpose_image_view(
        info.memory_allocator.as_ref(),
        info.graphics_queue.clone(),
        [info.window_size.x as u32, info.window_size.y as u32],
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

    let vs = vs::load(info.device.clone()).expect("failed to create shader module");
    let fs = fs::load(info.device.clone()).expect("failed to create shader module");

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: info.window_size.into(),
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
            clear_values: vec![Some([0.0, 0.0, 1.0, 1.0].into()), Some(1.0.into())],
            ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
        },
        SubpassContents::Inline,
    ).unwrap();

    let projection_view = camera.projection_view(&info.window_size);
    let model = Mat4::IDENTITY; // TODO: remove this
    let push_constants = vs::PushConstants { projection_view: projection_view.to_cols_array_2d(), model: model.to_cols_array_2d() };

    let pipeline_builder = GraphicsPipeline::start()
        .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
        .vertex_input_state(MyVertex::per_vertex())
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        .render_pass(Subpass::from(render_pass, 0).unwrap())
        .depth_stencil_state(DepthStencilState::simple_depth_test())
        .color_blend_state(ColorBlendState::default().blend_alpha()); // TODO: implement order independent transparency

    let pipeline = pipeline_builder.clone()
        .input_assembly_state(InputAssemblyState::new().topology(PrimitiveTopology::LineList))
        .rasterization_state(RasterizationState::new().polygon_mode(PolygonMode::Line))
        .build(info.device.clone())
        .unwrap();
    draw_lines(&canvas.lines, pipeline, info.memory_allocator.clone(), &mut command_buffer_builder, push_constants);

    let pipeline = pipeline_builder.clone()
        .build(info.device.clone())
        .unwrap();
    draw_rectangles(&canvas.rectangles, pipeline, info.memory_allocator.clone(), &mut command_buffer_builder, push_constants);

    command_buffer_builder.end_render_pass().unwrap();
    command_buffer_builder.build().unwrap()
}

fn draw_lines(lines: &Vec<[(Vec3, Vec4); 2]>, pipeline: Arc<GraphicsPipeline>, memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, push_constants: vs::PushConstants) {
    if lines.is_empty() {
        return;
    }

    let vertices = lines.iter()
        .flatten()
        .map(|(v, c)| { MyVertex { position: v.to_array(), color: c.to_array() } })
        .collect::<Vec<_>>();

    let vertex_buffer = Buffer::from_iter(
        memory_allocator.as_ref(),
        BufferCreateInfo { usage: BufferUsage::VERTEX_BUFFER, ..Default::default() },
        AllocationCreateInfo { usage: MemoryUsage::Upload, ..Default::default() },
        vertices.into_iter()
    ).unwrap();

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
        .map(|(v, c)| { MyVertex { position: v.to_array(), color: c.to_array() } })
        .collect::<Vec<_>>();

    let indices = rectangles.iter()
        .enumerate()
        .map(|(i, _)| [i * 4, i * 4 + 1, i * 4 + 2, i * 4 + 2, i * 4 + 3, i * 4])
        .flatten()
        .map(|i| i as u16)
        .collect::<Vec<_>>();

    let vertex_buffer = Buffer::from_iter(
        memory_allocator.as_ref(),
        BufferCreateInfo { usage: BufferUsage::VERTEX_BUFFER, ..Default::default() },
        AllocationCreateInfo { usage: MemoryUsage::Upload, ..Default::default() },
        vertices.into_iter()
    ).unwrap();

    let index_buffer = Buffer::from_iter(
        memory_allocator.as_ref(),
        BufferCreateInfo { usage: BufferUsage::INDEX_BUFFER, ..Default::default() },
        AllocationCreateInfo { usage: MemoryUsage::Upload, ..Default::default() },
        indices.into_iter()
    ).unwrap();

    command_buffer_builder
        .bind_pipeline_graphics(pipeline.clone())
        .push_constants(pipeline.layout().clone(), 0, push_constants)
        .bind_vertex_buffers(0, vertex_buffer.clone())
        .bind_index_buffer(index_buffer.clone())
        .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
        .unwrap();
}

#[derive(BufferContents, Vertex)]
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
                mat4 model;
            } pcs;

            layout(location = 0) in vec3 position;
            layout(location = 1) in vec4 color;

            layout(location = 0) out vec4 out_color;

            void main() {
                gl_Position = pcs.projection_view * pcs.model * vec4(position, 1.0);
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
