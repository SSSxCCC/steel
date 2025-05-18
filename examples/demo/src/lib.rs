mod components;
mod ray_tracing_in_one_weekend;

use components::{DemoComponent, TagComponent};
use ray_tracing_in_one_weekend::RayTracingInOneWeekend;
use shipyard::{Unique, UniqueView, UniqueViewMut};
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
        .register_component::<DemoComponent>()
        .register_component::<TagComponent>()
        .register_component::<RayTracingInOneWeekend>()
        .add_system(
            Schedule::PreUpdate,
            0,
            ray_tracing_in_one_weekend::generate_scene_system,
        )
        .add_system(Schedule::PostUpdate, 0, test_system)
        .boxed()
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
