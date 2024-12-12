mod ray_tracing_in_one_weekend;

use glam::{IVec2, IVec3, IVec4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};
use ray_tracing_in_one_weekend::RayTracingInOneWeekend;
use shipyard::{Component, EntityId, Unique, UniqueView, UniqueViewMut};
use steel::{
    app::{App, Schedule, SteelApp},
    asset::AssetId,
    data::{Data, Value},
    edit::Edit,
    physics2d::Physics2DPlugin,
    scene::SceneManager,
    ui::EguiContext,
};

#[no_mangle]
pub fn create() -> Box<dyn App> {
    SteelApp::new()
        .add_plugin(Physics2DPlugin)
        .add_and_register_unique(MyUnique::default())
        .register_component::<TestComponent>()
        .register_component::<TagComponent>()
        .register_component::<RayTracingInOneWeekend>()
        .add_system(
            Schedule::PreUpdate,
            ray_tracing_in_one_weekend::generate_scene_system,
        )
        .add_system(Schedule::PostUpdate, test_system)
        .boxed()
}

#[derive(Component, Edit, Default)]
struct TestComponent {
    bool: bool,
    int32: i32,
    uint32: u32,
    float32: f32,
    string: String,
    entity: EntityId,
    vec2: Vec2,
    vec3: Vec3,
    vec4: Vec4,
    ivec2: IVec2,
    ivec3: IVec3,
    ivec4: IVec4,
    uvec2: UVec2,
    uvec3: UVec3,
    uvec4: UVec4,
}

#[derive(Unique, Edit, Default)]
struct MyUnique {
    scene1: AssetId,
    scene2: AssetId,
}

fn test_system(
    ctx: UniqueView<EguiContext>,
    mut scene_manager: UniqueViewMut<SceneManager>,
    my_unique: UniqueView<MyUnique>,
) {
    egui::Window::new("TestWindow").show(ctx.as_ref(), |ui| {
        if ui.button("Button").clicked() {
            log::info!("Click button of TestWindow");
            if let Some(current_scene) = scene_manager.current_scene() {
                if current_scene == my_unique.scene1 {
                    scene_manager.switch_scene(my_unique.scene2);
                } else if current_scene == my_unique.scene2 {
                    scene_manager.switch_scene(my_unique.scene1);
                }
            }
        }
    });
}

#[derive(Component, Edit, Default)]
struct TagComponent;
