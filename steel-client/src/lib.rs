//! The game client for the [steel game engine](https://github.com/SSSxCCC/steel).

use std::path::PathBuf;
use egui_winit_vulkano::{Gui, GuiConfig};
use glam::UVec2;
use steel_common::{engine::{Command, DrawInfo, FrameInfo, FrameStage, InitInfo}, platform::Platform};
use vulkano_util::{context::{VulkanoConfig, VulkanoContext}, window::{VulkanoWindows, WindowDescriptor}};
use winit::{event::{Event, WindowEvent}, event_loop::{ControlFlow, EventLoop, EventLoopBuilder}};

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Trace));
    use winit::platform::android::EventLoopBuilderExtAndroid;
    let event_loop = EventLoopBuilder::new().with_android_app(app.clone()).build();
    let platform = Platform::new(app);
    _main(event_loop, platform);
}

#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Debug).parse_default_env().init();
    let event_loop = EventLoopBuilder::new().build();
    let platform = Platform::new_client();
    _main(event_loop, platform);
}

fn _main(event_loop: EventLoop<()>, platform: Platform) {
    // graphics
    let mut config = VulkanoConfig::default();
    config.device_features.fill_mode_non_solid = true;
    let context = VulkanoContext::new(config);
    let mut windows = VulkanoWindows::default();

    // input
    let mut events = Vec::new();

    // egui
    let mut gui = None;

    // engine
    let mut engine = steel::create();
    engine.init(InitInfo { platform, context: &context, scene: Some(PathBuf::from("scene_path")) }); // scene path will be modified to init scene path temporily while compiling

    log::debug!("Start main loop!");
    event_loop.run(move |event, event_loop, control_flow| match event {
        Event::Resumed => {
            log::debug!("Event::Resumed");
            windows.create_window(&event_loop, &context,
                &WindowDescriptor::default(), |_|{});
            let renderer = windows.get_primary_renderer().unwrap();
            gui = Some(Gui::new(&event_loop, renderer.surface(),
                renderer.graphics_queue(),
                renderer.swapchain_format(),
                GuiConfig { is_overlay: true, ..Default::default() }));
        }
        Event::Suspended => {
            log::debug!("Event::Suspended");
            gui = None;
            windows.remove_renderer(windows.primary_window_id().unwrap());
        }
        Event::WindowEvent { event , .. } => {
            if let Some(gui) = gui.as_mut() {
                let _pass_events_to_game = !gui.update(&event);
            }
            match event {
                WindowEvent::CloseRequested => {
                    log::debug!("WindowEvent::CloseRequested");
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Resized(_) => {
                    log::debug!("WindowEvent::Resized");
                    if let Some(renderer) = windows.get_primary_renderer_mut() { renderer.resize() }
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    log::debug!("WindowEvent::ScaleFactorChanged");
                    if let Some(renderer) = windows.get_primary_renderer_mut() { renderer.resize() }
                }
                _ => ()
            }
            // Warning: event.to_static() may drop some events, like ScaleFactorChanged
            // TODO: find a way to deliver all events to WinitInputHelper
            if let Some(event) = event.to_static() {
                events.push(event);
            }
        }
        Event::RedrawRequested(_) => {
            log::trace!("Event::RedrawRequested");
            engine.command(Command::UpdateInput(&events));
            events.clear();
            if let Some(renderer) = windows.get_primary_renderer_mut() {
                let window_size = renderer.window().inner_size();
                if window_size.width == 0 || window_size.height == 0 {
                    return; // Prevent "Failed to recreate swapchain: ImageExtentZeroLengthDimensions" in renderer.acquire().unwrap()
                }
                let mut gpu_future = renderer.acquire().unwrap();

                let gui = gui.as_mut().unwrap();
                gui.begin_frame();

                let mut frame_info = FrameInfo { stage: FrameStage::Maintain, ctx: &gui.egui_ctx };
                engine.frame(&frame_info);
                frame_info.stage = FrameStage::Update;
                engine.frame(&frame_info);
                frame_info.stage = FrameStage::Finish;
                engine.frame(&frame_info);

                gpu_future = engine.draw(DrawInfo {
                    before_future: gpu_future,
                    context: &context, renderer: &renderer,
                    image: renderer.swapchain_image_view(),
                    window_size: UVec2::from_array(renderer.swapchain_image_size()),
                    editor_info: None,
                });

                gpu_future = gui.draw_on_image(gpu_future, renderer.swapchain_image_view());

                renderer.present(gpu_future, true);
            }
        }
        Event::MainEventsCleared => {
            log::trace!("Event::MainEventsCleared");
            if let Some(renderer) = windows.get_primary_renderer() { renderer.window().request_redraw() }
        }
        _ => (),
    });
}
