use glam::Vec2;
use steel_common::{platform::Platform, DrawInfo, WorldData};
use vulkano_util::{context::{VulkanoConfig, VulkanoContext}, window::{VulkanoWindows, WindowDescriptor}};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
};
use winit_input_helper::WinitInputHelper;

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
    let platform = Platform::new();
    _main(event_loop, platform);
}

fn _main(event_loop: EventLoop<()>, platform: Platform) {
    // graphics
    let mut config = VulkanoConfig::default();
    config.device_features.fill_mode_non_solid = true;
    let context = VulkanoContext::new(config);
    let mut windows = VulkanoWindows::default();

    // input
    let mut input = WinitInputHelper::new();
    let mut events = Vec::new();

    // engine
    let world_data = WorldData::load_from_file("scene.json", &platform);
    let mut engine = steel::create();
    engine.init(world_data.as_ref());

    log::debug!("Start main loop!");
    event_loop.run(move |event, event_loop, control_flow| match event {
        Event::Resumed => {
            log::debug!("Event::Resumed");
            windows.create_window(&event_loop, &context,
                &WindowDescriptor::default(), |_|{});
        }
        Event::Suspended => {
            log::debug!("Event::Suspended");
            windows.remove_renderer(windows.primary_window_id().unwrap());
        }
        Event::WindowEvent { event , .. } => {
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
            input.step_with_window_events(&events);
            events.clear();
            if let Some(renderer) = windows.get_primary_renderer_mut() {
                let mut gpu_future = renderer.acquire().unwrap();

                engine.maintain();
                engine.update();
                engine.draw();

                gpu_future = engine.draw_game(DrawInfo {
                    before_future: gpu_future,
                    context: &context, renderer: &renderer,
                    image: renderer.swapchain_image_view(),
                    window_size: Vec2::from_array(renderer.swapchain_image_size().map(|s| s as f32)),
                });

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
