use steel_common::{Engine, DrawInfo};
use libloading::{Library, Symbol};
use log::{Log, LevelFilter, SetLoggerError};
use glam::Vec2;
use egui_winit_vulkano::{Gui, GuiConfig};
use vulkano::image::{StorageImage, ImageUsage};
use vulkano_util::{window::{VulkanoWindows, WindowDescriptor}, context::VulkanoContext};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
};

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
    env_logger::builder().filter_level(log::LevelFilter::Trace).parse_default_env().init();
    let event_loop = EventLoopBuilder::new().build();
    _main(event_loop);
}

fn _main(event_loop: EventLoop<()>) {
    let context = VulkanoContext::default();
    let mut windows = VulkanoWindows::default();
    let mut gui = None;
    let mut demo_windows = egui_demo_lib::DemoWindows::default();
    let mut scene_image = None;
    let mut scene_texture_id = None;
    let mut scene_size = Vec2::ZERO;
    let mut project: Option<Project> = None;
    let mut project_path = String::from("../../examples/test-project/target/debug/steel.dll");
    let mut open_project_window = false;

    log::warn!("Start main loop!");
    event_loop.run(move |event, event_loop, control_flow| match event {
        Event::Resumed => {
            log::info!("Event::Resumed");
            windows.create_window(&event_loop, &context,
                &WindowDescriptor::default(), |_|{});
            let renderer = windows.get_primary_renderer().unwrap();
            gui = Some(Gui::new(&event_loop, renderer.surface(),
                renderer.graphics_queue(),
                renderer.swapchain_format(),
                GuiConfig { is_overlay: false, ..Default::default() }));
        }
        Event::Suspended => {
            log::info!("Event::Suspended");
            scene_texture_id = None;
            scene_image = None;
            gui = None;
            windows.remove_renderer(windows.primary_window_id().unwrap());
        }
        Event::WindowEvent { event , .. } => {
            if let Some(gui) = gui.as_mut() {
                let _pass_events_to_game = !gui.update(&event);
            }
            match event {
                WindowEvent::CloseRequested => {
                    log::info!("WindowEvent::CloseRequested");
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Resized(_) => {
                    log::info!("WindowEvent::Resized");
                    if let Some(renderer) = windows.get_primary_renderer_mut() { renderer.resize() }
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    log::info!("WindowEvent::ScaleFactorChanged");
                    if let Some(renderer) = windows.get_primary_renderer_mut() { renderer.resize() }
                }
                _ => ()
            }
        }
        Event::RedrawRequested(_) => {
            log::info!("Event::RedrawRequested");
            if let Some(renderer) = windows.get_primary_renderer_mut() {
                let gui = gui.as_mut().unwrap();
                gui.immediate_ui(|gui| {
                    let ctx = gui.context();
                    demo_windows.ui(&ctx);

                    let mut open = open_project_window;
                    egui::Window::new("Open Project").open(&mut open).show(&ctx, |ui| {
                        ui.text_edit_singleline(&mut project_path);
                        if ui.button("Open").clicked() {
                            log::info!("Open project, path={project_path}");
                            scene_image = None;
                            scene_texture_id = None;
                            project = None; // prevent a library from being loaded twice at same time
                            project = Some(Project::new(project_path.clone()));
                            open_project_window = false;
                        }
                    });
                    open_project_window &= open;

                    egui::TopBottomPanel::top("my_top_panel").show(&ctx, |ui| {
                        egui::menu::bar(ui, |ui| {
                            ui.menu_button("Project", |ui| {
                                if ui.button("Open project").clicked() {
                                    log::info!("Open project");
                                    open_project_window = true;
                                    ui.close_menu();
                                }
                                if project.is_some() {
                                    if ui.button("Close project").clicked() {
                                        log::info!("Close project");
                                        scene_image = None;
                                        scene_texture_id = None;
                                        project = None;
                                        ui.close_menu();
                                    }
                                }
                            });
                        });
                    });

                    if let Some(project) = project.as_mut() {
                        egui::Window::new("Scene").resizable(true).show(&ctx, |ui| {
                            let available_size = ui.available_size();
                            if scene_image.is_none() || scene_size.x != available_size.x || scene_size.y != available_size.y {
                                (scene_size.x, scene_size.y) = (available_size.x, available_size.y);
                                scene_image = Some(StorageImage::general_purpose_image_view(
                                    context.memory_allocator(),
                                    context.graphics_queue().clone(),
                                    [scene_size.x as u32, scene_size.y as u32],
                                    renderer.swapchain_format(),
                                    ImageUsage::SAMPLED | ImageUsage::COLOR_ATTACHMENT,
                                ).unwrap());
                                if let Some(scene_texture_id) = scene_texture_id {
                                    gui.unregister_user_image(scene_texture_id);
                                }
                                scene_texture_id = Some(gui.register_user_image_view(
                                    scene_image.as_ref().unwrap().clone(), Default::default()));
                                log::info!("Created scene image, scene_size={scene_size}");
                            }
                            ui.image(scene_texture_id.unwrap(), available_size);
                        });
                    }
                });

                let mut gpu_future = renderer.acquire().unwrap();

                if let Some(project) = project.as_mut() {
                    project.engine.update();
                    gpu_future = project.engine.draw(DrawInfo {
                        before_future: gpu_future, context: &context, renderer: &renderer,
                        image: scene_image.as_ref().unwrap().clone(), window_size: scene_size,
                    });
                }

                let gpu_future = gui.draw_on_image(gpu_future, renderer.swapchain_image_view());

                renderer.present(gpu_future, true);
            }
        }
        Event::MainEventsCleared => {
            log::info!("Event::MainEventsCleared");
            if let Some(renderer) = windows.get_primary_renderer() { renderer.window().request_redraw() }
        }
        _ => (),
    });
}

struct Project {
    path: String,
    engine: Box<dyn Engine>,
    library: Library, // Library must be destroyed after Engine
}

impl Project {
    fn new(path: String) -> Self {
        let library: Library = unsafe { Library::new(&path) }.unwrap();

        let setup_logger_fn: Symbol<fn(&'static dyn Log, LevelFilter) -> Result<(), SetLoggerError>> = unsafe { library.get(b"setup_logger") }.unwrap();
        setup_logger_fn(log::logger(), log::max_level()).unwrap();

        let create_engine_fn: Symbol<fn() -> Box<dyn Engine>> = unsafe { library.get(b"create") }.unwrap();
        let mut engine = create_engine_fn();
        engine.init();

        Project { path, library, engine }
    }
}
