//! The editor for the [steel game engine](https://github.com/SSSxCCC/steel).

mod locale;
mod project;
mod ui;
mod utils;

use crate::{project::Project, ui::Editor, utils::LocalData};
use egui_winit_vulkano::{Gui, GuiConfig};
use glam::Vec2;
use std::sync::Arc;
use steel_common::{
    app::{Command, DrawInfo, EditorInfo, UpdateInfo},
    camera::SceneCamera,
    data::WorldData,
};
use vulkano::{
    command_buffer::allocator::{CommandBufferAllocator, StandardCommandBufferAllocator},
    format::Format,
    image::ImageUsage,
    sync::GpuFuture,
};
use vulkano_util::{
    context::VulkanoContext,
    window::{VulkanoWindows, WindowDescriptor},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
};

/// Whether to exit the editor when compiling the code.
/// Steel Editor supports hot reload by using libloading. However libloading and rayon have critical issues when working together.
/// The rayon global thread pool can not be destroyed when unloading a dynamic library, which will cause memory leak and crash.
/// We avoid this issue by re-running the editor when compiling the code.
/// TODO: fix this issue and remove this.
pub const EXIT_EDITOR_WHEN_COMPILE: bool = true;

// Currently we can not use cargo in android, so that running steel-editor in android is useless
// TODO: remove android code in steel-editor, or find a way to make steel-editor work in android

#[cfg(target_os = "android")]
use winit::platform::android::{activity::AndroidApp, EventLoopBuilderExtAndroid};

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Trace),
    );
    let event_loop = EventLoop::builder().with_android_app(app).build().unwrap();
    _main(event_loop);
}

#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .parse_default_env()
        .init();
    let event_loop = EventLoop::new().unwrap();
    _main(event_loop);
}

fn _main(event_loop: EventLoop<()>) {
    event_loop.set_control_flow(ControlFlow::Poll);

    // local data
    let mut local_data = LocalData::load();

    // graphics
    let (context, ray_tracing_supported) = steel_common::create_context();
    // TODO: use the same command buffer allocator in steel::render::RenderContext,
    // or fix issue in main::ui::image_window::ImageWindow and remove this.
    let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
        context.device().clone(),
        Default::default(),
    ));
    let windows = VulkanoWindows::default();
    let mut window_title = None;
    let mut scene_camera = SceneCamera::default();

    // egui
    let gui_editor = None; // for editor ui
    let mut gui: Option<Gui> = None; // for in-game ui
    let editor = Editor::new(&local_data);

    // project
    let project = Project::new(
        ray_tracing_supported,
        &mut local_data,
        &mut scene_camera,
        &mut window_title,
        &context,
        &mut gui,
    );

    // application
    let mut application = Application {
        local_data,
        context,
        command_buffer_allocator,
        windows,
        window_title,
        scene_camera,
        gui_editor,
        gui,
        editor,
        project,
    };

    log::info!("Start main loop!");
    event_loop.run_app(&mut application).unwrap();
}

