use crate::{camera::{Camera, CameraInfo}, physics2d::{Collider2D, Physics2DManager, RigidBody2D}, render::{canvas::{Canvas, RenderInfo}, renderer2d::Renderer2D}, ComponentFn, DrawInfo, Engine, EntityInfo, Transform, WorldData, WorldDataExt, WorldExt};
use indexmap::IndexMap;
use shipyard::{UniqueViewMut, World};
use rapier2d::prelude::*;
use glam::{Quat, Vec3};
use steel_common::{Command, EditorCamera};
use vulkano::{sync::GpuFuture, command_buffer::PrimaryCommandBufferAbstract};

pub struct EngineImpl {
    pub world: World, // ecs world, also contains resources and managers
    pub component_fn: IndexMap<&'static str, ComponentFn>,
}

impl EngineImpl {
    pub fn new() -> Self {
        EngineImpl { world: World::new(), component_fn: ComponentFn::with_core_components() }
    }
}

impl Engine for EngineImpl {
    fn init(&mut self, world_data: Option<&WorldData>) {
        log::debug!("Engine::init");
        self.world.add_unique(Physics2DManager::new()); // TODO: load unique from world_data
        self.world.add_unique(CameraInfo::new());
        self.world.add_unique(Canvas::new());

        if let Some(world_data) = world_data { // load from world_data
            self.reload(world_data);
        } else { // create empty scene
            self.world.add_entity((EntityInfo::new("Camera"),
                    Transform::default(),
                    Camera { height: 20.0 }));
            self.world.add_entity((EntityInfo::new("Object"),
                    Transform { position: Vec3 { x: 0.0, y: 10.0, z: 0.0 }, rotation: Quat::IDENTITY, scale: Vec3::ONE },
                    RigidBody2D::new(RigidBodyType::Dynamic),
                    Collider2D::new(SharedShape::cuboid(0.5, 0.5), 0.7),
                    Renderer2D::default()));
            self.world.add_entity((EntityInfo::new("Ground"),
                    Transform { position: Vec3 { x: 0.0, y: 0.0, z: 0.0 }, rotation: Quat::IDENTITY, scale: Vec3 { x: 20.0, y: 0.2, z: 1.0 } },
                    Collider2D::new(SharedShape::cuboid(10.0, 0.1), 0.7),
                    Renderer2D::default()));
        }
    }

    fn maintain(&mut self) {
        self.world.run(crate::render::canvas::canvas_clear_system);
        self.world.run(crate::camera::camera_maintain_system);
        self.world.run(crate::physics2d::physics2d_maintain_system);
    }

    fn update(&mut self) {
        log::trace!("Engine::update");
        self.world.run(crate::physics2d::physics2d_update_system);
    }

    fn draw(&mut self) {
        self.world.run(crate::render::renderer2d::renderer2d_to_canvas_system);
    }

    fn draw_game(&mut self, info: DrawInfo) -> Box<dyn GpuFuture> {
        log::trace!("Engine::draw");
        self.world.add_unique(RenderInfo::from(&info));
        let command_buffer = self.world.run(crate::render::canvas::canvas_render_system);
        self.world.remove_unique::<RenderInfo>().unwrap();
        command_buffer.execute_after(info.before_future, info.context.graphics_queue().clone()).unwrap().boxed()
    }

    fn draw_editor(&mut self, info: DrawInfo, camera: &EditorCamera) -> Box<dyn GpuFuture> {
        self.world.run(|mut camera_info: UniqueViewMut<CameraInfo>| {
            camera_info.set(camera);
        });
        self.world.run(crate::physics2d::physics2d_debug_render_system);
        self.draw_game(info)
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

    fn command(&mut self, cmd: Command) {
        match cmd {
            Command::CreateEntity => {
                self.world.add_entity((EntityInfo::new("New Entity"),
                    Transform::default(),
                    Renderer2D::default()));
            },
            Command::DestroyEntity(id) => {
                self.world.delete_entity(id);
            },
            Command::ClearEntity => {
                self.world.clear();
            },
            Command::GetComponents(components) => {
                *components = self.component_fn.keys().map(|s| *s).collect(); // TODO: cache components
            },
            Command::CreateComponent(id, component_name) => {
                if let Some(component_fn) = self.component_fn.get(component_name) {
                    (component_fn.create)(&mut self.world, id);
                }
            },
            Command::DestroyComponent(id, component_name) => {
                if let Some(component_fn) = self.component_fn.get(component_name.as_str()) {
                    (component_fn.destroy)(&mut self.world, id);
                }
            },
        }
    }
}
