use std::{path::PathBuf, sync::Arc, time::Instant};
use egui_winit_vulkano::Gui;
use glam::{Vec2, Vec3, Vec4};
use shipyard::EntityId;
use steel_common::{Command, Engine, EntityData, Limit, Range, Value, WorldData};
use vulkano::image::{ImageViewAbstract, StorageImage, ImageUsage};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};

use crate::{project::Project, utils::LocalData};

pub struct Editor {
    scene_window: ImageWindow,
    game_window: ImageWindow,
    #[allow(unused)] demo_windows: egui_demo_lib::DemoWindows,
    show_open_project_dialog: bool,
    project_path: PathBuf,
    fps_counter: FpsCounter,
    selected_entity: EntityId,
}

impl Editor {
    pub fn new(local_data: &LocalData) -> Self {
        Editor { scene_window: ImageWindow::new("Scene"), game_window: ImageWindow::new("Game"),
            demo_windows: egui_demo_lib::DemoWindows::default(), show_open_project_dialog: false,
            project_path: local_data.last_open_project_path.clone(), fps_counter: FpsCounter::new(),
            selected_entity: EntityId::dead() }
    }

    pub fn ui(&mut self, gui: &mut Gui, context: &VulkanoContext, renderer: &VulkanoWindowRenderer,
            project: &mut Project, local_data: &mut LocalData, world_data: Option<&mut WorldData>) {
        gui.immediate_ui(|gui| {
            let ctx = gui.context();

            // display egui demo windows
            //self.demo_windows.ui(&ctx);

            self.open_project_dialog(&ctx, gui, project, local_data);
            self.menu_bars(&ctx, gui, project);

            if project.is_compiled() {
                self.scene_window.layer = None;
                self.game_window.layer = None;
                self.scene_window.ui(&ctx, gui, context, renderer);
                if project.is_running() {
                    self.game_window.ui(&ctx, gui, context, renderer);
                }
            } else if project.is_open() {
                self.compile_error_dialog(&ctx);
            }

            if let Some(world_data) = world_data {
                if let Some(engine) = project.engine() {
                    self.entity_component_view(&ctx, world_data, engine);
                }
            }
        });
    }

