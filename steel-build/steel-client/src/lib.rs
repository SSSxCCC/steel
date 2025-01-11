//! The game client for the [steel game engine](https://github.com/SSSxCCC/steel).

use egui_winit_vulkano::{Gui, GuiConfig};
use glam::UVec2;
use std::{error::Error, path::Path};
use steel_common::{
    app::{Command, DrawInfo, InitInfo, UpdateInfo},
    asset::{AssetId, AssetInfo},
    platform::Platform,
};
use vulkano::{format::Format, image::ImageUsage};
use vulkano_util::window::{VulkanoWindows, WindowDescriptor};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
};

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Trace),
    );
    use winit::platform::android::EventLoopBuilderExtAndroid;
    let event_loop = EventLoopBuilder::new()
        .with_android_app(app.clone())
        .build();
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
    let event_loop = EventLoopBuilder::new().build();
    let platform = Platform::new_client();
    _main(event_loop, platform);
}

fn _main(event_loop: EventLoop<()>, platform: Platform) {
    // graphics
    let (context, ray_tracing_supported) = steel_common::create_context();
    let mut windows = VulkanoWindows::default();

    // input
    let mut events = Vec::new();

    // egui
    let mut gui = None;

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

    log::info!("Start main loop!");
    event_loop.run(move |event, event_loop, control_flow| match event {
        Event::Resumed => {
            log::debug!("Event::Resumed");
            windows.create_window(
                &event_loop,
                &context,
                &WindowDescriptor::default(),
                |info| {
                    info.image_format = Format::B8G8R8A8_UNORM; // for egui, see https://github.com/hakolao/egui_winit_vulkano
                    info.image_usage |= ImageUsage::STORAGE;
                },
            );
            let renderer = windows.get_primary_renderer().unwrap();
            log::info!("Swapchain image format: {:?}", renderer.swapchain_format());
            gui = Some(Gui::new(
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
        Event::Suspended => {
            log::debug!("Event::Suspended");
            gui = None;
            windows.remove_renderer(windows.primary_window_id().unwrap());
        }
        Event::WindowEvent { event, .. } => {
            if let Some(gui) = gui.as_mut() {
                let _pass_events_to_game = !gui.update(&event);
            }
            match event {
                WindowEvent::CloseRequested => {
                    log::info!("WindowEvent::CloseRequested");
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Resized(_) => {
                    log::debug!("WindowEvent::Resized");
                    if let Some(renderer) = windows.get_primary_renderer_mut() {
                        renderer.resize()
                    }
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    log::debug!("WindowEvent::ScaleFactorChanged");
                    if let Some(renderer) = windows.get_primary_renderer_mut() {
                        renderer.resize()
                    }
                }
                _ => (),
            }
            // Warning: event.to_static() may drop some events, like ScaleFactorChanged
            // TODO: find a way to deliver all events to WinitInputHelper
            if let Some(event) = event.to_static() {
                events.push(event);
            }
        }
        Event::RedrawRequested(_) => {
            app.command(Command::UpdateInput(&events));
            events.clear();
            if let Some(renderer) = windows.get_primary_renderer_mut() {
                let window_size = renderer.window().inner_size();
                if window_size.width == 0 || window_size.height == 0 {
                    return; // Prevent "Failed to recreate swapchain: ImageExtentZeroLengthDimensions" in renderer.acquire().unwrap()
                }
                let mut gpu_future = renderer.acquire().unwrap();

                let gui = gui.as_mut().unwrap();
                gui.begin_frame();

                app.update(UpdateInfo {
                    update: true,
                    ctx: &gui.egui_ctx,
                });

                gpu_future = app.draw(DrawInfo {
                    before_future: gpu_future,
                    context: &context,
                    renderer: &renderer,
                    image: renderer.swapchain_image_view(),
                    window_size: UVec2::from_array(renderer.swapchain_image_size()),
                    editor_info: None,
                });

                gpu_future = gui.draw_on_image(gpu_future, renderer.swapchain_image_view());

                renderer.present(gpu_future, true);
            }
        }
        Event::MainEventsCleared => {
            if let Some(renderer) = windows.get_primary_renderer() {
                renderer.window().request_redraw()
            }
        }
        _ => (),
    });
}
