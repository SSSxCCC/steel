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

#[cfg(target_os = "android")]
use winit::platform::android::{activity::AndroidApp, EventLoopBuilderExtAndroid};

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Trace),
    );
    let event_loop = EventLoop::builder()
        .with_android_app(app.clone())
        .build()
        .unwrap();
    let platform = Platform::new(app);
    _main(event_loop, platform);
}

#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .parse_default_env()
        .init();
    let event_loop = EventLoop::new().unwrap();
    let platform = Platform::new_client();
    _main(event_loop, platform);
}

fn _main(event_loop: EventLoop<()>, platform: Platform) {
    event_loop.set_control_flow(ControlFlow::Poll);

    // graphics
    let (context, ray_tracing_supported) = steel_common::create_context();
    let windows = VulkanoWindows::default();

    // egui
    let gui = None;

    // app
    let mut app = steel::create();

    // insert all assets into AssetManager
    let insert_asset_fn = |asset_info_file: &Path| -> Result<(), Box<dyn Error>> {
        let asset_info_string = platform.read_asset_to_string(asset_info_file)?;
        let asset_id = serde_json::from_str::<AssetInfo>(&asset_info_string)?.id;
        let asest_file_path = AssetInfo::asset_info_path_to_asset_path(asset_info_file);
        log::debug!(
            "Insert asset \"{}\", id: {asset_id:?}",
            asest_file_path.display()
        );
        app.command(Command::InsertAsset(asset_id, asest_file_path));
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

    // call App::init
    app.init(InitInfo {
        platform,
        context: &context,
        ray_tracing_supported,
        scene: Some(AssetId::new("init_scene".parse().unwrap())),
    }); // init scene will be modified to init scene asset id temporily while compiling

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
