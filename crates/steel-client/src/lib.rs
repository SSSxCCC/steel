//! The game client for the [steel game engine](https://github.com/SSSxCCC/steel).

use egui_winit_vulkano::{Gui, GuiConfig};
use glam::UVec2;
use std::{error::Error, path::Path};
use steel_common::{
    app::{App, Command, DrawInfo, InitInfo, UpdateInfo},
    asset::{AssetId, AssetInfo},
    platform::{BuildTarget, Platform, BUILD_TARGET},
};
use vulkano::{format::Format, image::ImageUsage};
use vulkano_util::{
    context::VulkanoContext,
    window::{VulkanoWindows, WindowDescriptor},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
};

pub fn run(mut app: Box<dyn App>) {
    #[cfg(not(target_os = "android"))]
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .parse_default_env()
        .init();

    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Trace),
    );

    let mut event_loop_builder = EventLoop::builder();

    #[cfg(target_os = "android")]
    {
        use winit::platform::android::EventLoopBuilderExtAndroid;
        let msg = "Steel must be setup with the #[steel_main] macro on Android";
        event_loop_builder.with_android_app(
            steel_common::platform::android::ANDROID_APP
                .get()
                .expect(msg)
                .clone(),
        );
    }

    let event_loop = event_loop_builder.build().unwrap();
    let platform = Platform::new();
    event_loop.set_control_flow(ControlFlow::Poll);

    // graphics
    let (context, ray_tracing_supported) = steel_common::create_context();
    let windows = VulkanoWindows::default();

    // egui
    let gui = None;

    // insert all assets into AssetManager
    let mut default_scene = None;
    let mut insert_asset_fn = |asset_info_file: &Path| -> Result<(), Box<dyn Error>> {
        let asset_info_string = platform.read_asset_to_string(asset_info_file)?;
        let asset_id = serde_json::from_str::<AssetInfo>(&asset_info_string)?.id;
        let asset_file_path = AssetInfo::asset_info_path_to_asset_path(asset_info_file);
        log::debug!(
            "Insert asset \"{}\", id: {asset_id:?}",
            asset_file_path.display()
        );
        default_scene = default_scene.or_else(|| default_scene_asset(&asset_file_path, asset_id));
        app.command(Command::InsertAsset(asset_id, asset_file_path));
        Ok(())
    };
    platform
        .list_asset_files()
        .unwrap()
        .into_iter()
        .filter(|f| f.extension().is_some_and(|e| e == "asset"))
        .for_each(|asset_info_file| {
            if let Err(e) = insert_asset_fn(&asset_info_file) {
                log::warn!(
                    "Failed to insert asset: {}, error: {e}",
                    asset_info_file.display()
                );
            }
        });

    if let Some(scene) = default_scene {
        log::info!("Loading default scene: assets/main.scene ({scene:?})");
    } else {
        log::info!("No assets/main.scene found; starting with an empty scene");
    }

    // call App::init
    app.init(InitInfo {
        platform,
        context: &context,
        ray_tracing_supported,
        scene: default_scene,
    });

    // application
    let mut application = Application {
        context,
        windows,
        gui,
        app,
    };

    log::info!("Start main loop!");
    event_loop.run_app(&mut application).unwrap();
}

fn default_scene_asset(asset_path: &Path, asset_id: AssetId) -> Option<AssetId> {
    (asset_path == Path::new("main.scene")).then_some(asset_id)
}

#[cfg(test)]
mod tests {
    use super::default_scene_asset;
    use std::path::Path;
    use steel_common::asset::AssetId;

    #[test]
    fn root_main_scene_is_the_default_scene() {
        let asset_id = AssetId::new(42);

        assert_eq!(
            default_scene_asset(Path::new("main.scene"), asset_id),
            Some(asset_id)
        );
    }

    #[test]
    fn other_scene_paths_are_not_selected_as_default() {
        assert_eq!(
            default_scene_asset(Path::new("game.scene"), AssetId::new(42)),
            None
        );
        assert_eq!(
            default_scene_asset(Path::new("scenes/main.scene"), AssetId::new(43)),
            None
        );
    }
}

struct Application {
    context: VulkanoContext,
    windows: VulkanoWindows,
    gui: Option<Gui>,
    app: Box<dyn App>,
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        log::debug!("Event::Resumed");
        self.windows.create_window(
            &event_loop,
            &self.context,
            &WindowDescriptor::default(),
            |info| {
                // for egui, see https://github.com/hakolao/egui_winit_vulkano
                match BUILD_TARGET {
                    BuildTarget::Desktop => info.image_format = Format::B8G8R8A8_UNORM,
                    BuildTarget::Android => info.image_format = Format::R8G8B8A8_UNORM,
                }
                info.image_usage |= ImageUsage::STORAGE;
            },
        );
        let renderer = self.windows.get_primary_renderer().unwrap();
        log::info!("Swapchain image format: {:?}", renderer.swapchain_format());
        self.gui = Some(Gui::new(
            &event_loop,
            renderer.surface(),
            renderer.graphics_queue(),
            renderer.swapchain_format(),
            GuiConfig {
                is_overlay: true,
                ..Default::default()
            },
        ));
    }

    fn suspended(&mut self, _: &winit::event_loop::ActiveEventLoop) {
        log::debug!("Event::Suspended");
        self.gui = None;
        self.windows
            .remove_renderer(self.windows.primary_window_id().unwrap());
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if let Some(gui) = self.gui.as_mut() {
            let _pass_events_to_game = !gui.update(&event);
        }

        match event {
            WindowEvent::CloseRequested => {
                log::info!("WindowEvent::CloseRequested");
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                log::debug!("WindowEvent::Resized");
                if let Some(renderer) = self.windows.get_primary_renderer_mut() {
                    renderer.resize();
                    renderer.window().request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                log::debug!("WindowEvent::ScaleFactorChanged");
                if let Some(renderer) = self.windows.get_primary_renderer_mut() {
                    renderer.resize();
                    renderer.window().request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.windows.get_primary_renderer_mut() {
                    let window_size = renderer.window().inner_size();
                    if window_size.width == 0 || window_size.height == 0 {
                        return; // Prevent "Failed to recreate swapchain: ImageExtentZeroLengthDimensions" in renderer.acquire().unwrap()
                    }
                    let mut gpu_future = renderer.acquire(None, |_| {}).unwrap();

                    let gui = self.gui.as_mut().unwrap();
                    gui.begin_frame();

                    self.app.update(UpdateInfo {
                        update: true,
                        ctx: &gui.egui_ctx,
                    });

                    gpu_future = self.app.draw(DrawInfo {
                        before_future: gpu_future,
                        context: &self.context,
                        renderer: &renderer,
                        image: renderer.swapchain_image_view(),
                        window_size: UVec2::from_array(renderer.swapchain_image_size()),
                        editor_info: None,
                    });

                    gpu_future = gui.draw_on_image(gpu_future, renderer.swapchain_image_view());

                    renderer.present(gpu_future, true);

                    renderer.window().request_redraw();
                }
            }
            _ => (),
        }
    }
}
