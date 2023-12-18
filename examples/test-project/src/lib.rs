use log::{Log, LevelFilter, SetLoggerError};
use steel::{physics2d::{Physics2DManager, RigidBody2D, Collider2D, physics2d_maintain_system, physics2d_update_system}, Transform2D, add_component, Engine, WorldData, DrawInfo, render2d::{RenderInfo, render2d_system, Renderer2D}};
use shipyard::World;
use rapier2d::prelude::*;
use glam::{Vec2, Vec3};
use vulkano::{sync::GpuFuture, command_buffer::PrimaryCommandBufferAbstract};

#[no_mangle]
pub fn setup_logger(logger: &'static dyn Log, level: LevelFilter) -> Result<(), SetLoggerError> {
    log::set_max_level(level);
    log::set_logger(logger)
}

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
        add_component::<Transform2D>(&mut world_data, &self.world);
        add_component::<RigidBody2D>(&mut world_data, &self.world);
        add_component::<Collider2D>(&mut world_data, &self.world);
        log::info!("world_data={:?}", world_data);
    }

    fn draw(&mut self, info: DrawInfo) -> Box<dyn GpuFuture> {
        self.world.add_unique(RenderInfo::new(info.context.device().clone(),
            info.context.graphics_queue().clone(), info.context.memory_allocator().clone(),
            info.window_size, info.image, info.renderer.swapchain_format()));
        let command_buffer = self.world.run(render2d_system);
        self.world.remove_unique::<RenderInfo>().unwrap();
        command_buffer.execute_after(info.before_future, info.context.graphics_queue().clone()).unwrap().boxed()
    }
}
