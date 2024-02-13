use std::sync::Arc;
use vulkano::{format::Format, render_pass::{FramebufferCreateInfo, Framebuffer, Subpass}, buffer::{BufferContents, Buffer, BufferCreateInfo, BufferUsage}, pipeline::{graphics::{vertex_input::Vertex, viewport::{Viewport, ViewportState}, input_assembly::InputAssemblyState}, GraphicsPipeline, Pipeline}, memory::allocator::{AllocationCreateInfo, MemoryUsage, StandardMemoryAllocator}, command_buffer::{allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents, PrimaryAutoCommandBuffer}, device::{Device, Queue}, image::ImageViewAbstract};
use shipyard::{View, IntoIter, Unique, UniqueView, Component};
use glam::{Vec2, Vec3, Mat4, Quat};
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

pub fn render2d_system(info: UniqueView<RenderInfo>, camera: UniqueView<CameraInfo>, transform2d: View<Transform2D>, renderer2d: View<Renderer2D>) -> PrimaryAutoCommandBuffer {
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

    let pipeline = GraphicsPipeline::start()
        .vertex_input_state(MyVertex::per_vertex())
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .input_assembly_state(InputAssemblyState::new())
        .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        .render_pass(Subpass::from(render_pass, 0).unwrap())
        .build(info.device.clone())
        .unwrap();

    let command_buffer_allocator = StandardCommandBufferAllocator::new(info.device.clone(), Default::default());

    let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
        &command_buffer_allocator,
        info.graphics_queue.queue_family_index(),
        CommandBufferUsage::MultipleSubmit,
    )
    .unwrap();

    command_buffer_builder
        .begin_render_pass(
            RenderPassBeginInfo {
                clear_values: vec![Some([0.0, 0.0, 1.0, 1.0].into())],
                ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
            },
            SubpassContents::Inline,
        )
        .unwrap()
        .bind_pipeline_graphics(pipeline.clone());

    let projection_view = camera.projection_view(&info.window_size);

    for (transform2d, renderer2d) in (&transform2d, &renderer2d).iter() {
        let model = Mat4::from_scale_rotation_translation(Vec3 { x: transform2d.scale.x, y: transform2d.scale.y, z: 1.0 },
                Quat::from_axis_angle(Vec3::Z, transform2d.rotation), transform2d.position);

        let push_constants = vs::PushConstants { projection_view: projection_view.to_cols_array_2d(), model: model.to_cols_array_2d() };

        let vertex1 = MyVertex {
            position: [-0.5, -0.5],
        };
        let vertex2 = MyVertex {
            position: [-0.5, 0.5],
        };
        let vertex3 = MyVertex {
            position: [0.5, 0.5],
        };
        let vertex4 = MyVertex {
            position: [0.5, -0.5],
        };
        let vertex_buffer = Buffer::from_iter(
            info.memory_allocator.as_ref(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                usage: MemoryUsage::Upload,
                ..Default::default()
            },
            vec![vertex1, vertex2, vertex3, vertex4].into_iter(),
        )
        .unwrap();

        let index_buffer = Buffer::from_iter(
            &info.memory_allocator,
            BufferCreateInfo { usage: BufferUsage::INDEX_BUFFER, ..Default::default() },
            AllocationCreateInfo { usage: MemoryUsage::Upload, ..Default::default() },
            vec![0u16, 1, 2, 2, 3, 0].into_iter()).unwrap();

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
