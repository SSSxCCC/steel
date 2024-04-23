use std::path::PathBuf;
use shipyard::UniqueViewMut;
use steel::{engine::{Engine, EngineImpl}, scene::SceneManager};

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
    }

    fn frame(&mut self, info: &steel::engine::FrameInfo) {
        self.inner.frame(info);
        if matches!(info.stage, steel::engine::FrameStage::Maintain) {
            egui::Window::new("TestWindow").show(info.ctx, |ui| {
                if ui.button("Button").clicked() {
                    log::info!("Click button of TestWindow");
                    self.inner.world.run(|mut scene_manager: UniqueViewMut<SceneManager>| {
                        if let Some(current_scene) = scene_manager.current_scene() {
                            if *current_scene == PathBuf::from("scene/test.scene") {
                                scene_manager.switch_scene("scene/scene.scene".into());
                            } else if *current_scene == PathBuf::from("scene/scene.scene") {
                                scene_manager.switch_scene("scene/test.scene".into());
                            }
                        }
                    });
                }
            });
        }
    }

    fn draw(&mut self, info: steel::engine::DrawInfo) -> Box<dyn vulkano::sync::GpuFuture> {
        self.inner.draw(info)
    }

    fn command(&mut self, cmd: steel::engine::Command) {
        self.inner.command(cmd);
    }
}
