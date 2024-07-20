pub use steel_common::app::*;

use crate::{
    camera::{Camera, CameraInfo},
    data::{ComponentRegistry, ComponentRegistryExt, EntitiesDataExt, UniqueRegistry},
    edit::Edit,
    entityinfo::EntityInfo,
    hierarchy::{Child, Hierarchy, Parent},
    input::Input,
    render::{
        canvas::{Canvas, GetEntityAtScreenParam},
        renderer2d::Renderer2D,
        FrameRenderInfo, RenderManager,
    },
    scene::SceneManager,
    time::Time,
    transform::Transform,
    ui::EguiContext,
};
use shipyard::{EntitiesView, IntoWorkloadSystem, Unique, UniqueViewMut, Workload, World};
use vulkano::{command_buffer::PrimaryCommandBufferAbstract, sync::GpuFuture};

/// SteelApp contains data and logic of a steel application.
/// # Examples
/// In src/lib.rs of a newly created steel project, you create a SteelApp and return it from create function:
/// ```rust
/// use steel::app::{App, SteelApp};
///
/// fn create() -> Box<dyn App> {
///     SteelApp::new().boxed()
/// }
/// ```
/// Note: create function is the entry point function of steel application that called once when the application started.
/// Please do not modify the function name and return value type of the create function, because the editor needs to
/// call the create function to generate an App object after dynamically loading the application code. If you modify
/// its function name or return value type, the editor will not be able to find the create function and crash.
///
/// Write a hello world system that runs at the start of application:
/// ```rust
/// use steel::app::{App, SteelApp, Schedule};
///
/// fn create() -> Box<dyn App> {
///     SteelApp::new()
///         .add_system(Schedule::Init, hello_world)
///         .boxed()
/// }
///
/// fn hello_world() {
///     log::info!("Hello world!");
/// }
/// ```
/// You can add plugins, register/add uniques, and register components:
/// ```rust
/// use steel::{app::{App, SteelApp, Schedule}, data::Data, edit::Edit, physics2d::Physics2DPlugin};
/// use shipyard::{Component, Unique};
///
/// fn create() -> Box<dyn App> {
///     SteelApp::new()
///         .add_plugin(Physics2DPlugin)
///         .register_component::<MyComponent>()
///         .add_unique(MyUnique)
///         .add_system(Schedule::Update, my_system)
///         .boxed()
/// }
///
/// #[derive(Unique)]
/// struct MyUnique;
///
/// #[derive(Component, Edit, Default)]
/// struct MyComponent;
///
/// fn my_system() {
///     log::info!("my_system");
/// }
/// ```
pub struct SteelApp {
    /// The ecs world, contains entities, components, and uniques.
    pub world: World,
    /// Registered components.
    pub component_registry: ComponentRegistry,
    /// Registered uniques.
    pub unique_registry: UniqueRegistry,

    pre_init_workload: Option<Workload>,
    init_workload: Option<Workload>,
    post_init_workload: Option<Workload>,

    pre_update_workload: Option<Workload>,
    update_workload: Option<Workload>,
    post_update_workload: Option<Workload>,

    pre_update_workload_editor: Option<Workload>,
    post_update_workload_editor: Option<Workload>,

    draw_editor_workload: Option<Workload>,
}

impl SteelApp {
    /// Create a new SteelApp.
    pub fn new() -> Self {
        SteelApp {
            world: World::new(),
            component_registry: ComponentRegistry::new(),
            unique_registry: UniqueRegistry::new(),
            pre_init_workload: Some(Workload::new("pre_init")),
            init_workload: Some(Workload::new("init")),
            post_init_workload: Some(Workload::new("post_init")),
            pre_update_workload: Some(Workload::new("pre_update")),
            update_workload: Some(Workload::new("update")),
            post_update_workload: Some(Workload::new("post_update")),
            pre_update_workload_editor: Some(Workload::new("pre_update_editor")),
            post_update_workload_editor: Some(Workload::new("post_update_editor")),
            draw_editor_workload: Some(Workload::new("draw_editor")),
        }
        .register_component::<EntityInfo>()
        .register_component::<Parent>()
        .register_component::<Child>()
        .register_component::<Transform>()
        .register_component::<Camera>()
        .register_component::<Renderer2D>()
        .register_unique::<RenderManager>()
        .add_and_register_unique(Hierarchy::default())
        .add_unique(CameraInfo::new())
        .add_unique(Canvas::new())
        .add_unique(Input::new())
        .add_unique(Time::new())
        .add_system(
            Schedule::PreUpdate,
            crate::hierarchy::hierarchy_maintain_system,
        )
        .add_system(Schedule::PreUpdate, crate::time::time_maintain_system)
        .add_system(
            Schedule::PreUpdate,
            crate::render::canvas::canvas_clear_system,
        )
        .add_system(Schedule::PreUpdate, crate::camera::camera_maintain_system)
        .add_system(
            Schedule::PostUpdate,
            crate::render::renderer2d::renderer2d_to_canvas_system,
        )
    }

