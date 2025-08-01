pub use steel_common::app::*;

use crate::{
    asset::AssetManager,
    camera::{Camera, CameraInfo},
    data::{ComponentRegistry, ComponentRegistryExt, EntitiesDataExt, UniqueRegistry},
    edit::Edit,
    hierarchy::{Children, Hierarchy, Parent},
    name::Name,
    prefab::{CreatePrefabParam, LoadPrefabParam, Prefab, PrefabAssets},
    render::{
        canvas::{Canvas, CanvasRenderContext, GetEntityAtScreenParam},
        image::ImageAssets,
        mesh::{Mesh, MeshAssets},
        model::ModelAssets,
        pipeline::raytracing::material::Material,
        texture::{Texture, TextureAssets},
        FrameRenderInfo, RenderContext, RenderSettings,
    },
    scene::SceneManager,
    time::Time,
    transform::Transform,
    ui::EguiContext,
};
use shipyard::{
    EntitiesView, Get, IntoWorkloadSystem, Unique, UniqueView, UniqueViewMut, View, ViewMut,
    Workload, World,
};
use std::collections::{BTreeMap, HashMap};
use steel_common::platform::Platform;
use vulkano::{command_buffer::PrimaryCommandBufferAbstract, sync::GpuFuture};

/// SteelApp contains data and logic of a steel application.
/// # Examples
/// In src/lib.rs of a newly created steel project, you create a SteelApp and return it from create function:
/// ```rust
/// use steel::app::{App, SteelApp};
///
/// #[no_mangle]
/// pub fn create() -> Box<dyn App> {
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
/// #[no_mangle]
/// pub fn create() -> Box<dyn App> {
///     SteelApp::new()
///         .add_system(Schedule::Init, 0, hello_world)
///         .boxed()
/// }
///
/// fn hello_world() {
///     log::info!("Hello world!");
/// }
/// ```
/// You can add plugins, register/add uniques, and register components:
/// ```rust
/// use steel::{
///     app::{App, SteelApp, Schedule},
///     data::Data,
///     edit::Edit,
///     physics2d::Physics2DPlugin,
/// };
/// use shipyard::{Component, Unique};
///
/// #[no_mangle]
/// pub fn create() -> Box<dyn App> {
///     SteelApp::new()
///         .add_plugin(Physics2DPlugin)
///         .register_component::<MyComponent>()
///         .add_unique(MyUnique)
///         .add_system(Schedule::Update, 0, my_system)
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
    /// The systems that will be added to [Workload] and executed in the application.
    systems: HashMap<Schedule, BTreeMap<i32, Vec<Box<dyn Fn(Workload) -> Workload>>>>,
}

impl SteelApp {
    /// Create a new SteelApp.
    pub fn new() -> Self {
        let world = World::new();
        world.add_unique(ComponentRegistry::new());
        world.add_unique(UniqueRegistry::new());
        SteelApp {
            world,
            systems: HashMap::new(),
        }
        .register_component::<Name>()
        .register_component::<Prefab>()
        .register_component::<Parent>()
        .register_component::<Children>()
        .register_component::<Transform>()
        .register_component::<Camera>()
        .register_component::<Mesh>()
        .register_component::<Texture>()
        .register_component::<Material>()
        .add_and_register_unique(RenderSettings::default())
        .add_and_register_unique(Hierarchy::default())
        .add_unique(AssetManager::default())
        .add_unique(PrefabAssets::default())
        .add_unique(ImageAssets::default())
        .add_unique(TextureAssets::default())
        .add_unique(ModelAssets::default())
        .add_unique(MeshAssets::default())
        .add_unique(CameraInfo::new())
        .add_unique(Canvas::default())
        .add_unique(Time::new())
        .add_system(
            Schedule::PreUpdate,
            crate::scene::SCENE_MAINTAIN_SYSTEM_ORDER,
            crate::scene::scene_maintain_system,
        )
        .add_system(
            Schedule::PreUpdate,
            crate::hierarchy::HIERARCHY_MAINTAIN_SYSTEM_ORDER,
            crate::hierarchy::hierarchy_maintain_system,
        )
        .add_system(
            Schedule::PreUpdate,
            crate::time::TIME_MAINTAIN_SYSTEM_ORDER,
            crate::time::time_maintain_system,
        )
        .add_system(
            Schedule::PreUpdate,
            crate::render::canvas::CANVAS_CLEAR_SYSTEM_ORDER,
            crate::render::canvas::canvas_clear_system,
        )
        .add_system(
            Schedule::PostUpdate,
            crate::render::canvas::CANVAS_UPDATE_SYSTEM_ORDER,
            crate::render::canvas::canvas_update_system,
        )
        .add_system(
            Schedule::PostUpdate,
            crate::camera::CAMERA_MAINTAIN_SYSTEM_ORDER,
            crate::camera::camera_maintain_system,
        )
    }

