mod ui;
mod project;
mod utils;

use glam::{UVec2, Vec2, Vec3};
use steel_common::{data::WorldData, engine::{Command, DrawInfo, EditorCamera, EditorInfo, UpdateInfo}};
use egui_winit_vulkano::{Gui, GuiConfig};
use vulkano::sync::GpuFuture;
use vulkano_util::{context::{VulkanoConfig, VulkanoContext}, window::{VulkanoWindows, WindowDescriptor}};
use winit::{event::{Event, VirtualKeyCode, WindowEvent}, event_loop::{ControlFlow, EventLoop, EventLoopBuilder}};
use winit_input_helper::WinitInputHelper;

use crate::{project::Project, ui::Editor, utils::LocalData};

// Currently we can not use cargo in android, so that running steel-editor in android is useless
// TODO: remove android code in steel-editor, or find a way to make steel-editor work in android

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
    // local data
    let mut local_data = LocalData::load();

    // graphics
    let mut config = VulkanoConfig::default();
    config.device_features.fill_mode_non_solid = true;
    let context = VulkanoContext::new(config);
    let mut windows = VulkanoWindows::default();
    let mut editor_camera = EditorCamera { position: Vec3::ZERO, height: 20.0 };

    // input
    let mut input = WinitInputHelper::new();
    let mut events = Vec::new();

    // egui
    let mut gui_editor = None; // for editor ui
    let mut gui: Option<Gui> = None; // for in-game ui
    let mut editor = Editor::new(&local_data);

    // project
    let mut project = Project::new();

    log::debug!("Start main loop!");
    event_loop.run(move |event, event_loop, control_flow| match event {
        Event::Resumed => {
            log::debug!("Event::Resumed");
            windows.create_window(&event_loop, &context,
                &WindowDescriptor::default(), |_|{});
            let renderer = windows.get_primary_renderer().unwrap();
            gui_editor = Some(Gui::new(&event_loop, renderer.surface(),
                renderer.graphics_queue(),
                renderer.swapchain_format(),
                GuiConfig { is_overlay: false, ..Default::default() }));
        }
        Event::Suspended => {
            log::debug!("Event::Suspended");
            editor.suspend();
            gui_editor = None;
            gui = None;
            windows.remove_renderer(windows.primary_window_id().unwrap());
        }
        Event::WindowEvent { event , .. } => {
            if let Some(gui_editor) = gui_editor.as_mut() {
                let _pass_events_to_game = !gui_editor.update(&event);
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
            if let Some(mut event) = event.to_static() {
                events.push(event.clone());

                if project.is_running() {
                    if let Some(gui) = gui.as_mut() {
                        process_event_for_game_gui(&mut event, editor.game_position(), gui.egui_ctx.pixels_per_point());
                        let _pass_events_to_game = !gui.update(&event);
                    }
                }
            }
        }
        Event::RedrawRequested(_) => {
            log::trace!("Event::RedrawRequested");
            input.step_with_window_events(&events);
            events.clear();
            if let Some(renderer) = windows.get_primary_renderer_mut() {
                let window_size = renderer.window().inner_size();
                if window_size.width == 0 || window_size.height == 0 {
                    return; // Prevent "Failed to recreate swapchain: ImageExtentZeroLengthDimensions" in renderer.acquire().unwrap()
                }
                let mut gpu_future = renderer.acquire().unwrap();

                let gui_editor = gui_editor.as_mut().unwrap();
                let mut world_data = project.engine().map(|e| {
                    let mut world_data = WorldData::new();
                    e.command(Command::Save(&mut world_data));
                    world_data
                });
                editor.ui(gui_editor, &mut gui, &context, renderer, &mut project, &mut local_data, &mut world_data);

                let is_running = project.is_running();
                if let Some(engine) = project.engine() {
                    if gui.is_none() {
                        gui = Some(Gui::new(&event_loop, renderer.surface(),
                            renderer.graphics_queue(),
                            renderer.swapchain_format(),
                            GuiConfig { is_overlay: true, ..Default::default() }));
                    }
                    let gui = gui.as_mut().unwrap();
                    let mut raw_input = gui.egui_winit.take_egui_input(renderer.window());
                    let screen_size = if is_running { editor.game_pixel() } else { editor.scene_pixel() };
                    raw_input.screen_rect = Some(egui::Rect::from_x_y_ranges(0.0..=(screen_size.x as f32), 0.0..=(screen_size.y as f32)));
                    raw_input.pixels_per_point = Some(gui_editor.egui_ctx.pixels_per_point());
                    gui.egui_ctx.begin_frame(raw_input);

                    if let Some(world_data) = world_data.as_mut() {
                        engine.command(Command::Load(world_data));
                    }

                    let update_info = UpdateInfo { input: &input, ctx: &gui.egui_ctx };
                    engine.maintain(&update_info);
                    if is_running {
                        engine.update(&update_info);
                    }
                    engine.finish(&update_info);

                    if is_running {
                        let mut draw_future = engine.draw(DrawInfo {
                            before_future: vulkano::sync::now(context.device().clone()).boxed(),
                            context: &context, renderer: &renderer,
                            image: editor.game_image().as_ref().unwrap().clone(),
                            window_size: editor.game_pixel(),
                            editor_info: None,
                        });
                        draw_future = gui.draw_on_image(draw_future, editor.game_image().as_ref().unwrap().clone());
                        gpu_future = gpu_future.join(draw_future).boxed();
                    }

                    let scene_window_size = editor.scene_pixel();
                    if editor.scene_focus() {
                        update_editor_camera(&mut editor_camera, &input, &scene_window_size);
                    }

                    let mut draw_future = engine.draw(DrawInfo {
                        before_future: vulkano::sync::now(context.device().clone()).boxed(),
                        context: &context, renderer: &renderer,
                        image: editor.scene_image().as_ref().unwrap().clone(),
                        window_size: scene_window_size,
                        editor_info: Some(EditorInfo { camera: &editor_camera }),
                    });
                    if !is_running {
                        draw_future = gui.draw_on_image(draw_future, editor.scene_image().as_ref().unwrap().clone());
                    }
                    gpu_future = gpu_future.join(draw_future).boxed();
                }

                let gpu_future = gui_editor.draw_on_image(gpu_future, renderer.swapchain_image_view());

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

fn update_editor_camera(editor_camera: &mut EditorCamera, input: &WinitInputHelper, window_size: &UVec2) {
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
        let screen_to_world = editor_camera.height / window_size.y as f32;
        let mouse_diff = input.mouse_diff();
        editor_camera.position.x -= mouse_diff.0 * screen_to_world;
        editor_camera.position.y += mouse_diff.1 * screen_to_world;
    }
}

fn process_event_for_game_gui(event: &mut WindowEvent<'static>, window_position: Vec2, scale_factor: f32) {
    match event {
        WindowEvent::CursorMoved { position, .. } => {
            let (window_position, scale_factor) = (window_position.as_dvec2(), scale_factor as f64);
            position.x = position.x - window_position.x * scale_factor;
            position.y = position.y - window_position.y * scale_factor;
        }
        WindowEvent::Touch(touch) => {
            let (window_position, scale_factor) = (window_position.as_dvec2(), scale_factor as f64);
            touch.location.x = touch.location.x - window_position.x * scale_factor;
            touch.location.y = touch.location.y - window_position.y * scale_factor;
        }
        // events that we do not need change
        // note: we must write all types of event here, because when we upgrade winit,
        // we will know the types of event we did not consider
        WindowEvent::CursorLeft { .. } |
        WindowEvent::CursorEntered { .. } |
        WindowEvent::Focused(_) |
        WindowEvent::Resized(_) |
        WindowEvent::Moved(_) |
        WindowEvent::CloseRequested |
        WindowEvent::Destroyed |
        WindowEvent::DroppedFile(_) |
        WindowEvent::HoveredFile(_) |
        WindowEvent::HoveredFileCancelled |
        WindowEvent::ReceivedCharacter(_) |
        WindowEvent::KeyboardInput { .. } |
        WindowEvent::ModifiersChanged(_) |
        WindowEvent::Ime(_) |
        WindowEvent::MouseWheel { .. } |
        WindowEvent::MouseInput { .. } |
        WindowEvent::TouchpadMagnify { .. } |
        WindowEvent::SmartMagnify { .. } |
        WindowEvent::TouchpadRotate { .. } |
        WindowEvent::TouchpadPressure { .. } |
        WindowEvent::AxisMotion { .. } |
        WindowEvent::ThemeChanged(_) |
        WindowEvent::Occluded(_) => (),
        WindowEvent::ScaleFactorChanged { .. } => unreachable!("Static event can't be about scale factor changing"),
    }
}