    /// Box self.
    pub fn boxed(self) -> Box<SteelApp> {
        Box::new(self)
    }

    /// Register a component type so that this component can be edited in steel-editor.
    /// Trait bounds <C: ComponentRegistryExt> equals to <C: Component + Edit + Default + Send + Sync>.
    pub fn register_component<C: ComponentRegistryExt>(mut self) -> Self {
        self.component_registry.register::<C>();
        self
    }

    /// Register a unique type so that this unique can be edited in steel-editor.
    pub fn register_unique<U: Unique + Edit + Send + Sync>(mut self) -> Self {
        self.unique_registry.register::<U>();
        self
    }

    /// Add a unique into ecs world.
    pub fn add_unique<U: Unique + Send + Sync>(self, unique: U) -> Self {
        self.world.add_unique(unique);
        self
    }

    /// Add a unique into ecs world, also register this unique type so that this unique can be edited in steel-editor.
    pub fn add_and_register_unique<U: Unique + Edit + Send + Sync>(mut self, unique: U) -> Self {
        self.world.add_unique(unique);
        self.unique_registry.register::<U>();
        self
    }

    /// Add a system into ecs world that runs on schedule.
    pub fn add_system<B>(
        mut self,
        schedule: Schedule,
        system: impl IntoWorkloadSystem<B, ()> + Copy,
    ) -> Self {
        match schedule {
            Schedule::PreInit => {
                self.pre_init_workload =
                    Some(self.pre_init_workload.take().unwrap().with_system(system));
            }
            Schedule::Init => {
                self.init_workload = Some(self.init_workload.take().unwrap().with_system(system));
            }
            Schedule::PostInit => {
                self.post_init_workload =
                    Some(self.post_init_workload.take().unwrap().with_system(system));
            }
            Schedule::PreUpdate => {
                self.pre_update_workload =
                    Some(self.pre_update_workload.take().unwrap().with_system(system));
                self.pre_update_workload_editor = Some(
                    self.pre_update_workload_editor
                        .take()
                        .unwrap()
                        .with_system(system),
                );
            }
            Schedule::Update => {
                self.update_workload =
                    Some(self.update_workload.take().unwrap().with_system(system));
            }
            Schedule::PostUpdate => {
                self.post_update_workload = Some(
                    self.post_update_workload
                        .take()
                        .unwrap()
                        .with_system(system),
                );
                self.post_update_workload_editor = Some(
                    self.post_update_workload_editor
                        .take()
                        .unwrap()
                        .with_system(system),
                );
            }
            Schedule::DrawEditor => {
                self.draw_editor_workload = Some(
                    self.draw_editor_workload
                        .take()
                        .unwrap()
                        .with_system(system),
                );
            }
        }
        self
    }

    /// Add a plugin, see [Plugin] for more information.
    pub fn add_plugin(self, plugin: impl Plugin) -> Self {
        plugin.apply(self)
    }
}

impl App for SteelApp {
    fn init(&mut self, info: InitInfo) {
        self.world.add_unique(info.platform);
        self.world.add_unique(RenderManager::new(info.context));
        self.world.add_unique(SceneManager::new(info.scene));
        Workload::new("init")
            .append(&mut self.pre_init_workload.take().unwrap())
            .append(&mut self.init_workload.take().unwrap())
            .append(&mut self.post_init_workload.take().unwrap())
            .add_to_world(&self.world)
            .unwrap();
        Workload::new("update_all")
            .append(&mut self.pre_update_workload.take().unwrap())
            .append(&mut self.update_workload.take().unwrap())
            .append(&mut self.post_update_workload.take().unwrap())
            .add_to_world(&self.world)
            .unwrap();
        Workload::new("update_editor")
            .append(&mut self.pre_update_workload_editor.take().unwrap())
            .append(&mut self.post_update_workload_editor.take().unwrap())
            .add_to_world(&self.world)
            .unwrap();
        Workload::new("draw_editor")
            .append(&mut self.draw_editor_workload.take().unwrap())
            .add_to_world(&self.world)
            .unwrap();

        self.world.run_workload("init").unwrap();
    }

    fn update(&mut self, info: UpdateInfo) {
        self.world.add_unique(EguiContext::new(info.ctx.clone()));

        SceneManager::maintain_system(
            &mut self.world,
            &self.component_registry,
            &self.unique_registry,
        );

        let workload = if info.update {
            "update_all"
        } else {
            "update_editor"
        };
        self.world.run_workload(workload).unwrap();

        self.world.remove_unique::<EguiContext>().unwrap();
    }

    fn draw(&mut self, mut info: DrawInfo) -> Box<dyn GpuFuture> {
        if let Some(editor) = &info.editor_info {
            self.world
                .run(|mut camera: UniqueViewMut<CameraInfo>| camera.set(editor.camera));
            self.world.run_workload("draw_editor").unwrap();
        }
        self.world.add_unique(FrameRenderInfo::from(&mut info));
        let command_buffer = self.world.run(crate::render::canvas::canvas_render_system);
        self.world.remove_unique::<FrameRenderInfo>().unwrap();
        command_buffer
            .execute_after(info.before_future, info.context.graphics_queue().clone())
            .unwrap()
            .boxed()
    }