    /// Box self.
    pub fn boxed(self) -> Box<SteelApp> {
        Box::new(self)
    }

    /// Register a component type so that this component can be edited in steel-editor.
    /// Trait bounds <C: ComponentRegistryExt> equals to <C: Component + Edit + Default + Send + Sync>.
    pub fn register_component<C: ComponentRegistryExt>(self) -> Self {
        self.world
            .get_unique::<&mut ComponentRegistry>()
            .unwrap()
            .register::<C>();
        self
    }

    /// Register a unique type so that this unique can be edited in steel-editor.
    pub fn register_unique<U: Unique + Edit + Default + Send + Sync>(self) -> Self {
        self.world
            .get_unique::<&mut UniqueRegistry>()
            .unwrap()
            .register::<U>();
        self
    }

    /// Add a unique into ecs world.
    pub fn add_unique<U: Unique + Send + Sync>(self, unique: U) -> Self {
        self.world.add_unique(unique);
        self
    }

    /// Add a unique into ecs world, also register this unique type so that this unique can be edited in steel-editor.
    pub fn add_and_register_unique<U: Unique + Edit + Default + Send + Sync>(
        self,
        unique: U,
    ) -> Self {
        self.world.add_unique(unique);
        self.world
            .get_unique::<&mut UniqueRegistry>()
            .unwrap()
            .register::<U>();
        self
    }

    /// Add a system into ecs world that runs on schedule in execution order. The system will try
    /// to execute in parallel as much as possible. If it is not possible to execute in parallel,
    /// it will be executed in order from small to large. For systems with the same order,
    /// they will be executed in the order they were added.
    pub fn add_system<B, R>(
        mut self,
        schedule: Schedule,
        order: i32,
        system: impl IntoWorkloadSystem<B, R> + Copy + 'static,
    ) -> Self {
        self.systems
            .entry(schedule)
            .or_default()
            .entry(order)
            .or_default()
            .push(Box::new(move |workload| workload.with_system(system)));
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
        self.world
            .add_unique(RenderContext::new(info.context, info.ray_tracing_supported));
        self.world.add_unique(CanvasRenderContext::new(
            &self.world.get_unique::<&RenderContext>().unwrap(),
        ));
        self.world.add_unique(SceneManager::new(info.scene));

        let mut create_workload_fn = |schedule: Schedule| {
            let mut workload = Workload::new("");
            for (_, systems) in self.systems.entry(schedule).or_default() {
                for add_system_fn in systems {
                    workload = add_system_fn(workload);
                }
            }
            workload
        };
        Workload::new("init")
            .append(&mut create_workload_fn(Schedule::PreInit))
            .append(&mut create_workload_fn(Schedule::Init))
            .append(&mut create_workload_fn(Schedule::PostInit))
            .add_to_world(&self.world)
            .unwrap();
        Workload::new("update_all")
            .append(&mut create_workload_fn(Schedule::PreUpdate))
            .append(&mut create_workload_fn(Schedule::Update))
            .append(&mut create_workload_fn(Schedule::PostUpdate))
            .add_to_world(&self.world)
            .unwrap();
        Workload::new("update_editor")
            .append(&mut create_workload_fn(Schedule::PreUpdate))
            .append(&mut create_workload_fn(Schedule::PostUpdate))
            .add_to_world(&self.world)
            .unwrap();
        Workload::new("draw_editor")
            .append(&mut create_workload_fn(Schedule::DrawEditor))
            .add_to_world(&self.world)
            .unwrap();
        self.systems.clear();

        self.world.run_workload("init").unwrap();
    }

    fn update(&self, info: UpdateInfo) {
        self.world.add_unique(EguiContext::new(info.ctx.clone()));

        let workload = if info.update {
            "update_all"
        } else {
            "update_editor"
        };
        self.world.run_workload(workload).unwrap();

        self.world.remove_unique::<EguiContext>().unwrap();
    }

