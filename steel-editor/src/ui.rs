use std::{sync::Arc, path::PathBuf, fs, time::Instant};
use egui_winit_vulkano::Gui;
use glam::Vec2;
use shipyard::EntityId;
use steel_common::{WorldData, Value};
use vulkano::image::{ImageViewAbstract, StorageImage, ImageUsage};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};

use crate::project::Project;

pub struct Editor {
    scene_window: ImageWindow,
    game_window: ImageWindow,
    demo_windows: egui_demo_lib::DemoWindows,
    show_open_project_dialog: bool,
    project_path: PathBuf,
    fps_counter: FpsCounter,
    selected_entity: EntityId,
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
        Editor { scene_window: ImageWindow::new("Scene"), game_window: ImageWindow::new("Game"),
            demo_windows: egui_demo_lib::DemoWindows::default(), show_open_project_dialog: false,
            project_path, fps_counter: FpsCounter::new(), selected_entity: EntityId::dead() }
    }

    pub fn ui(&mut self, gui: &mut Gui, context: &VulkanoContext, renderer: &VulkanoWindowRenderer,
            project: &mut Project, world_data: Option<&mut WorldData>) {
        gui.immediate_ui(|gui| {
            let ctx = gui.context();

            self.demo_windows.ui(&ctx);

            self.open_project_dialog(&ctx, gui, project);
            self.menu_bars(&ctx, gui, project);

            if project.is_compiled() {
                self.scene_window.ui(&ctx, gui, context, renderer);
                if project.is_running() {
                    self.game_window.ui(&ctx, gui, context, renderer);
                }
            } else if project.is_open() {
                self.compile_error_dialog(&ctx);
            }

            if let Some(world_data) = world_data {
                self.entity_component_view(&ctx, world_data);
            }
        });
    }

    fn open_project_dialog(&mut self, ctx: &egui::Context, gui: &mut Gui, project: &mut Project) {
        let mut show = self.show_open_project_dialog;
        egui::Window::new("Open Project").open(&mut show).show(&ctx, |ui| {
            let mut path_str = self.project_path.display().to_string();
            ui.text_edit_singleline(&mut path_str);
            self.project_path = path_str.into();
            if ui.button("Open").clicked() {
                log::info!("Open project, path={}", self.project_path.display());
                self.scene_window.close(Some(gui));
                self.game_window.close(Some(gui));
                project.open(self.project_path.clone());
                project.compile();
                self.show_open_project_dialog = false;
            }
        });
        self.show_open_project_dialog &= show;
    }

    fn compile_error_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("Compile error!").show(&ctx, |ui| {
            ui.label("We have some compile issues, \
                please solve them according to the terminal output, \
                then click 'Project -> Compile' to try again.");
        });
    }

    fn menu_bars(&mut self, ctx: &egui::Context, gui: &mut Gui, project: &mut Project) {
        egui::TopBottomPanel::top("my_top_panel").show(&ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Project", |ui| {
                    if ui.button("Open").clicked() {
                        log::info!("Menu->Project->Open");
                        self.show_open_project_dialog = true;
                        ui.close_menu();
                    }
                    if project.is_open() {
                        if ui.button("Close").clicked() {
                            log::info!("Menu->Project->Close");
                            self.scene_window.close(Some(gui));
                            self.game_window.close(Some(gui));
                            project.close();
                            ui.close_menu();
                        }
                        if ui.button("Compile").clicked() {
                            log::info!("Menu->Project->Compile");
                            self.scene_window.close(Some(gui));
                            self.game_window.close(Some(gui));
                            project.compile();
                            ui.close_menu();
                        }
                    }
                });
                if project.is_compiled() {
                    ui.menu_button("Run", |ui| {
                        let text = if project.is_running() { "Stop" } else { "Start" };
                        if ui.button(text).clicked() {
                            log::info!("Menu->Run->{text}");
                            if project.is_running() {
                                project.load();
                            } else {
                                project.save();
                            }
                            project.set_running(!project.is_running());
                            ui.close_menu();
                        }
                    });
                }
                self.fps_counter.update();
                ui.label(format!("fps: {:.2}", self.fps_counter.fps));
            });
        });
    }

    fn entity_component_view(&mut self, ctx: &egui::Context, world_data: &mut WorldData) {
        egui::Window::new("Entities").show(&ctx, |ui| {
            for entity_data in &mut world_data.entities {
                if ui.selectable_label(self.selected_entity == entity_data.id, format!("{:?}", entity_data.id)).clicked() {
                    self.selected_entity = entity_data.id;
                }
            }
        });

        if let Some(index) = world_data.id_index_map.get(&self.selected_entity) {
            let entity_data = &mut world_data.entities[*index];
            egui::Window::new("Components").show(&ctx, |ui| {
                for component_data in &mut entity_data.components {
                    ui.label(&component_data.name);
                    for variant in &mut component_data.variants {
                        ui.horizontal(|ui| {
                            ui.label(&variant.name);
                            match &mut variant.value {
                                Value::Float32(v) => {
                                    ui.label(format!("{}", v));
                                }
                                Value::Int32(v) => {
                                    ui.label(format!("{}", v));
                                }
                                Value::String(v) => {
                                    ui.label(format!("{}", v));
                                }
                                Value::Vec2(v) => {
                                    ui.label(format!("{}", v));
                                }
                                Value::Vec3(v) => {
                                    ui.label(format!("{}", v));
                                }
                                Value::Vec4(v) => {
                                    ui.label(format!("{}", v));
                                }
                            }
                        });
                    }
                    ui.separator();
                }
            });
        }
    }

    pub fn suspend(&mut self) {
        self.scene_window.close(None);
        self.game_window.close(None);
    }

    pub fn scene_image(&self) -> &Option<Arc<dyn ImageViewAbstract + Send + Sync>> {
        &self.scene_window.image
    }

    pub fn scene_size(&self) -> Vec2 {
        self.scene_window.size
    }

    pub fn game_image(&self) -> &Option<Arc<dyn ImageViewAbstract + Send + Sync>> {
        &self.game_window.image
    }

    pub fn game_size(&self) -> Vec2 {
        self.game_window.size
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

struct ImageWindow {
    title: String,
    image: Option<Arc<dyn ImageViewAbstract + Send + Sync>>, // TODO: use multi-buffering
    texture_id: Option<egui::TextureId>,
    size: Vec2,
}

impl ImageWindow {
    fn new(title: impl Into<String>) -> Self {
        ImageWindow { title: title.into(), image: None, texture_id: None, size: Vec2::ZERO }
    }

    fn ui(&mut self, ctx: &egui::Context, gui: &mut Gui, context: &VulkanoContext, renderer: &VulkanoWindowRenderer) {
        egui::Window::new(&self.title).resizable(true).show(&ctx, |ui| {
            let available_size = ui.available_size();
            if self.image.is_none() || self.size.x != available_size.x || self.size.y != available_size.y {
                (self.size.x, self.size.y) = (available_size.x, available_size.y);
                self.close(Some(gui));
                self.image = Some(StorageImage::general_purpose_image_view(
                    context.memory_allocator(),
                    context.graphics_queue().clone(),
                    [self.size.x as u32, self.size.y as u32],
                    renderer.swapchain_format(),
                    ImageUsage::SAMPLED | ImageUsage::COLOR_ATTACHMENT,
                ).unwrap());
                self.texture_id = Some(gui.register_user_image_view(
                    self.image.as_ref().unwrap().clone(), Default::default()));
                log::info!("ImageWindow({}): image created, size={}", self.title, self.size);
            }
            ui.image(self.texture_id.unwrap(), available_size);
        });
    }

    fn close(&mut self, gui: Option<&mut Gui>) {
        self.image = None;
        if let (Some(gui), Some(texture_id)) = (gui, self.texture_id) {
            gui.unregister_user_image(texture_id);
        }
        self.texture_id = None;
    }
}