struct Application {
    local_data: LocalData,
    context: VulkanoContext,
    command_buffer_allocator: Arc<dyn CommandBufferAllocator>,
    windows: VulkanoWindows,
    window_title: Option<String>,
    scene_camera: SceneCamera,
    gui_editor: Option<Gui>,
    gui: Option<Gui>,
    editor: Editor,
    project: Project,
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        log::debug!("Resumed");
        self.windows.create_window(
            &event_loop,
            &self.context,
            &WindowDescriptor {
                title: self.window_title.take().unwrap_or("Steel Editor".into()),
                ..Default::default()
            },
            |info| {
                info.image_format = Format::B8G8R8A8_UNORM; // for egui, see https://github.com/hakolao/egui_winit_vulkano
                info.image_usage |= ImageUsage::STORAGE;
            },
        );
        let renderer = self.windows.get_primary_renderer().unwrap();
        log::info!("Swapchain image format: {:?}", renderer.swapchain_format());
        self.gui_editor = Some(Gui::new(
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
            Arc::new(egui::FontData::from_static(include_bytes!(
                "../fonts/msyh.ttc"
            ))),
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
        self.gui_editor.as_ref().unwrap().egui_ctx.set_fonts(fonts);
    }

    fn suspended(&mut self, _: &winit::event_loop::ActiveEventLoop) {
        log::debug!("Suspended");
        self.editor.suspend();
        self.gui_editor = None;
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
        if let Some(gui_editor) = self.gui_editor.as_mut() {
            let _pass_events_to_game = !gui_editor.update(&event);
        }
        if self.project.is_running() {
            if let Some(gui) = self.gui.as_mut() {
                let mut event = event.clone();
                adjust_event_for_window(
                    &mut event,
                    self.editor.game_window().position(),
                    gui.egui_ctx.pixels_per_point(),
                );
                let _pass_events_to_game = !gui.update(&event);
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                log::info!("WindowEvent::CloseRequested");
                self.project.exit(&mut self.local_data, self.scene_camera);
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
                self.project.maintain_asset_dir();
                if let Some(renderer) = self.windows.get_primary_renderer_mut() {
                    if let Some(window_title) = self.window_title.take() {
                        renderer.window().set_title(&window_title);
                    }

                    let window_size = renderer.window().inner_size();
                    if window_size.width == 0 || window_size.height == 0 {
                        return; // Prevent "Failed to recreate swapchain: ImageExtentZeroLengthDimensions" in renderer.acquire().unwrap()
                    }
                    let mut gpu_future = renderer.acquire(None, |_| {}).unwrap();

                    let gui_editor = self.gui_editor.as_mut().unwrap();
                    let mut world_data = self.project.app().map(|app| {
                        let mut world_data = WorldData::default();
                        app.command(Command::Save(&mut world_data));
                        world_data
                    });
                    self.editor.ui(
                        gui_editor,
                        &mut self.gui,
                        &self.context,
                        renderer,
                        &mut self.project,
                        &mut world_data,
                        &mut self.local_data,
                        &mut self.scene_camera,
                        &mut self.window_title,
                    );

                    let is_running = self.project.is_running();
                    if let Some(app) = self.project.app() {
                        if self.gui.is_none() {
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
                        let gui = self.gui.as_mut().unwrap();
                        let mut raw_input = gui.egui_winit.take_egui_input(renderer.window());
                        let screen_size = self.editor.game_window().size();
                        raw_input.screen_rect = Some(egui::Rect::from_x_y_ranges(
                            0.0..=(screen_size.x as f32),
                            0.0..=(screen_size.y as f32),
                        ));
                        gui.egui_ctx.options_mut(|options| {
                            options.zoom_factor = gui_editor.egui_ctx.zoom_factor()
                        });
                        gui.egui_ctx.begin_pass(raw_input);

                        if let Some(world_data) = world_data.as_mut() {
                            app.command(Command::Load(world_data));
                        }

                        app.update(UpdateInfo {
                            update: is_running,
                            ctx: &gui.egui_ctx,
                        });

                        if let Some(image) = self.editor.game_window().image() {
                            let draw_future = app.draw(DrawInfo {
                                before_future: vulkano::sync::now(self.context.device().clone())
                                    .boxed(),
                                context: &self.context,
                                renderer: &renderer,
                                image: image.clone(),
                                window_size: self.editor.game_window().pixel(),
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
                                vulkano::sync::now(self.context.device().clone()).boxed(), // TODO: use draw_future
                                image.clone(),
                            );
                            let copy_future = self.editor.game_window().copy_image(
                                self.command_buffer_allocator.clone(),
                                self.context.graphics_queue().clone(),
                                gui_future,
                            );
                            gpu_future = gpu_future.join(copy_future).boxed();
                        }

                        if let Some(image) = self.editor.scene_window().image() {
                            let draw_future = app.draw(DrawInfo {
                                before_future: vulkano::sync::now(self.context.device().clone())
                                    .boxed(),
                                context: &self.context,
                                renderer: &renderer,
                                image: image.clone(),
                                window_size: self.editor.scene_window().pixel(),
                                editor_info: Some(EditorInfo {
                                    camera: &self.scene_camera,
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
                            let copy_future = self.editor.scene_window().copy_image(
                                self.command_buffer_allocator.clone(),
                                self.context.graphics_queue().clone(),
                                vulkano::sync::now(self.context.device().clone()).boxed(), // TODO: use draw_future
                            );
                            gpu_future = gpu_future.join(copy_future).boxed();
                        }
                    }

                    gpu_future =
                        gui_editor.draw_on_image(gpu_future, renderer.swapchain_image_view());

                    renderer.present(gpu_future, true);

                    renderer.window().request_redraw();
                }
            }
            _ => (),
        }
    }
}

fn adjust_event_for_window(event: &mut WindowEvent, window_position: Vec2, scale_factor: f32) {
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
        | WindowEvent::KeyboardInput { .. }
        | WindowEvent::ModifiersChanged(_)
        | WindowEvent::Ime(_)
        | WindowEvent::MouseWheel { .. }
        | WindowEvent::MouseInput { .. }
        | WindowEvent::TouchpadPressure { .. }
        | WindowEvent::AxisMotion { .. }
        | WindowEvent::ThemeChanged(_)
        | WindowEvent::Occluded(_)
        | WindowEvent::ActivationTokenDone { .. }
        | WindowEvent::PinchGesture { .. }
        | WindowEvent::PanGesture { .. }
        | WindowEvent::DoubleTapGesture { .. }
        | WindowEvent::RotationGesture { .. }
        | WindowEvent::ScaleFactorChanged { .. }
        | WindowEvent::RedrawRequested => (),
    }
}
