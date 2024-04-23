pub use steel_common::engine::*;

use shipyard::{track::{All, Insertion, Modification, Removal, Untracked}, Component, Unique, UniqueViewMut, World};
use vulkano::{sync::GpuFuture, command_buffer::PrimaryCommandBufferAbstract};
use crate::{camera::CameraInfo, data::{ComponentFn, ComponentFns, UniqueFn, UniqueFns}, edit::Edit, entityinfo::EntityInfo, physics2d::Physics2DManager, render::{canvas::{Canvas, GetEntityAtScreenParam}, renderer2d::Renderer2D, FrameRenderInfo, RenderManager}, scene::SceneManager, transform::Transform};

pub struct EngineImpl {
    /// ecs world, contains entities, components and uniques
    pub world: World,
    /// registered components
    pub component_fns: ComponentFns,
    /// registered uniques
    pub unique_fns: UniqueFns,
}

impl EngineImpl {
    pub fn new() -> Self {
        EngineImpl { world: World::new(), component_fns: ComponentFn::with_core_components(), unique_fns: UniqueFn::with_core_uniques() }
    }

    /// Register a component type with <Tracking = Untracked> so that this component can be edited in steel-editor.
    /// Currently we must write different generic functions for different tracking type, see ComponentFn::load_from_data_untracked_fn
    pub fn register_component<C: Component<Tracking = Untracked> + Edit + Default + Send + Sync>(&mut self) {
        ComponentFn::register::<C>(&mut self.component_fns);
    }

    /// Register a component type with <Tracking = Insertion> so that this component can be edited in steel-editor.
    pub fn register_component_track_insertion<C: Component<Tracking = Insertion> + Edit + Default + Send + Sync>(&mut self) {
        ComponentFn::register_track_insertion::<C>(&mut self.component_fns);
    }

    /// Register a component type with <Tracking = Modification> so that this component can be edited in steel-editor.
    pub fn register_component_track_modification<C: Component<Tracking = Modification> + Edit + Default + Send + Sync>(&mut self) {
        ComponentFn::register_track_modification::<C>(&mut self.component_fns);
    }

    /// Register a component type with <Tracking = Removal> so that this component can be edited in steel-editor.
    pub fn register_component_track_removal<C: Component<Tracking = Removal> + Edit + Default + Send + Sync>(&mut self) {
        ComponentFn::register_track_removal::<C>(&mut self.component_fns);
    }

    /// Register a component type with <Tracking = All> so that this component can be edited in steel-editor.
    pub fn register_component_track_all<C: Component<Tracking = All> + Edit + Default + Send + Sync>(&mut self) {
        ComponentFn::register_track_all::<C>(&mut self.component_fns);
    }

    /// Register a unique type so that this unique can be edited in steel-editor.
    pub fn register_unique<U: Unique + Edit + Send + Sync>(&mut self) {
        UniqueFn::register::<U>(&mut self.unique_fns);
    }
}

impl EngineImpl {
    pub fn maintain(&mut self, _info: &FrameInfo) {
        SceneManager::maintain_system(&mut self.world, &self.component_fns, &self.unique_fns);
        self.world.run(crate::render::canvas::canvas_clear_system);
        self.world.run(crate::camera::camera_maintain_system);
        self.world.run(crate::physics2d::physics2d_maintain_system);
    }

    pub fn update(&mut self, _info: &FrameInfo) {
        self.world.run(crate::physics2d::physics2d_update_system);
    }

    pub fn finish(&mut self, _info: &FrameInfo) {
        self.world.run(crate::render::renderer2d::renderer2d_to_canvas_system);
    }
}

impl Engine for EngineImpl {
    fn init(&mut self, info: InitInfo) {
        self.world.add_unique(info.platform);
        self.world.add_unique(Physics2DManager::default());
        self.world.add_unique(CameraInfo::new());
        self.world.add_unique(RenderManager::new(info.context));
        self.world.add_unique(Canvas::new());
        self.world.add_unique(SceneManager::new(info.scene));
    }

    fn frame(&mut self, info: &FrameInfo) {
        match info.stage {
            FrameStage::Maintain => self.maintain(info),
            FrameStage::Update => self.update(info),
            FrameStage::Finish => self.finish(info),
        }
    }

    fn draw(&mut self, mut info: DrawInfo) -> Box<dyn GpuFuture> {
        if let Some(editor) = &info.editor_info {
            self.world.run(|mut camera: UniqueViewMut<CameraInfo>| camera.set(editor.camera));
            self.world.run(crate::physics2d::physics2d_debug_render_system);
        }
        self.world.add_unique(FrameRenderInfo::from(&mut info));
        let command_buffer = self.world.run(crate::render::canvas::canvas_render_system);
        self.world.remove_unique::<FrameRenderInfo>().unwrap();
        command_buffer.execute_after(info.before_future, info.context.graphics_queue().clone()).unwrap().boxed()
    }

    fn command(&mut self, cmd: Command) {
        match cmd {
            Command::Save(world_data) => {
                world_data.clear();
                for component_fn in self.component_fns.values() {
                    (component_fn.save_to_data)(world_data, &self.world);
                }
                for unique_fn in self.unique_fns.values() {
                    (unique_fn.save_to_data)(world_data, &self.world);
                }
            },
            Command::Load(world_data) => {
                for component_fn in self.component_fns.values() {
                    (component_fn.load_from_data)(&mut self.world, world_data);
                }
                for unique_fn in self.unique_fns.values() {
                    (unique_fn.load_from_data)(&mut self.world, world_data);
                }
            },
            Command::Reload(world_data) => {
                SceneManager::load(&mut self.world, world_data, &self.component_fns, &self.unique_fns);
            },
            Command::SetCurrentScene(scene) => {
                SceneManager::set_current_scene(&mut self.world, scene);
            }
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
            Command::GetEntityAtScreen(window_index, screen_position, out_eid) => {
                self.world.add_unique(GetEntityAtScreenParam { window_index, screen_position });
                *out_eid = self.world.run(crate::render::canvas::get_entity_at_screen_system);
                self.world.remove_unique::<GetEntityAtScreenParam>().unwrap();
            },
        }
    }
}
