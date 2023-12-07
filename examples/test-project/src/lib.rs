
#[no_mangle]
pub fn create() -> Box<dyn Engine> {
    let world = World::new();
    Box::new(EngineImpl { world })
}

struct EngineImpl {
    world: World, // ecs world, also contains resources and managers
}

impl Engine for EngineImpl {
    fn init(&mut self) {
        log::info!("Engine::init");

        self.world.add_unique(Physics2DManager::new());

        self.world.add_entity((Transform2D { position: Vec3 { x: 0.0, y: 10.0, z: 0.0 }, rotation: 0.0, scale: Vec2::ONE },
                RigidBody2D::new(RigidBodyType::Dynamic),
                Collider2D::new(SharedShape::cuboid(0.5, 0.5), 0.7),
                Renderer2D));
        self.world.add_entity((Transform2D { position: Vec3 { x: 0.0, y: 0.0, z: 0.0 }, rotation: 0.0, scale: Vec2 { x: 20.0, y: 0.2 } },
                Collider2D::new(SharedShape::cuboid(10.0, 0.1), 0.7),
                Renderer2D));
    }

    fn update(&mut self) {
        log::info!("Engine::update");

        self.world.run(physics2d_maintain_system);
        self.world.run(physics2d_update_system);

        let mut world_data = WorldData::new();
        world_data.add_component::<Transform2D>(&self.world);
        world_data.add_component::<RigidBody2D>(&self.world);
        world_data.add_component::<Collider2D>(&self.world);
        log::info!("world_data={:?}", world_data);
    }

    fn draw(&mut self, info: DrawInfo) -> Box<dyn GpuFuture> {
        self.world.run(|transform2d: View<Transform2D>, renderer2d: View<Renderer2D>| {
            let render_pass = vulkano::single_pass_renderpass!(
                info.context.device().clone(),
                attachments: {
                    color: {
                        load: Clear,
                        store: Store,
                        format: info.renderer.swapchain_format(), // set the format the same as the swapchain
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
                    attachments: vec![info.image],
                    ..Default::default()
                },
            )
            .unwrap();

            let vs = vs::load(info.context.device().clone()).expect("failed to create shader module");
            let fs = fs::load(info.context.device().clone()).expect("failed to create shader module");
        
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
                .build(info.context.device().clone())
                .unwrap();

            let command_buffer_allocator = StandardCommandBufferAllocator::new(info.context.device().clone(), Default::default());

            let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
                &command_buffer_allocator,
                info.renderer.graphics_queue().queue_family_index(),
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

            let camera_pos = Vec3::ZERO;
            let view = Mat4::look_at_lh(camera_pos, camera_pos + Vec3::NEG_Z, Vec3::Y);
            let half_height = 10.0;
            let half_width = half_height * info.window_size.x / info.window_size.y as f32;
            let projection = Mat4::orthographic_lh(half_width, -half_width, half_height, -half_height, -1000.0, 1000.0);

            for (transform2d, renderer2d) in (&transform2d, &renderer2d).iter() {
                let model = Mat4::from_scale_rotation_translation(Vec3 { x: transform2d.scale.x, y: transform2d.scale.y, z: 1.0 },
                        Quat::from_axis_angle(Vec3::Z, transform2d.rotation), transform2d.position);

                let push_constants = vs::PushConstants { projection_view: (projection * view).to_cols_array_2d(), model: model.to_cols_array_2d() };

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
                    info.context.memory_allocator().as_ref(),
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
                    info.context.memory_allocator(),
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
            let command_buffer = command_buffer_builder.build().unwrap();
            command_buffer.execute_after(info.before_future, info.renderer.graphics_queue()).unwrap().boxed()
        })
    }
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

