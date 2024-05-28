use std::path::PathBuf;
use glam::{IVec2, IVec3, IVec4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};
use shipyard::{Component, EntityId, UniqueView, UniqueViewMut};
use steel::{data::{Data, Value}, edit::Edit, engine::{Engine, EngineImpl}, scene::SceneManager, ui::EguiContext};

#[no_mangle]
pub fn create() -> Box<dyn Engine> {
    Box::new(EngineWrapper { inner: EngineImpl::new() })
}

struct EngineWrapper {
    inner: EngineImpl,
}

impl Engine for EngineWrapper {
    fn init(&mut self, info: steel::engine::InitInfo) {
        self.inner.init(info);
        self.inner.register_component::<TestComponent>();
    }

    fn frame(&mut self, info: &steel::engine::FrameInfo) {
        self.inner.frame(info);
        if matches!(info.stage, steel::engine::FrameStage::Maintain) {
            self.inner.world.run(test_system);
        }
    }

    fn draw(&mut self, info: steel::engine::DrawInfo) -> Box<dyn vulkano::sync::GpuFuture> {
        self.inner.draw(info)
    }

    fn command(&mut self, cmd: steel::engine::Command) {
        self.inner.command(cmd);
    }
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

fn test_system(ctx: UniqueView<EguiContext>, mut scene_manager: UniqueViewMut<SceneManager>) {
    egui::Window::new("TestWindow").show(ctx.as_ref(), |ui| {
        if ui.button("Button").clicked() {
            log::info!("Click button of TestWindow");
            if let Some(current_scene) = scene_manager.current_scene() {
                if *current_scene == PathBuf::from("scene/test.scene") {
                    scene_manager.switch_scene("scene/scene.scene".into());
                } else if *current_scene == PathBuf::from("scene/scene.scene") {
                    scene_manager.switch_scene("scene/test.scene".into());
                }
            }
        }
    });
}
