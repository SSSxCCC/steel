use std::{process::Command, path::PathBuf};

use steel_common::{Engine, DrawInfo};
use libloading::{Library, Symbol};
use log::{Log, LevelFilter, SetLoggerError};
use egui_winit_vulkano::{Gui, GuiConfig};
use vulkano_util::{window::{VulkanoWindows, WindowDescriptor}, context::VulkanoContext};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
};

mod ui;

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

use crate::ui::Editor;

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
    // graphics
    let context = VulkanoContext::default();
    let mut windows = VulkanoWindows::default();

    // egui
    let mut gui = None;
    let mut editor = Editor::new();

    // project
    let mut project: Option<Project> = None;

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
                editor.ui(gui, &context, renderer, &mut project);

                let mut gpu_future = renderer.acquire().unwrap();

                if let Some(project) = project.as_mut() {
                    project.data.as_mut().unwrap().engine.update(); // TODO: project.data is None if project failed to compile
                    gpu_future = project.data.as_mut().unwrap().engine.draw(DrawInfo {
                        before_future: gpu_future, context: &context, renderer: &renderer,
                        image: editor.scene_image().as_ref().unwrap().clone(), window_size: editor.scene_size(),
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

struct ProjectData {
    engine: Box<dyn Engine>,
    library: Library, // Library must be destroyed after Engine
}

pub struct Project {
    path: PathBuf,
    data: Option<ProjectData>,
}

impl Project {
    fn new(path: PathBuf) -> Self {
        Project { path, data: None }
    }

    fn compile(&mut self) {
        let mut complie_process = Command::new("cargo")
            .arg("build")
            .current_dir(&self.path)
            .spawn()
            .unwrap();

        complie_process.wait().unwrap();

        let lib_path = self.path.join("target/debug/steel.dll");
        let library: Library = unsafe { Library::new(&lib_path) }.unwrap();

        let setup_logger_fn: Symbol<fn(&'static dyn Log, LevelFilter) -> Result<(), SetLoggerError>> = unsafe { library.get(b"setup_logger") }.unwrap();
        setup_logger_fn(log::logger(), log::max_level()).unwrap();

        let create_engine_fn: Symbol<fn() -> Box<dyn Engine>> = unsafe { library.get(b"create") }.unwrap();
        let mut engine = create_engine_fn();
        engine.init();

        self.data = Some(ProjectData { engine, library });
    }
}
