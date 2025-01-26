//! The editor for the [steel game engine](https://github.com/SSSxCCC/steel).

mod locale;
mod project;
mod ui;
mod utils;

use crate::{project::Project, ui::Editor, utils::LocalData};
use egui_winit_vulkano::{Gui, GuiConfig};
use glam::Vec2;
use steel_common::{
    app::{Command, DrawInfo, EditorInfo, UpdateInfo},
    camera::SceneCamera,
    data::WorldData,
};
use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator, format::Format, image::ImageUsage,
    sync::GpuFuture,
};
use vulkano_util::window::{VulkanoWindows, WindowDescriptor};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
};
use winit_input_helper::WinitInputHelper;

// Currently we can not use cargo in android, so that running steel-editor in android is useless
// TODO: remove android code in steel-editor, or find a way to make steel-editor work in android

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Trace),
    );
    use winit::platform::android::EventLoopBuilderExtAndroid;
    let event_loop = EventLoopBuilder::new().with_android_app(app).build();
    _main(event_loop);
}

#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .parse_default_env()
        .init();
    let event_loop = EventLoopBuilder::new().build();
    _main(event_loop);
}

fn _main(event_loop: EventLoop<()>) {
    // local data
    let mut local_data = LocalData::load();

    // graphics
    let (context, ray_tracing_supported) = steel_common::create_context();
    // TODO: use the same command buffer allocator in steel::render::RenderContext,
    // or fix issue in main::ui::image_window::ImageWindow and remove this.
    let command_buffer_allocator =
        StandardCommandBufferAllocator::new(context.device().clone(), Default::default());
    let mut windows = VulkanoWindows::default();
    let mut window_title = None;
    let mut scene_camera = SceneCamera::default();

    // input
    let mut input_editor = WinitInputHelper::new(); // for editor window
    let mut events = Vec::new();

    // egui
    let mut gui_editor = None; // for editor ui
    let mut gui: Option<Gui> = None; // for in-game ui
    let mut editor = Editor::new(&local_data);

    // project
    let mut project = Project::new(
        ray_tracing_supported,
        &mut local_data,
        &mut scene_camera,
        &mut window_title,
        &context,
        &mut gui,
    );

    log::info!("Start main loop!");
    event_loop.run(move |event, event_loop, control_flow| match event {
        Event::Resumed => {
            log::debug!("Event::Resumed");
            windows.create_window(
                &event_loop,
                &context,
                &WindowDescriptor {
                    title: window_title.take().unwrap_or("Steel Editor".into()),
                    ..Default::default()
                },
                |info| {
                    info.image_format = Format::B8G8R8A8_UNORM; // for egui, see https://github.com/hakolao/egui_winit_vulkano
                    info.image_usage |= ImageUsage::STORAGE;
                },
            );
            let renderer = windows.get_primary_renderer().unwrap();
            log::info!("Swapchain image format: {:?}", renderer.swapchain_format());
            gui_editor = Some(Gui::new(
                &event_loop,
                renderer.surface(),
                renderer.graphics_queue(),
                renderer.swapchain_format(),
                GuiConfig {
                    is_overlay: false,
                    ..Default::default()
                },
            ));
            // load fonts to support displaying chinese in egui
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "msyh".to_owned(),
                egui::FontData::from_static(include_bytes!("../fonts/msyh.ttc")),
            );
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "msyh".to_owned());
            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .push("msyh".to_owned());
            gui_editor.as_ref().unwrap().egui_ctx.set_fonts(fonts);
        }
        Event::Suspended => {
            log::debug!("Event::Suspended");
            editor.suspend();
            gui_editor = None;
            gui = None;
            windows.remove_renderer(windows.primary_window_id().unwrap());
        }
        Event::WindowEvent { event, .. } => {
            if let Some(gui_editor) = gui_editor.as_mut() {
                let _pass_events_to_game = !gui_editor.update(&event);
            }
            match event {
                WindowEvent::CloseRequested => {
                    log::info!("WindowEvent::CloseRequested");
                    project.exit(&mut local_data, scene_camera);
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
            if let Some(mut event) = event.to_static() {
                events.push(event.clone());

                if project.is_running() {
                    if let Some(gui) = gui.as_mut() {
                        adjust_event_for_window(
                            &mut event,
                            editor.game_window().position(),
                            gui.egui_ctx.pixels_per_point(),
                        );
                        let _pass_events_to_game = !gui.update(&event);
                    }
                }
            }
        }
        Event::RedrawRequested(_) => {
            project.maintain_asset_dir();
            input_editor.step_with_window_events(&events);
            if let Some(renderer) = windows.get_primary_renderer_mut() {
                if let Some(window_title) = window_title.take() {
                    renderer.window().set_title(&window_title);
                }

                let window_size = renderer.window().inner_size();
                if window_size.width == 0 || window_size.height == 0 {
                    return; // Prevent "Failed to recreate swapchain: ImageExtentZeroLengthDimensions" in renderer.acquire().unwrap()
                }
                let mut gpu_future = renderer.acquire().unwrap();

                let gui_editor = gui_editor.as_mut().unwrap();
                let mut world_data = project.app().map(|e| {
                    let mut world_data = WorldData::default();
                    e.command(Command::Save(&mut world_data));
                    world_data
                });
                editor.ui(
                    gui_editor,
                    &mut gui,
                    &context,
                    renderer,
                    &mut project,
                    &mut world_data,
                    &mut local_data,
                    &mut scene_camera,
                    &mut window_title,
                    &input_editor,
                );

                let is_running = project.is_running();
                if let Some(app) = project.app() {
                    if gui.is_none() {
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
                    let gui = gui.as_mut().unwrap();
                    let mut raw_input = gui.egui_winit.take_egui_input(renderer.window());
                    let screen_size = editor.game_window().size();
                    raw_input.screen_rect = Some(egui::Rect::from_x_y_ranges(
                        0.0..=(screen_size.x as f32),
                        0.0..=(screen_size.y as f32),
                    ));
                    gui.egui_ctx.options_mut(|options| {
                        options.zoom_factor = gui_editor.egui_ctx.zoom_factor()
                    });
                    gui.egui_ctx.begin_frame(raw_input);

                    events.iter_mut().for_each(|e| {
                        adjust_event_for_window(
                            e,
                            editor.game_window().position(),
                            gui.egui_ctx.pixels_per_point(),
                        )
                    });
                    app.command(Command::UpdateInput(&events));

                    if let Some(world_data) = world_data.as_mut() {
                        app.command(Command::Load(world_data));
                    }

                    app.update(UpdateInfo {
                        update: is_running,
                        ctx: &gui.egui_ctx,
                    });

                    if let Some(image) = editor.game_window().image() {
                        let draw_future = app.draw(DrawInfo {
                            before_future: vulkano::sync::now(context.device().clone()).boxed(),
                            context: &context,
                            renderer: &renderer,
                            image: image.clone(),
                            window_size: editor.game_window().pixel(),
                            editor_info: None,
                        });
                        // There is a display abnormal problem, it seems like that this image displayed without finishing drawing.
                        // We wait for draw future here to avoid this problem.
                        // TODO: fix this display problem.
                        draw_future
                            .then_signal_fence_and_flush()
                            .unwrap()
                            .wait(None)
                            .unwrap();
                        let gui_future = gui.draw_on_image(
                            vulkano::sync::now(context.device().clone()).boxed(), // TODO: use draw_future
                            image.clone(),
                        );
                        let copy_future = editor.game_window().copy_image(
                            &command_buffer_allocator,
                            context.graphics_queue().clone(),
                            gui_future,
                        );
                        gpu_future = gpu_future.join(copy_future).boxed();
                    }

                    if let Some(image) = editor.scene_window().image() {
                        let draw_future = app.draw(DrawInfo {
                            before_future: vulkano::sync::now(context.device().clone()).boxed(),
                            context: &context,
                            renderer: &renderer,
                            image: image.clone(),
                            window_size: editor.scene_window().pixel(),
                            editor_info: Some(EditorInfo {
                                camera: &scene_camera,
                            }),
                        });
                        // There is a crash problem "access to a resource has been denied".
                        // We wait for draw future here to avoid this problem.
                        // TODO: fix this crash problem.
                        draw_future
                            .then_signal_fence_and_flush()
                            .unwrap()
                            .wait(None)
                            .unwrap();
                        let copy_future = editor.scene_window().copy_image(
                            &command_buffer_allocator,
                            context.graphics_queue().clone(),
                            vulkano::sync::now(context.device().clone()).boxed(), // TODO: use draw_future
                        );
                        gpu_future = gpu_future.join(copy_future).boxed();
                    }
                }

                gpu_future = gui_editor.draw_on_image(gpu_future, renderer.swapchain_image_view());

                renderer.present(gpu_future, true);
            }
            events.clear();
        }
        Event::MainEventsCleared => {
            if let Some(renderer) = windows.get_primary_renderer() {
                renderer.window().request_redraw()
            }
        }
        _ => (),
    });
}

fn adjust_event_for_window(
    event: &mut WindowEvent<'static>,
    window_position: Vec2,
    scale_factor: f32,
) {
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
        WindowEvent::CursorLeft { .. }
        | WindowEvent::CursorEntered { .. }
        | WindowEvent::Focused(_)
        | WindowEvent::Resized(_)
        | WindowEvent::Moved(_)
        | WindowEvent::CloseRequested
        | WindowEvent::Destroyed
        | WindowEvent::DroppedFile(_)
        | WindowEvent::HoveredFile(_)
        | WindowEvent::HoveredFileCancelled
        | WindowEvent::ReceivedCharacter(_)
        | WindowEvent::KeyboardInput { .. }
        | WindowEvent::ModifiersChanged(_)
        | WindowEvent::Ime(_)
        | WindowEvent::MouseWheel { .. }
        | WindowEvent::MouseInput { .. }
        | WindowEvent::TouchpadMagnify { .. }
        | WindowEvent::SmartMagnify { .. }
        | WindowEvent::TouchpadRotate { .. }
        | WindowEvent::TouchpadPressure { .. }
        | WindowEvent::AxisMotion { .. }
        | WindowEvent::ThemeChanged(_)
        | WindowEvent::Occluded(_) => (),
        WindowEvent::ScaleFactorChanged { .. } => {
            unreachable!("Static event can't be about scale factor changing")
        }
    }
}
