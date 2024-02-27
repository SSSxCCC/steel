use std::sync::Arc;
use vulkano::{buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage}, command_buffer::{allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassContents}, device::{Device, Queue}, format::Format, image::ImageViewAbstract, memory::allocator::{AllocationCreateInfo, MemoryUsage, StandardMemoryAllocator}, pipeline::{graphics::{input_assembly::InputAssemblyState, rasterization::{PolygonMode, RasterizationState}, vertex_input::Vertex, viewport::{Viewport, ViewportState}}, GraphicsPipeline, Pipeline}, render_pass::{Framebuffer, FramebufferCreateInfo, Subpass}};
use shipyard::{Component, IntoIter, Unique, UniqueView, UniqueViewMut, View};
use glam::{Mat4, Quat, Vec2, Vec3, Vec4, Vec4Swizzles};
use crate::{camera::CameraInfo, Edit, Transform2D};

#[derive(Component, Default, Debug)]
pub struct Renderer2D; // can only render cuboid currently. TODO: render multiple shape

impl Edit for Renderer2D {
    fn name() -> &'static str { "Renderer2D" }
}

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
    pub points: Vec<Vec3>,
    pub lines: Vec<[Vec3; 2]>,
    pub triangles: Vec<[Vec3; 3]>,
    /// the index buffer: [0, 1, 2, 2, 3, 0]
    pub rectangles: Vec<[Vec3; 4]>,
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

pub fn renderer2d_to_canvas_system(renderer2d: View<Renderer2D>, transform2d: View<Transform2D>, mut canvas: UniqueViewMut<Canvas>) {
    for (transform2d, renderer2d) in (&transform2d, &renderer2d).iter() {
        let model = Mat4::from_scale_rotation_translation(Vec3 { x: transform2d.scale.x, y: transform2d.scale.y, z: 1.0 },
            Quat::from_axis_angle(Vec3::Z, transform2d.rotation), transform2d.position);
        let vertex = [(model * Vec4::new(-0.5, -0.5, 0.0, 1.0)).xyz(),
            (model * Vec4::new(-0.5, 0.5, 0.0, 1.0)).xyz(),
            (model * Vec4::new(0.5, 0.5, 0.0, 1.0)).xyz(),
            (model * Vec4::new(0.5, -0.5, 0.0, 1.0)).xyz()];
        canvas.rectangles.push(vertex);
    }
}

pub fn canvas_render_system(info: UniqueView<RenderInfo>, camera: UniqueView<CameraInfo>, mut canvas: UniqueViewMut<Canvas>) -> PrimaryAutoCommandBuffer {
    let render_pass = vulkano::single_pass_renderpass!(
        info.device.clone(),
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: info.format, // set the format the same as the swapchain
                samples: 1,
            },
        },
        pass: {
            color: [color],
            depth_stencil: {},
        },
    )
    .unwrap();

    let framebuffer = Framebuffer::new(
        render_pass.clone(),
        FramebufferCreateInfo {
            attachments: vec![info.image.clone()],
            ..Default::default()
        },
    )
    .unwrap();

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
    )
    .unwrap();

    command_buffer_builder.begin_render_pass(
        RenderPassBeginInfo {
            clear_values: vec![Some([0.0, 0.0, 1.0, 1.0].into())],
            ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
        },
        SubpassContents::Inline,
    )
    .unwrap();

    let projection_view = camera.projection_view(&info.window_size);
    let model = Mat4::IDENTITY; // TODO: remove this
    let push_constants = vs::PushConstants { projection_view: projection_view.to_cols_array_2d(), model: model.to_cols_array_2d() };

    let pipeline_builder = GraphicsPipeline::start()
        .vertex_input_state(MyVertex::per_vertex())
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .input_assembly_state(InputAssemblyState::new())
        .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        .render_pass(Subpass::from(render_pass, 0).unwrap());

    if !canvas.lines.is_empty() {
        let pipeline = pipeline_builder.clone()
            .rasterization_state(RasterizationState::new().polygon_mode(PolygonMode::Line))
            .build(info.device.clone())
            .unwrap();
        command_buffer_builder.bind_pipeline_graphics(pipeline.clone());

        let vertices = canvas.lines.iter()
            .flatten()
            .map(|v| { MyVertex { position: [v.x, v.y] } })
            .collect::<Vec<_>>();

        let vertex_buffer = Buffer::from_iter(
            info.memory_allocator.as_ref(),
            BufferCreateInfo { usage: BufferUsage::VERTEX_BUFFER, ..Default::default() },
            AllocationCreateInfo { usage: MemoryUsage::Upload, ..Default::default() },
            vertices.into_iter()
        ).unwrap();

        command_buffer_builder
            .push_constants(pipeline.layout().clone(), 0, push_constants)
            .bind_vertex_buffers(0, vertex_buffer.clone())
            .draw(vertex_buffer.len() as u32, 1, 0, 0)
            .unwrap();
    }

    if !canvas.rectangles.is_empty() {
        let pipeline = pipeline_builder.clone()
            .build(info.device.clone())
            .unwrap();
        command_buffer_builder.bind_pipeline_graphics(pipeline.clone());

        let vertices = canvas.rectangles.iter()
            .flatten()
            .map(|v| { MyVertex { position: [v.x, v.y] } })
            .collect::<Vec<_>>();

        let indices = canvas.rectangles.iter()
            .enumerate()
            .map(|(i, _)| [i * 4, i * 4 + 1, i * 4 + 2, i * 4 + 2, i * 4 + 3, i * 4])
            .flatten()
            .map(|i| i as u16)
            .collect::<Vec<_>>();

        let vertex_buffer = Buffer::from_iter(
            info.memory_allocator.as_ref(),
            BufferCreateInfo { usage: BufferUsage::VERTEX_BUFFER, ..Default::default() },
            AllocationCreateInfo { usage: MemoryUsage::Upload, ..Default::default() },
            vertices.into_iter()
        ).unwrap();

        let index_buffer = Buffer::from_iter(
            info.memory_allocator.as_ref(),
            BufferCreateInfo { usage: BufferUsage::INDEX_BUFFER, ..Default::default() },
            AllocationCreateInfo { usage: MemoryUsage::Upload, ..Default::default() },
            indices.into_iter()
        ).unwrap();

        command_buffer_builder
            .push_constants(pipeline.layout().clone(), 0, push_constants)
            .bind_vertex_buffers(0, vertex_buffer.clone())
            .bind_index_buffer(index_buffer.clone())
            .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
            .unwrap();
    }

    command_buffer_builder.end_render_pass().unwrap();
    command_buffer_builder.build().unwrap()
}

#[derive(BufferContents, Vertex)]
#[repr(C)]
struct MyVertex {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2],
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

            layout(location = 0) in vec2 position;

            void main() {
                gl_Position = pcs.projection_view * pcs.model * vec4(position, 0.0, 1.0);
            }
        ",
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 460

            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = vec4(1.0, 0.0, 0.0, 1.0);
            }
        ",
    }
}
