use crate::{camera::{camera_maintain_system, Camera, CameraInfo}, physics2d::{physics2d_maintain_system, physics2d_update_system, Collider2D, Physics2DManager, RigidBody2D}, render2d::{render2d_system, RenderInfo, Renderer2D}, DrawInfo, Engine, EntityInfo, Transform2D, WorldData, WorldDataExt, WorldExt};
use shipyard::{UniqueViewMut, World};
use rapier2d::prelude::*;
use glam::{Vec2, Vec3};
use steel_common::EditorCamera;
use vulkano::{sync::GpuFuture, command_buffer::PrimaryCommandBufferAbstract};

pub struct EngineImpl {
    pub world: World, // ecs world, also contains resources and managers
}

impl EngineImpl {
    pub fn new() -> Self {
        EngineImpl { world: World::new() }
    }
}

impl Engine for EngineImpl {
    fn init(&mut self, world_data: Option<&WorldData>) {
        log::debug!("Engine::init");
        self.world.add_unique(Physics2DManager::new()); // TODO: load unique from world_data
        self.world.add_unique(CameraInfo::new());

        if let Some(world_data) = world_data { // load from world_data
            self.reload(world_data);
        } else { // create empty scene
            self.world.add_entity((EntityInfo::new("Camera"),
                    Transform2D::default(),
                    Camera { height: 20.0 }));
            self.world.add_entity((EntityInfo::new("Object"),
                    Transform2D { position: Vec3 { x: 0.0, y: 10.0, z: 0.0 }, rotation: 0.0, scale: Vec2::ONE },
                    RigidBody2D::new(RigidBodyType::Dynamic),
                    Collider2D::new(SharedShape::cuboid(0.5, 0.5), 0.7),
                    Renderer2D));
            self.world.add_entity((EntityInfo::new("Ground"),
                    Transform2D { position: Vec3 { x: 0.0, y: 0.0, z: 0.0 }, rotation: 0.0, scale: Vec2 { x: 20.0, y: 0.2 } },
                    Collider2D::new(SharedShape::cuboid(10.0, 0.1), 0.7),
                    Renderer2D));
        }
    }

    fn maintain(&mut self) {
        self.world.run(camera_maintain_system);
        self.world.run(physics2d_maintain_system);
    }

    fn update(&mut self) {
        log::trace!("Engine::update");
        self.world.run(physics2d_update_system);
    }

    fn draw(&mut self, info: DrawInfo) -> Box<dyn GpuFuture> {
        log::trace!("Engine::draw");
        self.world.add_unique(RenderInfo::new(info.context.device().clone(),
            info.context.graphics_queue().clone(), info.context.memory_allocator().clone(),
            info.window_size, info.image, info.renderer.swapchain_format()));
        let command_buffer = self.world.run(render2d_system);
        self.world.remove_unique::<RenderInfo>().unwrap();
        command_buffer.execute_after(info.before_future, info.context.graphics_queue().clone()).unwrap().boxed()
    }

    fn draw_editor(&mut self, info: DrawInfo, camera: &EditorCamera) -> Box<dyn GpuFuture> {
        self.world.run(|mut camera_info: UniqueViewMut<CameraInfo>| {
            camera_info.set(camera);
        });
        self.draw(info)
    }

    fn save(&self) -> WorldData {
        log::trace!("Engine::save");
        WorldData::with_core_components(&self.world)
    }

    fn load(&mut self, world_data: &WorldData) {
        log::trace!("Engine::load");
        self.world.load_core_components(world_data);
    }

    fn reload(&mut self, world_data: &WorldData) {
        log::debug!("Engine::reload");
        self.world.recreate_core_components(world_data);
    }
}
