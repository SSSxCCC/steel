pub use steel_common::engine::*;

use std::collections::HashMap;
use shipyard::{track::{All, Insertion, Modification, Removal, Untracked}, UniqueViewMut, World};
use vulkano::{sync::GpuFuture, command_buffer::PrimaryCommandBufferAbstract};
use crate::{camera::CameraInfo, data::{ComponentFn, ComponentFns}, edit::Edit, entityinfo::EntityInfo, physics2d::Physics2DManager, render::{canvas::{Canvas, RenderInfo}, renderer2d::Renderer2D}, transform::Transform};

pub struct EngineImpl {
    pub world: World, // ecs world, also contains resources and managers
    pub component_fns: ComponentFns,
}

impl EngineImpl {
    pub fn new() -> Self {
        EngineImpl { world: World::new(), component_fns: ComponentFn::with_core_components() }
    }

    /// Register a component type with <Tracking = Untracked> so that this component can be edited in steel-editor.
    /// Currently we must write different generic functions for different tracking type, see ComponentFn::load_from_data_untracked_fn
    pub fn register<T: Edit<Tracking = Untracked> + Send + Sync>(&mut self) {
        ComponentFn::register::<T>(&mut self.component_fns);
    }

    /// Register a component type with <Tracking = Insertion> so that this component can be edited in steel-editor.
    pub fn register_track_insertion<T: Edit<Tracking = Insertion> + Send + Sync>(&mut self) {
        ComponentFn::register_track_insertion::<T>(&mut self.component_fns);
    }

    /// Register a component type with <Tracking = Modification> so that this component can be edited in steel-editor.
    pub fn register_track_modification<T: Edit<Tracking = Modification> + Send + Sync>(&mut self) {
        ComponentFn::register_track_modification::<T>(&mut self.component_fns);
    }

    /// Register a component type with <Tracking = Removal> so that this component can be edited in steel-editor.
    pub fn register_track_removal<T: Edit<Tracking = Removal> + Send + Sync>(&mut self) {
        ComponentFn::register_track_removal::<T>(&mut self.component_fns);
    }

    /// Register a component type with <Tracking = All> so that this component can be edited in steel-editor.
    pub fn register_track_all<T: Edit<Tracking = All> + Send + Sync>(&mut self) {
        ComponentFn::register_track_all::<T>(&mut self.component_fns);
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
                world_data.clear();
                for component_fn in self.component_fns.values() {
                    (component_fn.save_to_data)(world_data, &self.world);
                }
            },
            Command::Load(world_data) => {
                for component_fn in self.component_fns.values() {
                    (component_fn.load_from_data)(&mut self.world, world_data);
                }
            },
            Command::Relaod(world_data) => {
                self.world.clear();
                let mut old_id_to_new_id = HashMap::new();
                for (old_id, entity_data) in &world_data.entities {
                    let new_id = *old_id_to_new_id.entry(old_id).or_insert_with(|| self.world.add_entity(()));
                    for (component_name, component_data) in &entity_data.components {
                        if let Some(component_fn) = self.component_fns.get(component_name.as_str()) {
                            (component_fn.create_with_data)(&mut self.world, new_id, component_data);
                        }
                    }
                }
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
                *components = self.component_fns.keys().map(|s| *s).collect(); // TODO: cache components
            },
            Command::CreateComponent(id, component_name) => {
                if let Some(component_fn) = self.component_fns.get(component_name) {
                    (component_fn.create)(&mut self.world, id);
                }
            },
            Command::DestroyComponent(id, component_name) => {
                if let Some(component_fn) = self.component_fns.get(component_name.as_str()) {
                    (component_fn.destroy)(&mut self.world, id);
                }
            },
        }
    }
}