    fn command(&mut self, cmd: Command) {
        match cmd {
            Command::Save(world_data) => {
                world_data.clear();
                for component_fn in self.component_registry.values() {
                    (component_fn.save_to_data)(world_data, &self.world);
                }
                for unique_fn in self.unique_registry.values() {
                    (unique_fn.save_to_data)(world_data, &self.world);
                }
            }
            Command::Load(world_data) => {
                for component_fn in self.component_registry.values() {
                    (component_fn.load_from_data)(&mut self.world, world_data);
                }
                for unique_fn in self.unique_registry.values() {
                    (unique_fn.load_from_data)(&mut self.world, world_data);
                }
            }
            Command::Reload(world_data) => {
                SceneManager::load(
                    &mut self.world,
                    world_data,
                    &self.component_registry,
                    &self.unique_registry,
                );
            }
            Command::SetCurrentScene(scene) => {
                SceneManager::set_current_scene(&mut self.world, scene);
            }
            Command::CreateEntity => {
                self.world.add_entity((
                    EntityInfo::new("New Entity"),
                    Transform::default(),
                    Renderer2D::default(),
                ));
            }
            Command::DestroyEntity(id) => {
                self.world.delete_entity(id);
            }
            Command::ClearEntity => {
                self.world.clear();
            }
            Command::GetEntityCount(entity_count) => {
                *entity_count = self
                    .world
                    .run(|entities: EntitiesView| entities.iter().count());
            }
            Command::AddEntities(entities_data, old_id_to_new_id) => {
                *old_id_to_new_id =
                    entities_data.add_to_world(&mut self.world, &self.component_registry);
            }
            Command::GetComponents(components) => {
                *components = self.component_registry.keys().map(|s| *s).collect();
                // TODO: cache components
            }
            Command::CreateComponent(id, component_name) => {
                if let Some(component_fn) = self.component_registry.get(component_name) {
                    (component_fn.create)(&mut self.world, id);
                }
            }
            Command::DestroyComponent(id, component_name) => {
                if let Some(component_fn) = self.component_registry.get(component_name.as_str()) {
                    (component_fn.destroy)(&mut self.world, id);
                }
            }
            Command::UpdateInput(events) => {
                self.world
                    .run(|mut input: UniqueViewMut<Input>| input.step_with_window_events(events));
            }
            Command::GetEntityAtScreen(window_index, screen_position, out_eid) => {
                self.world.add_unique(GetEntityAtScreenParam {
                    window_index,
                    screen_position,
                });
                *out_eid = self
                    .world
                    .run(crate::render::canvas::get_entity_at_screen_system);
                self.world
                    .remove_unique::<GetEntityAtScreenParam>()
                    .unwrap();
            }
            Command::ResetTime => {
                self.world.run(|mut time: UniqueViewMut<Time>| time.reset());
            }
            Command::AttachBefore(eid, parent, before) => {
                crate::hierarchy::attach_before(&mut self.world, eid, parent, before);
            }
            Command::AttachAfter(eid, parent, after) => {
                crate::hierarchy::attach_after(&mut self.world, eid, parent, after);
            }
        }
    }
}

/// System running schedule.
pub enum Schedule {
    /// The schedule that runs once when the application starts before [Schedule::Init].
    PreInit,
    /// The schedule that runs once when the application starts.
    Init,
    /// The schedule that runs once when the application starts after [Schedule::Init].
    PostInit,
    /// The schedule that runs every frame before [Schedule::Update].
    PreUpdate,
    /// The schedule that runs every frame.
    /// This schedule is skipped in steel-editor when the game is not running.
    /// For example, physics2d_update_system should run in this scheduler so that
    /// physics objects do not fall due to gravity when the game is not running in the Editor.
    Update,
    /// The schedule that runs every frame after [Schedule::Update].
    PostUpdate,
    /// The schedule that runs before drawing editor scene window.
    /// You can put systems here to display something only in scene window.
    /// For example, physics2d_debug_render_system shows colliders' bounds only in scene window.
    DrawEditor,
}

/// Plugin is a collection of components, uniques, and systems. You can use [SteelApp::add_plugin] to add them to SteelApp.
/// # Example
/// ```
/// pub struct Physics2DPlugin;
///
/// impl Plugin for Physics2DPlugin {
///     fn apply(self, app: SteelApp) -> SteelApp {
///         app.add_and_register_unique(Physics2DManager::default())
///             .register_component_track_all::<RigidBody2D>()
///             .register_component_track_all::<Collider2D>()
///             .add_system(Schedule::PreUpdate, crate::physics2d::physics2d_maintain_system)
///             .add_system(Schedule::Update, crate::physics2d::physics2d_update_system)
///             .add_system(Schedule::DrawEditor, crate::physics2d::physics2d_debug_render_system)
///     }
/// }
/// ```
pub trait Plugin {
    fn apply(self, app: SteelApp) -> SteelApp;
}
