use std::{sync::Arc, path::PathBuf, fs, time::Instant};
use egui_winit_vulkano::Gui;
use glam::Vec2;
use steel_common::WorldData;
use vulkano::image::{ImageViewAbstract, StorageImage, ImageUsage};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};

use crate::project::Project;

pub struct Editor {
    scene_image: Option<Arc<dyn ImageViewAbstract + Send + Sync>>,
    scene_texture_id: Option<egui::TextureId>,
    scene_size: Vec2,

    demo_windows: egui_demo_lib::DemoWindows,
    show_open_project_dialog: bool,
    project_path: PathBuf,
    fps_counter: FpsCounter,
}

impl Editor {
    pub fn new() -> Self {
        let mut project_path = fs::canonicalize("examples/test-project").unwrap();
        // the windows path prefix "\\?\" makes cargo build fail in std::process::Command
        const WINDOWS_PATH_PREFIX: &str = r#"\\?\"#;
        if project_path.display().to_string().starts_with(WINDOWS_PATH_PREFIX) {
            // TODO: convert PathBuf to String and back to PathBuf may lose data, find a better way to do this
            project_path = PathBuf::from(&project_path.display().to_string()[WINDOWS_PATH_PREFIX.len()..]);
        };
        Editor { scene_image: None, scene_texture_id: None, scene_size: Vec2::ZERO,
            demo_windows: egui_demo_lib::DemoWindows::default(), show_open_project_dialog: false,
            project_path, fps_counter: FpsCounter::new() }
    }

    pub fn ui(&mut self, gui: &mut Gui, context: &VulkanoContext, renderer: &VulkanoWindowRenderer,
            project: &mut Option<Project>, world_data: Option<&mut WorldData>) {
        gui.immediate_ui(|gui| {
            let ctx = gui.context();

            self.demo_windows.ui(&ctx);

            self.open_project_dialog(&ctx, project);
            self.menu_bars(&ctx, project);
            self.scene_window(&ctx, project, context, renderer, gui);
        });
    }

    fn open_project_dialog(&mut self, ctx: &egui::Context, project: &mut Option<Project>) {
        let mut show = self.show_open_project_dialog;
        egui::Window::new("Open Project").open(&mut show).show(&ctx, |ui| {
            let mut path_str = self.project_path.display().to_string();
            ui.text_edit_singleline(&mut path_str);
            self.project_path = path_str.into();
            if ui.button("Open").clicked() {
                log::info!("Open project, path={}", self.project_path.display());
                self.scene_image = None;
                self.scene_texture_id = None;
                *project = None; // prevent a library from being loaded twice at same time
                *project = Some(Project::new(self.project_path.clone()));
                project.as_mut().unwrap().compile();
                self.show_open_project_dialog = false;
            }
        });
        self.show_open_project_dialog &= show;
    }

    fn menu_bars(&mut self, ctx: &egui::Context, project: &mut Option<Project>) {
        egui::TopBottomPanel::top("my_top_panel").show(&ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Project", |ui| {
                    if ui.button("Open project").clicked() {
                        log::info!("Open project");
                        self.show_open_project_dialog = true;
                        ui.close_menu();
                    }
                    if project.is_some() {
                        if ui.button("Close project").clicked() {
                            log::info!("Close project");
                            self.scene_image = None;
                            self.scene_texture_id = None;
                            *project = None;
                            ui.close_menu();
                        }
                    }
                });
                self.fps_counter.update();
                ui.label(format!("fps: {:.2}", self.fps_counter.fps));
            });
        });
    }

    fn scene_window(&mut self, ctx: &egui::Context, project: &mut Option<Project>, context: &VulkanoContext, renderer: &VulkanoWindowRenderer, gui: &mut Gui) {
        if let Some(project) = project.as_mut() {
            egui::Window::new("Scene").resizable(true).show(&ctx, |ui| {
                let available_size = ui.available_size();
                if self.scene_image.is_none() || self.scene_size.x != available_size.x || self.scene_size.y != available_size.y {
                    (self.scene_size.x, self.scene_size.y) = (available_size.x, available_size.y);
                    self.scene_image = Some(StorageImage::general_purpose_image_view(
                        context.memory_allocator(),
                        context.graphics_queue().clone(),
                        [self.scene_size.x as u32, self.scene_size.y as u32],
                        renderer.swapchain_format(),
                        ImageUsage::SAMPLED | ImageUsage::COLOR_ATTACHMENT,
                    ).unwrap());
                    if let Some(scene_texture_id) = self.scene_texture_id {
                        gui.unregister_user_image(scene_texture_id);
                    }
                    self.scene_texture_id = Some(gui.register_user_image_view(
                        self.scene_image.as_ref().unwrap().clone(), Default::default()));
                    log::info!("Created scene image, scene_size={}", self.scene_size);
                }
                ui.image(self.scene_texture_id.unwrap(), available_size);
            });
        }
    }

    pub fn suspend(&mut self) {
        self.scene_texture_id = None;
        self.scene_image = None;
    }

    pub fn scene_image(&self) -> &Option<Arc<dyn ImageViewAbstract + Send + Sync>> {
        &self.scene_image
    }

    pub fn scene_size(&self) -> Vec2 {
        self.scene_size
    }
}

struct FpsCounter {
    start: Instant,
    frame: u32,
    fps: f32,
}

impl FpsCounter {
    fn new() -> Self {
        FpsCounter { start: Instant::now(), frame: 0, fps: 0.0 }
    }

    fn update(&mut self) {
        self.frame += 1;
        let now = Instant::now();
        let duration = now.duration_since(self.start).as_secs_f32();
        if duration >= 1.0 {
            self.fps = self.frame as f32 / duration;
            self.frame = 0;
            self.start = now;
        }
    }
}