    fn draw(&self, mut info: DrawInfo) -> Box<dyn GpuFuture> {
        if let Some(editor) = &info.editor_info {
            self.world
                .run(|mut camera: UniqueViewMut<CameraInfo>| camera.set(editor.camera));
            self.world.run_workload("draw_editor").unwrap();
        }
        self.world.add_unique(FrameRenderInfo::from(&mut info));
        let (gpu_future, command_buffer) =
            self.world.run(crate::render::canvas::canvas_render_system);
        self.world.remove_unique::<FrameRenderInfo>().unwrap();
        command_buffer
            .execute_after(
                info.before_future.join(gpu_future),
                info.context.graphics_queue().clone(),
            )
            .unwrap()
            .boxed()
    }

    fn command(&self, cmd: Command) {
        match cmd {
            Command::Save(world_data) => {
                world_data.clear();
                for e in self.world.borrow::<EntitiesView>().unwrap().iter() {
                    // make sure that word_data contains entities without any components
                    world_data.entities.insert(e, Default::default());
                }
                for component_fn in self
                    .world
                    .get_unique::<&ComponentRegistry>()
                    .unwrap()
                    .values()
                {
                    (component_fn.save_to_data)(world_data, &self.world.all_storages().unwrap());
                }
                for unique_fn in self.world.get_unique::<&UniqueRegistry>().unwrap().values() {
                    (unique_fn.save_to_data)(world_data, &self.world.all_storages().unwrap());
                }
            }
            Command::Load(world_data) => {
                for component_fn in self
                    .world
                    .get_unique::<&ComponentRegistry>()
                    .unwrap()
                    .values()
                {
                    (component_fn.load_from_data)(&self.world.all_storages().unwrap(), world_data);
                }
                for unique_fn in self.world.get_unique::<&UniqueRegistry>().unwrap().values() {
                    (unique_fn.load_from_data)(&self.world.all_storages().unwrap(), world_data);
                }
            }
            Command::Reload(scene_data) => {
                SceneManager::load(&mut self.world.all_storages_mut().unwrap(), scene_data);
            }
            Command::SetCurrentScene(scene) => {
                self.world.run(
                    |mut scene_manager: UniqueViewMut<SceneManager>,
                     asset_manager: UniqueView<AssetManager>| {
                        scene_manager.set_current_scene(scene, &asset_manager);
                    },
                );
            }
            Command::CreateEntity => {
                self.world
                    .all_storages_mut()
                    .unwrap()
                    .add_entity((Name::new("New Entity"),));
            }
            Command::AddEntities(entities_data, old_id_to_new_id) => {
                *old_id_to_new_id = entities_data.add_to_world(&self.world.all_storages().unwrap());
            }
            Command::DestroyEntity(id) => {
                self.world.all_storages_mut().unwrap().delete_entity(id);
            }
            Command::GetEntityCount(entity_count) => {
                *entity_count = self
                    .world
                    .run(|entities: EntitiesView| entities.iter().count());
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
            Command::GetEntityName(id, name) => {
                *name = self
                    .world
                    .borrow::<View<Name>>()
                    .unwrap()
                    .get(id)
                    .ok()
                    .map(|name| name.0.clone());
            }
            Command::CreateComponent(id, component_name) => {
                if let Some(component_fn) = self
                    .world
                    .get_unique::<&ComponentRegistry>()
                    .unwrap()
                    .get(component_name)
                {
                    (component_fn.create)(&self.world.all_storages().unwrap(), id);
                }
            }
            Command::DestroyComponent(id, component_name) => {
                if let Some(component_fn) = self
                    .world
                    .get_unique::<&ComponentRegistry>()
                    .unwrap()
                    .get(component_name.as_str())
                {
                    (component_fn.destroy)(&self.world.all_storages().unwrap(), id);
                }
            }
            Command::GetComponents(components) => {
                *components = self
                    .world
                    .get_unique::<&ComponentRegistry>()
                    .unwrap()
                    .keys()
                    .map(|s| *s)
                    .collect();
                // TODO: cache components
            }
            Command::AttachBefore(eid, parent, before) => {
                self.world.run(
                    |mut hierarchy: UniqueViewMut<Hierarchy>,
                     mut childrens: ViewMut<Children>,
                     mut parents: ViewMut<Parent>| {
                        crate::hierarchy::attach_before(
                            &mut hierarchy,
                            &mut childrens,
                            &mut parents,
                            eid,
                            parent,
                            before,
                        );
                    },
                );
            }
            Command::AttachAfter(eid, parent, after) => {
                self.world.run(
                    |mut hierarchy: UniqueViewMut<Hierarchy>,
                     mut childrens: ViewMut<Children>,
                     mut parents: ViewMut<Parent>| {
                        crate::hierarchy::attach_after(
                            &mut hierarchy,
                            &mut childrens,
                            &mut parents,
                            eid,
                            parent,
                            after,
                        );
                    },
                );
            }
            Command::ResetTime => {
                self.world.run(|mut time: UniqueViewMut<Time>| time.reset());
            }
            Command::GetAssetPath(asset_id, path) => {
                *path = self
                    .world
                    .borrow::<UniqueView<AssetManager>>()
                    .unwrap()
                    .get_asset_path(asset_id)
                    .map(|path| path.clone());
            }
            Command::GetAssetContent(asset_id, content) => {
                *content = self.world.run(
                    |mut asset_manager: UniqueViewMut<AssetManager>,
                     platform: UniqueView<Platform>| {
                        asset_manager
                            .get_asset_content(asset_id, platform.as_ref())
                            .map(|content| content.clone())
                    },
                );
            }
            Command::AssetIdExists(asset_id, exists) => {
                *exists = self
                    .world
                    .borrow::<UniqueView<AssetManager>>()
                    .unwrap()
                    .contains_asset(asset_id);
            }
            Command::InsertAsset(asset_id, path) => {
                self.world
                    .borrow::<UniqueViewMut<AssetManager>>()
                    .unwrap()
                    .insert_asset(asset_id, path);
            }
            Command::DeleteAsset(asset_id) => {
                self.world
                    .borrow::<UniqueViewMut<AssetManager>>()
                    .unwrap()
                    .delete_asset(asset_id);
            }
            Command::DeleteAssetDir(dir) => {
                self.world
                    .borrow::<UniqueViewMut<AssetManager>>()
                    .unwrap()
                    .delete_asset_dir(dir);
            }
            Command::UpdateAssetPath(asset_id, path) => {
                self.world
                    .borrow::<UniqueViewMut<AssetManager>>()
                    .unwrap()
                    .update_asset_path(asset_id, path);
            }
            Command::GetPrefabData(asset_id, data) => {
                *data = self.world.run(
                    |mut prefab_assets: UniqueViewMut<PrefabAssets>,
                     mut asset_manager: UniqueViewMut<AssetManager>,
                     platform: UniqueView<Platform>| {
                        prefab_assets.get_prefab_data(
                            asset_id,
                            asset_manager.as_mut(),
                            platform.as_ref(),
                        )
                    },
                );
            }
            Command::CreatePrefab(
                prefab_root_entity,
                prefab_asset,
                prefab_root_entity_to_nested_prefabs_index,
            ) => {
                self.world.add_unique(CreatePrefabParam {
                    prefab_root_entity,
                    prefab_asset,
                    prefab_root_entity_to_nested_prefabs_index,
                });
                self.world.run(crate::prefab::create_prefab_system);
                self.world.remove_unique::<CreatePrefabParam>().unwrap();
            }
            Command::LoadPrefab(
                prefab_root_entity,
                prefab_asset,
                entity_id_to_prefab_entity_id_with_path,
            ) => {
                self.world.add_unique(LoadPrefabParam {
                    prefab_root_entity,
                    prefab_asset,
                    entity_id_to_prefab_entity_id_with_path,
                });
                self.world.run(crate::prefab::load_prefab_system);
                self.world.remove_unique::<LoadPrefabParam>().unwrap();
            }
            Command::AddEntitiesFromPrefab(prefab_asset, prefab_root_entity) => {
                *prefab_root_entity = crate::prefab::add_entities_from_prefab(
                    &self.world.all_storages().unwrap(),
                    prefab_asset,
                )
            }
        }
    }
}

/// System running schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
///             .register_component::<RigidBody2D>()
///             .register_component::<Collider2D>()
///             .add_system(Schedule::PreUpdate, crate::physics2d::physics2d_maintain_system)
///             .add_system(Schedule::Update, crate::physics2d::physics2d_update_system)
///             .add_system(Schedule::DrawEditor, crate::physics2d::physics2d_debug_render_system)
///     }
/// }
/// ```
pub trait Plugin {
    fn apply(self, app: SteelApp) -> SteelApp;
}