    fn open_project_dialog(&mut self, ctx: &egui::Context, gui: &mut Gui, project: &mut Project, local_data: &mut LocalData) {
        let mut show = self.show_open_project_dialog;
        egui::Window::new("Open Project").open(&mut show).show(&ctx, |ui| {
            ui.horizontal(|ui| {
                let mut path_str = self.project_path.display().to_string();
                ui.text_edit_singleline(&mut path_str);
                self.project_path = path_str.into();

                if ui.button("Browse").clicked() {
                    log::info!("Open FileDialog");
                    let folder = rfd::FileDialog::new()
                        .set_directory(&self.project_path)
                        .pick_folder();
                    log::info!("Close FileDialog, folder={folder:?}");
                    if let Some(folder) = folder {
                        self.project_path = folder;
                    }
                }
            });
            if ui.button("Open").clicked() {
                log::info!("Open project, path={}", self.project_path.display());
                self.scene_window.close(Some(gui));
                self.game_window.close(Some(gui));
                project.open(self.project_path.clone(), local_data);
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
                    if project.is_compiled() && !project.is_running() {
                        if ui.button("Save").clicked() {
                            log::info!("Menu->Project->Save");
                            project.save_to_file();
                            ui.close_menu();
                        }
                        if ui.button("Load").clicked() {
                            log::info!("Menu->Project->Load");
                            project.load_from_file();
                            ui.close_menu();
                        }
                    }
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
                    if project.is_compiled() && !project.is_running() {
                        ui.menu_button("Export", |ui| {
                            if ui.button("Windows").clicked() {
                                log::info!("Menu->Project->Export->Windows");
                                project.export_windows();
                                // TODO: display a dialog to show export result
                                ui.close_menu();
                            }
                            if ui.button("Android").clicked() {
                                log::info!("Menu->Project->Export->Android");
                                project.export_android();
                                // TODO: display a dialog to show export result
                                ui.close_menu();
                            }
                        });
                    }
                });
                if project.is_compiled() {
                    ui.menu_button("Run", |ui| {
                        let text = if project.is_running() { "Stop" } else { "Start" };
                        if ui.button(text).clicked() {
                            log::info!("Menu->Run->{text}");
                            if project.is_running() {
                                project.load_from_memory();
                            } else {
                                project.save_to_memory();
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

    fn entity_component_view(&mut self, ctx: &egui::Context, world_data: &mut WorldData, engine: &mut Box<dyn Engine>) {
        egui::Window::new("Entities").show(&ctx, |ui| {
            egui::Grid::new("Entities").show(ui, |ui| {
                for (id, entity_data) in &world_data.entities {
                    if ui.selectable_label(self.selected_entity == *id, Self::entity_label(id, entity_data)).clicked() {
                        self.selected_entity = *id;
                    }
                    if ui.button("-").clicked() {
                        engine.command(Command::DestroyEntity(*id));
                    }
                    ui.end_row();
                }
            });
            if ui.button("+").clicked() {
                engine.command(Command::CreateEntity);
            }
        });

        if let Some(entity_data) = world_data.entities.get_mut(&self.selected_entity) {
            egui::Window::new("Components").show(&ctx, |ui| {
                for (component_name, component_data) in &mut entity_data.components {
                    ui.horizontal(|ui| {
                        ui.label(component_name);
                        if ui.button("-").clicked() {
                            engine.command(Command::DestroyComponent(self.selected_entity, component_name));
                        }
                    });
                    for (name, value) in &mut component_data.values {
                        ui.horizontal(|ui| {
                            ui.label(name);
                            if let Some(Limit::ReadOnly) = component_data.limits.get(name) {
                                Self::color_label(ui, egui::Color32::BLACK, match value {
                                    Value::Int32(v) => format!("{v}"),
                                    Value::Float32(v) => format!("{v}"),
                                    Value::String(v) => format!("{v}"),
                                    Value::Vec2(v) => format!("{v}"),
                                    Value::Vec3(v) => format!("{v}"),
                                    Value::Vec4(v) => format!("{v}"),
                                });
                            } else {
                                match value {
                                    Value::Int32(v) => {
                                        if let Some(Limit::Int32Enum(int_enum)) = component_data.limits.get(name) {
                                            if int_enum.len() > 0 {
                                                let mut i = int_enum.iter().enumerate().find_map(|(i, (int, _))| {
                                                    if v == int { Some(i) } else { None }
                                                }).unwrap_or(0);
                                                // Use entity + component_name + value_name as id to make sure that every id is unique
                                                egui::ComboBox::from_id_source(format!("{:?} {} {}", self.selected_entity, component_name, name))
                                                    .show_index(ui, &mut i, int_enum.len(), |i| &int_enum[i].1);
                                                *v = int_enum[i].0;
                                            } else {
                                                Self::color_label(ui, egui::Color32::RED, "zero length int_enum!");
                                            }
                                        } else {
                                            let mut drag_value = egui::DragValue::new(v);
                                            if let Some(Limit::Int32Range(range)) = component_data.limits.get(name) {
                                                let min = if range.min_include { range.min } else { range.min + 1 };
                                                let max = if range.max_include { range.max } else { range.max - 1 };
                                                drag_value = drag_value.clamp_range(min..=max);
                                            }
                                            ui.add(drag_value);
                                        }
                                    }
                                    Value::String(v) => {
                                        if let Some(Limit::StringMultiline) = component_data.limits.get(name) {
                                            ui.text_edit_multiline(v);
                                        } else {
                                            ui.text_edit_singleline(v);
                                        }
                                    }
                                    Value::Float32(v) => {
                                        if let Some(Limit::Float32Rotation) = component_data.limits.get(name) {
                                            ui.drag_angle(v);
                                            if *v < 0.0 {
                                                *v += 2.0 * std::f32::consts::PI;
                                            } else if *v >= 2.0 * std::f32::consts::PI {
                                                *v -= 2.0 * std::f32::consts::PI;
                                            }
                                        } else {
                                            Self::drag_float32(ui, v, match component_data.limits.get(name) {
                                                Some(Limit::Float32Range(range)) => Some(range),
                                                _ => None,
                                            });
                                        }
                                    }
                                    Value::Vec2(v) => {
                                        ui.horizontal(|ui| {
                                            if let Some(Limit::Float32Rotation) = component_data.limits.get(name) {
                                                ui.drag_angle(&mut v.x);
                                                ui.drag_angle(&mut v.y);
                                            } else {
                                                let (range_x, range_y) = match component_data.limits.get(name) {
                                                    Some(Limit::Vec2Range { x, y }) => (Some(x), Some(y)),
                                                    _ => (None, None),
                                                };
                                                Self::drag_float32(ui, &mut v.x, range_x);
                                                Self::drag_float32(ui, &mut v.y, range_y);
                                            }
                                        });
                                    }
                                    Value::Vec3(v) => {
                                        ui.horizontal(|ui| {
                                            if let Some(Limit::Vec3Color) = component_data.limits.get(name) {
                                                let mut color = v.to_array();
                                                ui.color_edit_button_rgb(&mut color);
                                                *v = Vec3::from_array(color);
                                            } else if let Some(Limit::Float32Rotation) = component_data.limits.get(name) {
                                                ui.drag_angle(&mut v.x);
                                                ui.drag_angle(&mut v.y);
                                                ui.drag_angle(&mut v.z);
                                            } else {
                                                let (range_x, range_y, range_z) = match component_data.limits.get(name) {
                                                    Some(Limit::Vec3Range { x, y, z }) => (Some(x), Some(y), Some(z)),
                                                    _ => (None, None, None),
                                                };
                                                Self::drag_float32(ui, &mut v.x, range_x);
                                                Self::drag_float32(ui, &mut v.y, range_y);
                                                Self::drag_float32(ui, &mut v.z, range_z);
                                            }
                                        });
                                    }
                                    Value::Vec4(v) => {
                                        ui.horizontal(|ui| {
                                            if let Some(Limit::Vec4Color) = component_data.limits.get(name) {
                                                let mut color = v.to_array();
                                                ui.color_edit_button_rgba_unmultiplied(&mut color);
                                                *v = Vec4::from_array(color);
                                            } else if let Some(Limit::Float32Rotation) = component_data.limits.get(name) {
                                                ui.drag_angle(&mut v.x);
                                                ui.drag_angle(&mut v.y);
                                                ui.drag_angle(&mut v.z);
                                                ui.drag_angle(&mut v.w);
                                            } else {
                                                let (range_x, range_y, range_z, range_w) = match component_data.limits.get(name) {
                                                    Some(Limit::Vec4Range { x, y, z, w }) => (Some(x), Some(y), Some(z), Some(w)),
                                                    _ => (None, None, None, None),
                                                };
                                                Self::drag_float32(ui, &mut v.x, range_x);
                                                Self::drag_float32(ui, &mut v.y, range_y);
                                                Self::drag_float32(ui, &mut v.z, range_z);
                                                Self::drag_float32(ui, &mut v.w, range_w);
                                            }
                                        });
                                    }
                                }
                            }
                        });
                    }
                    if component_name == "EntityInfo" {
                        ui.horizontal(|ui| {
                            ui.label("id");
                            Self::color_label(ui, egui::Color32::BLACK, format!("{:?}", self.selected_entity));
                        });
                    }
                    ui.separator();
                } // for entity_data.components

                let mut components = Vec::new();
                engine.command(Command::GetComponents(&mut components));
                ui.menu_button("+", |ui| {
                    for component in components {
                        if ui.button(component).clicked() {
                            engine.command(Command::CreateComponent(self.selected_entity, component));
                            ui.close_menu();
                        }
                    }
                });
            });
        }
    }

    fn entity_label(id: &EntityId, entity_data: &EntityData) -> impl Into<egui::WidgetText> {
        if let Some(entity_info) = entity_data.components.get("EntityInfo") {
            if let Some(Value::String(s)) = entity_info.values.get("name") {
                if !s.is_empty() {
                    return format!("{s}");
                }
            }
        }
        format!("{:?}", id)
    }

    fn color_label(ui: &mut egui::Ui, color: egui::Color32, text: impl Into<egui::WidgetText>) {
        egui::Frame::none()
            .inner_margin(egui::style::Margin::symmetric(3.0, 1.0))
            .rounding(egui::Rounding::same(3.0))
            .fill(color)
            .show(ui, |ui| {
                ui.label(text);
            });
    }

    fn drag_float32(ui: &mut egui::Ui, v: &mut f32, range: Option<&Range<f32>>) {
        let mut drag_value = egui::DragValue::new(v);
        if let Some(range) = range {
            drag_value = drag_value.clamp_range(range.min..=range.max);
            ui.add(drag_value);
            if !range.min_include && *v <= range.min {
                *v = range.min + f32::EPSILON;
            }
            if !range.max_include && *v >= range.max {
                *v = range.max - f32::EPSILON;
            }
        } else {
            ui.add(drag_value);
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

    pub fn scene_focus(&self) -> bool {
        self.scene_window.layer.is_some_and(|this| !self.game_window.layer.is_some_and(|other| other > this))
    }

    pub fn game_image(&self) -> &Option<Arc<dyn ImageViewAbstract + Send + Sync>> {
        &self.game_window.image
    }

    pub fn game_size(&self) -> Vec2 {
        self.game_window.size
    }

    #[allow(unused)]
    pub fn game_focus(&self) -> bool {
        self.game_window.layer.is_some_and(|this| !self.scene_window.layer.is_some_and(|other| other > this))
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
    layer: Option<usize>, // Warning: the value of layer is undefined if !project.is_compiled()
}

impl ImageWindow {
    fn new(title: impl Into<String>) -> Self {
        ImageWindow { title: title.into(), image: None, texture_id: None, size: Vec2::ZERO, layer: None }
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
            let r = ui.image(self.texture_id.unwrap(), available_size);
            self.layer = ctx.memory(|mem| {
                match mem.focus() {
                    Some(_) => None, // We should not have focus if any widget has keyboard focus
                    None => mem.layer_ids().position(|layer_id| layer_id == r.layer_id),
                }
            });
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
