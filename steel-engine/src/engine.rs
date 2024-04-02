pub use steel_common::engine::*;

use indexmap::IndexMap;
use shipyard::{UniqueViewMut, World};
use steel_common::data::WorldData;
use vulkano::{sync::GpuFuture, command_buffer::PrimaryCommandBufferAbstract};
use crate::{camera::CameraInfo, physics2d::Physics2DManager, render::{canvas::{Canvas, RenderInfo}, renderer2d::Renderer2D}, entityinfo::EntityInfo, transform::Transform, data::{WorldDataExt, WorldExt, ComponentFn}};

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
    fn init(&mut self) {
        self.world.add_unique(Physics2DManager::new()); // TODO: load unique from world_data
        self.world.add_unique(CameraInfo::new());
        self.world.add_unique(Canvas::new());
    }

    fn maintain(&mut self) {
        self.world.run(crate::render::canvas::canvas_clear_system);
        self.world.run(crate::camera::camera_maintain_system);
        self.world.run(crate::physics2d::physics2d_maintain_system);
    }

    fn update(&mut self) {
        self.world.run(crate::physics2d::physics2d_update_system);
    }

    fn draw(&mut self) {
        self.world.run(crate::render::renderer2d::renderer2d_to_canvas_system);
    }

    fn draw_game(&mut self, info: DrawInfo) -> Box<dyn GpuFuture> {
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

    fn command(&mut self, cmd: Command) {
        match cmd {
            Command::Save(world_data) => {
                *world_data = WorldData::with_core_components(&self.world);
            },
            Command::Load(world_data) => {
                self.world.load_core_components(world_data);
            },
            Command::Relaod(world_data) => {
                self.world.recreate_core_components(world_data);
            },
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
