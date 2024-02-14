mod ui;
mod project;

use glam::{Vec2, Vec3};
use steel_common::{DrawInfo, EditorCamera};
use egui_winit_vulkano::{Gui, GuiConfig};
use vulkano::sync::GpuFuture;
use vulkano_util::{window::{VulkanoWindows, WindowDescriptor}, context::VulkanoContext};
use winit::{
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
};
use winit_input_helper::WinitInputHelper;

use crate::{ui::Editor, project::Project};

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Trace));
    use winit::platform::android::EventLoopBuilderExtAndroid;
    let event_loop = EventLoopBuilder::new().with_android_app(app).build();
    _main(event_loop);
}

#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Debug).parse_default_env().init();
    let event_loop = EventLoopBuilder::new().build();
    _main(event_loop);
}

fn _main(event_loop: EventLoop<()>) {
    // graphics
    let context = VulkanoContext::default();
    let mut windows = VulkanoWindows::default();
    let mut editor_camera = EditorCamera { position: Vec3::ZERO, height: 20.0 };

    // egui
    let mut gui = None;
    let mut editor = Editor::new();

    // project
    let mut project = Project::new();

    // input
    let mut input = WinitInputHelper::new();
    let mut events = Vec::new();

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
                GuiConfig { is_overlay: false, ..Default::default() }));
        }
        Event::Suspended => {
            log::debug!("Event::Suspended");
            editor.suspend();
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
            input.step_with_window_events(&events);
            events.clear();
            if let Some(renderer) = windows.get_primary_renderer_mut() {
                let gui = gui.as_mut().unwrap();
                let mut world_data = project.engine().map(|e| { e.save() });
                if let Some(world_data) = world_data.as_ref() { log::trace!("world_data={:?}", world_data); }
                editor.ui(gui, &context, renderer, &mut project, world_data.as_mut());

                let mut gpu_future = renderer.acquire().unwrap();

                let is_running = project.is_running();
                if let Some(engine) = project.engine() {
                    if let Some(world_data) = world_data.as_mut() { engine.load(world_data); }
                    engine.maintain();

                    if is_running {
                        engine.update();
                        gpu_future = gpu_future.join(engine.draw(DrawInfo {
                            before_future: vulkano::sync::now(context.device().clone()).boxed(),
                            context: &context, renderer: &renderer,
                            image: editor.game_image().as_ref().unwrap().clone(),
                            window_size: editor.game_size(),
                        })).boxed();
                    }

                    let scene_window_size = editor.scene_size();
                    update_editor_camera(&mut editor_camera, &input, &scene_window_size, renderer.window().scale_factor());
                    gpu_future = gpu_future.join(engine.draw_editor(DrawInfo {
                        before_future: vulkano::sync::now(context.device().clone()).boxed(),
                        context: &context, renderer: &renderer,
                        image: editor.scene_image().as_ref().unwrap().clone(),
                        window_size: scene_window_size,
                    }, &editor_camera)).boxed();
                }

                let gpu_future = gui.draw_on_image(gpu_future, renderer.swapchain_image_view());

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

fn update_editor_camera(editor_camera: &mut EditorCamera, input: &WinitInputHelper, window_size: &Vec2, scale_factor: f64) {
    if input.key_pressed(VirtualKeyCode::Home) {
        editor_camera.position = Vec3::ZERO;
        editor_camera.height = 20.0;
    }

    if input.key_held(VirtualKeyCode::A) || input.key_held(VirtualKeyCode::Left) {
        editor_camera.position.x -= 1.0; // TODO: * move_speed * delta_time
    }
    if input.key_held(VirtualKeyCode::D) || input.key_held(VirtualKeyCode::Right) {
        editor_camera.position.x += 1.0;
    }
    if input.key_held(VirtualKeyCode::W) || input.key_held(VirtualKeyCode::Up) {
        editor_camera.position.y += 1.0;
    }
    if input.key_held(VirtualKeyCode::S) || input.key_held(VirtualKeyCode::Down) {
        editor_camera.position.y -= 1.0;
    }

    let scroll_diff = input.scroll_diff();
    if scroll_diff > 0.0 {
        editor_camera.height /= 1.1;
    } else if scroll_diff < 0.0 {
        editor_camera.height *= 1.1;
    }

    if input.mouse_held(1) {
        let screen_to_world = editor_camera.height / window_size.y / scale_factor as f32;
        let mouse_diff = input.mouse_diff();
        editor_camera.position.x -= mouse_diff.0 * screen_to_world;
        editor_camera.position.y += mouse_diff.1 * screen_to_world;
    }
}
