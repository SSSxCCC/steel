use std::{ops::RangeInclusive, path::PathBuf, sync::Arc, time::Instant};
use egui_dock::{DockArea, DockState, NodeIndex, TabViewer};
use egui_winit_vulkano::Gui;
use glam::{UVec2, Vec2, Vec3, Vec4};
use shipyard::EntityId;
use steel_common::{data::{Data, EntityData, Limit, Value, WorldData}, engine::{Command, EditorCamera, Engine, WindowIndex}, ext::VulkanoWindowRendererExt};
use vulkano::{image::{view::ImageView, Image, ImageCreateInfo, ImageUsage}, memory::allocator::AllocationCreateInfo};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};
use winit::event::VirtualKeyCode;
use winit_input_helper::WinitInputHelper;
use crate::{project::Project, utils::LocalData};

pub struct Editor {
    scene_window: ImageWindow,
    game_window: ImageWindow,
    #[allow(unused)] demo_windows: egui_demo_lib::DemoWindows,
    use_dock: bool,
    focused_tab: Option<String>,
    show_open_project_dialog: bool,
    project_path: PathBuf,
    fps_counter: FpsCounter,
    pressed_entity: EntityId,
    selected_entity: EntityId,
    selected_unique: String,
}

impl Editor {
    pub fn new(local_data: &LocalData) -> Self {
        Editor { scene_window: ImageWindow::new("Scene"), game_window: ImageWindow::new("Game"),
            demo_windows: egui_demo_lib::DemoWindows::default(), use_dock: true, focused_tab: None, show_open_project_dialog: false,
            project_path: local_data.last_open_project_path.clone(), fps_counter: FpsCounter::new(),
            pressed_entity: EntityId::dead(), selected_entity: EntityId::dead(), selected_unique: String::new() }
    }

    pub fn ui(&mut self, gui: &mut Gui, gui_game: &mut Option<Gui>, dock_state: &mut DockState<String>, context: &VulkanoContext, renderer: &VulkanoWindowRenderer,
            project: &mut Project, local_data: &mut LocalData, world_data: &mut Option<WorldData>, input: &WinitInputHelper, editor_camera: &mut EditorCamera) {
        gui.immediate_ui(|gui| {
            let ctx = gui.context();

            // display egui demo windows
            //self.demo_windows.ui(&ctx);

            self.open_project_dialog(&ctx, gui, gui_game, context, project, local_data);
            self.menu_bars(&ctx, gui, gui_game, dock_state, context, project, world_data);

            if project.is_compiled() {
                self.update_editor_window(&ctx, project, world_data, input, editor_camera);
                self.scene_window.layer = None;
                self.game_window.layer = None;
                if !self.use_dock {
                    self.scene_window.show(&ctx, gui, context, renderer);
                    self.game_window.show(&ctx, gui, context, renderer);
                }
            } else if project.is_open() {
                Self::compile_error_dialog(&ctx);
            }

            if !self.use_dock {
                if let Some(world_data) = world_data {
                    if let Some(engine) = project.engine() {
                        self.entity_component_windows(&ctx, world_data, engine);
                        self.unique_windows(&ctx, world_data);
                    }
                }
            }

            if self.use_dock {
                DockArea::new(dock_state)
                    .style(egui_dock::Style::from_egui(gui.egui_ctx.style().as_ref()))
                    .show(&ctx, &mut MyTabViewer { add_contents: Box::new(|ui, tab| {
                        match tab.as_str() {
                            "Scene" => if project.is_compiled() {
                                self.scene_window.ui(ui, gui, context, renderer);
                            },
                            "Game" => if project.is_compiled() {
                                self.game_window.ui(ui, gui, context, renderer);
                            },
                            "Entities" => if let Some(world_data) = world_data {
                                if let Some(engine) = project.engine() {
                                    self.entities_view(ui, world_data, engine);
                                }
                            },
                            "Entity" => if let Some(world_data) = world_data {
                                if let Some(engine) = project.engine() {
                                    if let Some(entity_data) = world_data.entities.get_mut(&self.selected_entity) {
                                        self.entity_view(ui, entity_data, engine);
                                    }
                                }
                            },
                            "Uniques" => if let Some(world_data) = world_data {
                                self.uniques_view(ui, world_data);
                            },
                            "Unique" => if let Some(world_data) = world_data {
                                if let Some(unique_data) = world_data.uniques.get_mut(&self.selected_unique) {
                                    Self::data_view(ui, &self.selected_unique, unique_data);
                                }
                            },
                            _ => (),
                        }
                    }), }); // DockArea shows inside egui::CentralPanel

                self.focused_tab = dock_state.find_active_focused().map(|(_, tab)| tab.clone());
            }
        });
    }

    fn open_project_dialog(&mut self, ctx: &egui::Context, gui: &mut Gui, gui_game: &mut Option<Gui>,
            context: &VulkanoContext, project: &mut Project, local_data: &mut LocalData) {
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
                if self.project_path.display().to_string().is_empty() {
                    log::info!("Open project failed, path is empty");
                } else {
                    log::info!("Open project, path={}", self.project_path.display());
                    self.scene_window.close(Some(gui));
                    self.game_window.close(Some(gui));
                    project.open(self.project_path.clone(), local_data);
                    project.compile(gui_game, context);
                    self.show_open_project_dialog = false;
                }
            }
        });
        self.show_open_project_dialog &= show;
    }

    fn compile_error_dialog(ctx: &egui::Context) {
        egui::Window::new("Compile error!").show(&ctx, |ui| {
            ui.label("We have some compile issues, \
                please solve them according to the terminal output, \
                then click 'Project -> Compile' to try again.");
        });
    }

    fn menu_bars(&mut self, ctx: &egui::Context, gui: &mut Gui, gui_game: &mut Option<Gui>, dock_state: &mut DockState<String>,
            context: &VulkanoContext, project: &mut Project, world_data: &mut Option<WorldData>) {
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
                            project.close(gui_game);
                            ui.close_menu();
                        }
                        if ui.button("Compile").clicked() {
                            log::info!("Menu->Project->Compile");
                            self.scene_window.close(Some(gui));
                            self.game_window.close(Some(gui));
                            project.compile(gui_game, context);
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

                if project.is_compiled() && !project.is_running() {
                    ui.menu_button("Scene", |ui| {
                        if let Some(scene_path) = project.scene_relative_path() {
                            if ui.button(format!("Save ({})", scene_path.display())).clicked() {
                                log::info!("Menu->Scene->Save");
                                project.save_scene(scene_path);
                                ui.close_menu();
                            }
                        }
                        let starting_dir = if let Some(scene_path) = project.scene_absolute_path() {
                            scene_path.parent().map(|p| p.to_path_buf()).expect("scene file should be at least in the asset directory")
                        } else {
                            project.asset_dir().expect("project.asset_dir() must be some when project.is_compiled()")
                        };
                        if ui.button("Save As").clicked() {
                            log::info!("Menu->Scene->Save As");
                            let file = rfd::FileDialog::new()
                                .set_directory(&starting_dir)
                                .save_file();
                            log::info!("Close FileDialog, file={file:?}");
                            if let Some(mut file) = file {
                                file.set_extension("scene");
                                let file = project.convert_to_scene_relative_path(&file);
                                log::info!("After convert_to_scene_relative_path, file={file:?}");
                                if let Some(file) = file {
                                    project.save_scene(file);
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Load").clicked() {
                            log::info!("Menu->Scene->Load");
                            let file = rfd::FileDialog::new()
                                .set_directory(starting_dir)
                                .pick_file();
                            log::info!("Close FileDialog, file={file:?}");
                            if let Some(file) = file {
                                let file = project.convert_to_scene_relative_path(&file);
                                log::info!("After convert_to_scene_relative_path, file={file:?}");
                                if let Some(file) = file {
                                    project.load_scene(file);
                                    // We set world_data to None to prevent engine from loading outdated world_data later this frame.
                                    // But this will cause a splash screen problem due to the disappearance of the windows showing world_data for one frame.
                                    // TODO: find a way to avoid this splash screen problem.
                                    *world_data = None;
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("New").clicked() {
                            log::info!("Menu->Scene->New");
                            project.new_scene();
                            ui.close_menu();
                        }
                    });
                }

                if project.is_compiled() {
                    ui.menu_button("Run", |ui| {
                        let text = if project.is_running() { "Stop" } else { "Start" };
                        if ui.button(text).clicked() {
                            log::info!("Menu->Run->{text}");
                            if project.is_running() {
                                project.load_from_memory();
                            } else {
                                project.save_to_memory();
                                project.engine().unwrap().command(Command::ResetTime);
                                if let Some(tab) = dock_state.find_tab(&"Game".to_string()) {
                                    dock_state.set_active_tab(tab);
                                }
                            }
                            project.set_running(!project.is_running());
                            ui.close_menu();
                        }
                    });
                }

                ui.menu_button("Ui", |ui| {
                    ui.label(format!("Current Scale: {}", ctx.pixels_per_point()));
                    egui::gui_zoom::zoom_menu_buttons(ui);
                    if ui.button(if self.use_dock { "Disable Dock" } else { "Enable Dock" }).clicked() {
                        self.use_dock = !self.use_dock;
                        ui.close_menu();
                    }
                });

                self.fps_counter.update();
                ui.label(format!("fps: {:.2}", self.fps_counter.fps));
            });
        });
    }

    fn update_editor_window(&mut self, ctx: &egui::Context, project: &mut Project, world_data: &mut Option<WorldData>,
            input: &WinitInputHelper, editor_camera: &mut EditorCamera) {
        if self.scene_focus() {
            self.update_editor_camera(input, editor_camera);
            if let Some(engine) = project.engine() {
                self.click_entity(ctx, engine, input);
                if let Some(world_data) = world_data {
                    self.drag_entity(world_data, input, editor_camera);
                }
            }
        }
    }

    fn update_editor_camera(&mut self, input: &WinitInputHelper, editor_camera: &mut EditorCamera) {
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
            let screen_to_world = editor_camera.height / self.scene_window().pixel().y as f32;
            let mouse_diff = input.mouse_diff();
            editor_camera.position.x -= mouse_diff.0 * screen_to_world;
            editor_camera.position.y += mouse_diff.1 * screen_to_world;
        }
    }

    fn click_entity(&mut self, ctx: &egui::Context, engine: &mut Box<dyn Engine>, input: &WinitInputHelper) {
        if input.mouse_pressed(0) {
            if let Some((x, y)) = input.mouse() {
                let x = x - self.scene_window().position().x * ctx.pixels_per_point();
                let y = y - self.scene_window().position().y * ctx.pixels_per_point();
                let screen_position = UVec2::new(x as u32, y as u32);
                let mut eid = EntityId::dead();
                engine.command(Command::GetEntityAtScreen(WindowIndex::SCENE, screen_position, &mut eid));
                if eid != EntityId::dead() {
                    self.selected_entity = eid;
                }
                self.pressed_entity = eid;
            }
        }
    }

    fn drag_entity(&mut self, world_data: &mut WorldData, input: &WinitInputHelper, editor_camera: &EditorCamera) {
        if input.mouse_held(0) && self.selected_entity == self.pressed_entity {
            if let Some(entity_data) = world_data.entities.get_mut(&self.selected_entity) {
                if let Some(data) = entity_data.components.get_mut("Transform") {
                    if let Some(Value::Vec3(position)) = data.values.get_mut("position") {
                        let screen_to_world = editor_camera.height / self.scene_window().pixel().y as f32;
                        let mouse_diff = input.mouse_diff();
                        position.x += mouse_diff.0 * screen_to_world;
                        position.y -= mouse_diff.1 * screen_to_world;
                    }
                }
            }
        }
    }

    fn entity_component_windows(&mut self, ctx: &egui::Context, world_data: &mut WorldData, engine: &mut Box<dyn Engine>) {
        egui::Window::new("Entities").show(&ctx, |ui| {
            self.entities_view(ui, world_data, engine);
        });

        if let Some(entity_data) = world_data.entities.get_mut(&self.selected_entity) {
            egui::Window::new("Components").show(&ctx, |ui| {
                self.entity_view(ui, entity_data, engine);
            });
        }
    }

    fn entities_view(&mut self, ui: &mut egui::Ui, world_data: &WorldData, engine: &mut Box<dyn Engine>) {
        let hierarchy = world_data.uniques.get("Hierarchy").expect("Hierarchy unique is missing!");
        let root_entities = match hierarchy.get("roots") {
            Some(Value::VecEntity(v)) => v,
            _ => panic!("Hierarchy does not have roots!"),
        };

        if !root_entities.is_empty() {
            let (mut drag_entity, mut drop_parent, mut drop_before) = (EntityId::dead(), None, EntityId::dead());
            self.entity_level(root_entities, EntityId::dead(), ui, world_data, engine, &mut drag_entity, &mut drop_parent, &mut drop_before);
            if let Some(drop_parent) = drop_parent {
                if drag_entity != EntityId::dead() && ui.input(|input| input.pointer.any_released()) {
                    engine.command(Command::AttachEntity(drag_entity, drop_parent, drop_before));
                }
            }
        } else if !world_data.entities.is_empty() {
            panic!("entities_view: hierarchy.roots is empty but world_data.entities is not empty! world_data.entities={:?}", world_data.entities);
        }

        if ui.button("+").clicked() {
            engine.command(Command::CreateEntity);
        }
    }

    fn entity_level(&mut self, entities: &Vec<EntityId>, parent: EntityId, ui: &mut egui::Ui, world_data: &WorldData, engine: &mut Box<dyn Engine>,
            drag_entity: &mut EntityId, drop_parent: &mut Option<EntityId>, drop_before: &mut EntityId) {
        for (i, entity) in entities.iter().enumerate() {
            let entity = *entity;
            let entity_data = world_data.entities.get(&entity).expect(format!("entity_level: non-existent entity: {entity:?}").as_str());

            let mut entity_item = |ui: &mut egui::Ui| ui.horizontal(|ui| {
                let drag_id = egui::Id::new(entity);
                if ui.memory(|mem| mem.is_being_dragged(drag_id)) {
                    *drag_entity = entity;
                }

                let can_accept_what_is_being_dragged = entity != *drag_entity;
                let can_insert_before = true;
                let can_insert_after = i == entities.len() - 1;

                let drop_result = Self::drop_target(ui, can_accept_what_is_being_dragged, can_insert_before, can_insert_after, |ui| {
                    Self::drag_source(ui, drag_id, |ui| {
                        if ui.selectable_label(self.selected_entity == entity, Self::entity_label(&entity, entity_data)).clicked() {
                            self.selected_entity = entity;
                        }
                    });
                });

                if let Some(drop_result) = drop_result {
                    match drop_result {
                        DropResult::Before => {
                            *drop_parent = Some(parent);
                            *drop_before = entity;
                        }
                        DropResult::Into => *drop_parent = Some(entity),
                        DropResult::After => {
                            *drop_parent = Some(parent);
                            *drop_before = if i + 1 < entities.len() { entities[i + 1] } else { EntityId::dead() };
                        }
                    }
                }

                if ui.button("-").clicked() {
                    engine.command(Command::DestroyEntity(entity));
                }
            });

            if let Some(parent) = entity_data.components.get("Parent") {
                egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), egui::Id::new(entity), false)
                    .show_header(ui, |ui| entity_item(ui))
                    .body(|ui| {
                        let children = match parent.get("children") {
                            Some(Value::VecEntity(v)) => v,
                            _ => panic!("entity_level: no children value in Parent component: {parent:?}"),
                        };
                        self.entity_level(children, entity, ui, world_data, engine, drag_entity, drop_parent, drop_before)
                    });
            } else {
                ui.horizontal(|ui| {
                    ui.add_space(18.0); // align with header, TODO: get correct space value dynamically
                    entity_item(ui);
                });
            }
        }
    }

    fn drag_source<R>(ui: &mut egui::Ui, id: egui::Id, body: impl FnOnce(&mut egui::Ui) -> R) {
        let is_being_dragged = ui.memory(|mem| mem.is_being_dragged(id));

        if !is_being_dragged {
            let response = ui.scope(body).response;

            // caculate press time
            let press_time = ui.input(|input| {
                if let Some(press_origin) = input.pointer.press_origin() {
                    if response.rect.contains(press_origin) {
                        if let Some(press_start_time) = input.pointer.press_start_time() {
                            return input.time - press_start_time;
                        }
                    }
                }
                return 0.0;
            });

            // start drag after pressing some time
            if press_time > 0.3 {
                ui.memory_mut(|mem| mem.set_dragged_id(id));
            }
        } else {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);

            // paint the body to a new layer
            let layer_id = egui::LayerId::new(egui::Order::Tooltip, id);
            let response = ui.with_layer_id(layer_id, body).response;

            // now we move the visuals of the body to where the mouse is
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let delta = pointer_pos - response.rect.center();
                ui.ctx().translate_layer(layer_id, delta);
            }
        }
    }

    fn drop_target<R>(ui: &mut egui::Ui, can_accept_what_is_being_dragged: bool, can_insert_before: bool, can_insert_after: bool,
            body: impl FnOnce(&mut egui::Ui) -> R) -> Option<DropResult> {
        let is_anything_being_dragged = ui.memory(|mem| mem.is_anything_being_dragged());

        let margin = egui::Vec2::splat(1.0);
        let outer_rect_bounds = ui.available_rect_before_wrap();
        let inner_rect = outer_rect_bounds.shrink2(margin);
        let where_to_put_background = ui.painter().add(egui::Shape::Noop);
        let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
        body(&mut content_ui);
        let outer_rect = egui::Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);
        let (rect, response) = ui.allocate_at_least(outer_rect.size(), egui::Sense::hover());

        if is_anything_being_dragged && can_accept_what_is_being_dragged && response.hovered() {
            if let Some(hover_pos) = ui.input(|input| input.pointer.hover_pos()) {
                let style = ui.visuals().widgets.active;
                if can_insert_before && hover_pos.y - rect.top() < rect.height() / 4.0 {
                    ui.painter().set(where_to_put_background, egui::epaint::Shape::line_segment([rect.left_top(), rect.right_top()], style.bg_stroke));
                    return Some(DropResult::Before);
                } else if can_insert_after && hover_pos.y - rect.top() > rect.height() * 3.0 / 4.0 {
                    ui.painter().set(where_to_put_background, egui::epaint::Shape::line_segment([rect.left_bottom(), rect.right_bottom()], style.bg_stroke));
                    return Some(DropResult::After);
                } else {
                    ui.painter().set(where_to_put_background, egui::epaint::Shape::rect_stroke(rect, style.rounding, style.bg_stroke));
                    return Some(DropResult::Into);
                }
            }
        }
        None
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

    fn entity_view(&mut self, ui: &mut egui::Ui, entity_data: &mut EntityData, engine: &mut Box<dyn Engine>) {
        for (component_name, component_data) in &mut entity_data.components {
            ui.horizontal(|ui| {
                ui.label(component_name);
                if component_name != "Child" && component_name != "Parent" { // TODO: use a more generic way to prevent some components from being destroyed by user
                    if ui.button("-").clicked() {
                        engine.command(Command::DestroyComponent(self.selected_entity, component_name));
                    }
                }
            });
            Self::data_view(ui, component_name, component_data);
            if component_name == "EntityInfo" {
                ui.horizontal(|ui| {
                    ui.label("id");
                    Self::color_label(ui, egui::Color32::BLACK, format!("{:?}", self.selected_entity));
                });
            }
            ui.separator();
        }

        let mut components = Vec::new();
        engine.command(Command::GetComponents(&mut components));
        ui.menu_button("+", |ui| {
            for component in components.into_iter().filter(|c| *c != "Child" && *c != "Parent") { // TODO: use a more generic way to prevent some components from being created by user
                if ui.button(component).clicked() {
                    engine.command(Command::CreateComponent(self.selected_entity, component));
                    ui.close_menu();
                }
            }
        });
    }

    fn data_view(ui: &mut egui::Ui, data_name: &String, data: &mut Data) {
        for (name, value) in &mut data.values {
            ui.horizontal(|ui| {
                ui.label(name);
                if let Some(Limit::ReadOnly) = data.limits.get(name) {
                    let color = egui::Color32::BLACK;
                    match value {
                        Value::Bool(b) => Self::color_label(ui, color, if *b { "☑" } else { "☐" }),
                        Value::Int32(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::UInt32(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::Float32(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::String(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::Entity(v) => Self::color_label(ui, color, format!("{v:?}")),
                        Value::Vec2(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::Vec3(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::Vec4(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::IVec2(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::IVec3(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::IVec4(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::UVec2(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::UVec3(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::UVec4(v) => Self::color_label(ui, color, format!("{v}")),
                        Value::VecEntity(v) => {
                            ui.vertical(|ui| {
                                for e in v {
                                    Self::color_label(ui, color, format!("{e:?}"));
                                }
                            });
                        }
                    }
                } else {
                    match value {
                        Value::Bool(b) => {
                            ui.checkbox(b, "");
                        }
                        Value::Int32(v) => {
                            if let Some(Limit::Int32Enum(int_enum)) = data.limits.get(name) {
                                if int_enum.len() > 0 {
                                    let mut i = int_enum.iter().enumerate().find_map(|(i, (int, _))| {
                                        if v == int { Some(i) } else { None }
                                    }).unwrap_or(0);
                                    // Use component_name/unique_name + value_name as id to make sure that every id is unique
                                    egui::ComboBox::from_id_source(format!("{} {}", data_name, name))
                                        .show_index(ui, &mut i, int_enum.len(), |i| &int_enum[i].1);
                                    *v = int_enum[i].0;
                                } else {
                                    Self::color_label(ui, egui::Color32::RED, "zero length int_enum!");
                                }
                            } else {
                                let range = match data.limits.get(name) {
                                    Some(Limit::Int32Range(range)) => Some(range.clone()),
                                    _ => None,
                                };
                                Self::drag_value(ui, v, range);
                            }
                        }
                        Value::UInt32(v) => {
                            let range = match data.limits.get(name) {
                                Some(Limit::UInt32Range(range)) => Some(range.clone()),
                                _ => None,
                            };
                            Self::drag_value(ui, v, range);
                        }
                        Value::String(v) => {
                            if let Some(Limit::StringMultiline) = data.limits.get(name) {
                                ui.text_edit_multiline(v);
                            } else {
                                ui.text_edit_singleline(v);
                            }
                        }
                        Value::Entity(v) => {
                            ui.label(format!("{v:?}")); // TODO: change entity in editor
                        }
                        Value::Float32(v) => {
                            if let Some(Limit::Float32Rotation) = data.limits.get(name) {
                                ui.drag_angle(v);
                            } else {
                                Self::drag_float32(ui, v, match data.limits.get(name) {
                                    Some(Limit::Float32Range(range)) => Some(range.clone()),
                                    _ => None,
                                });
                            }
                        }
                        Value::Vec2(v) => {
                            ui.horizontal(|ui| {
                                if let Some(Limit::Float32Rotation) = data.limits.get(name) {
                                    ui.drag_angle(&mut v.x);
                                    ui.drag_angle(&mut v.y);
                                } else {
                                    let range = match data.limits.get(name) {
                                        Some(Limit::Float32Range(range)) => vec![Some(range.clone()); 2],
                                        Some(Limit::VecRange(range)) => range.clone(),
                                        _ => Vec::new(),
                                    };
                                    Self::drag_float32(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                                    Self::drag_float32(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                                }
                            });
                        }
                        Value::Vec3(v) => {
                            ui.horizontal(|ui| {
                                if let Some(Limit::Vec3Color) = data.limits.get(name) {
                                    let mut color = v.to_array();
                                    ui.color_edit_button_rgb(&mut color);
                                    *v = Vec3::from_array(color);
                                } else if let Some(Limit::Float32Rotation) = data.limits.get(name) {
                                    ui.drag_angle(&mut v.x);
                                    ui.drag_angle(&mut v.y);
                                    ui.drag_angle(&mut v.z);
                                } else {
                                    let range = match data.limits.get(name) {
                                        Some(Limit::Float32Range(range)) => vec![Some(range.clone()); 3],
                                        Some(Limit::VecRange(range)) => range.clone(),
                                        _ => Vec::new(),
                                    };
                                    Self::drag_float32(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                                    Self::drag_float32(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                                    Self::drag_float32(ui, &mut v.z, range.get(2).and_then(|r| r.clone()));
                                }
                            });
                        }
                        Value::Vec4(v) => {
                            ui.horizontal(|ui| {
                                if let Some(Limit::Vec4Color) = data.limits.get(name) {
                                    let mut color = v.to_array();
                                    ui.color_edit_button_rgba_unmultiplied(&mut color);
                                    *v = Vec4::from_array(color);
                                } else if let Some(Limit::Float32Rotation) = data.limits.get(name) {
                                    ui.drag_angle(&mut v.x);
                                    ui.drag_angle(&mut v.y);
                                    ui.drag_angle(&mut v.z);
                                    ui.drag_angle(&mut v.w);
                                } else {
                                    let range = match data.limits.get(name) {
                                        Some(Limit::Float32Range(range)) => vec![Some(range.clone()); 4],
                                        Some(Limit::VecRange(range)) => range.clone(),
                                        _ => Vec::new(),
                                    };
                                    Self::drag_float32(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                                    Self::drag_float32(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                                    Self::drag_float32(ui, &mut v.z, range.get(2).and_then(|r| r.clone()));
                                    Self::drag_float32(ui, &mut v.w, range.get(3).and_then(|r| r.clone()));
                                }
                            });
                        }
                        Value::IVec2(v) => {
                            ui.horizontal(|ui| {
                                let range = match data.limits.get(name) {
                                    Some(Limit::Int32Range(range)) => vec![Some(range.clone()); 2],
                                    Some(Limit::IVecRange(range)) => range.clone(),
                                    _ => Vec::new(),
                                };
                                Self::drag_value(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                                Self::drag_value(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                            });
                        }
                        Value::IVec3(v) => {
                            ui.horizontal(|ui| {
                                let range = match data.limits.get(name) {
                                    Some(Limit::Int32Range(range)) => vec![Some(range.clone()); 3],
                                    Some(Limit::IVecRange(range)) => range.clone(),
                                    _ => Vec::new(),
                                };
                                Self::drag_value(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                                Self::drag_value(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                                Self::drag_value(ui, &mut v.z, range.get(2).and_then(|r| r.clone()));
                            });
                        }
                        Value::IVec4(v) => {
                            ui.horizontal(|ui| {
                                let range = match data.limits.get(name) {
                                    Some(Limit::Int32Range(range)) => vec![Some(range.clone()); 4],
                                    Some(Limit::IVecRange(range)) => range.clone(),
                                    _ => Vec::new(),
                                };
                                Self::drag_value(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                                Self::drag_value(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                                Self::drag_value(ui, &mut v.z, range.get(2).and_then(|r| r.clone()));
                                Self::drag_value(ui, &mut v.w, range.get(3).and_then(|r| r.clone()));
                            });
                        }
                        Value::UVec2(v) => {
                            ui.horizontal(|ui| {
                                let range = match data.limits.get(name) {
                                    Some(Limit::UInt32Range(range)) => vec![Some(range.clone()); 2],
                                    Some(Limit::UVecRange(range)) => range.clone(),
                                    _ => Vec::new(),
                                };
                                Self::drag_value(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                                Self::drag_value(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                            });
                        }
                        Value::UVec3(v) => {
                            ui.horizontal(|ui| {
                                let range = match data.limits.get(name) {
                                    Some(Limit::UInt32Range(range)) => vec![Some(range.clone()); 3],
                                    Some(Limit::UVecRange(range)) => range.clone(),
                                    _ => Vec::new(),
                                };
                                Self::drag_value(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                                Self::drag_value(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                                Self::drag_value(ui, &mut v.z, range.get(2).and_then(|r| r.clone()));
                            });
                        }
                        Value::UVec4(v) => {
                            ui.horizontal(|ui| {
                                let range = match data.limits.get(name) {
                                    Some(Limit::UInt32Range(range)) => vec![Some(range.clone()); 4],
                                    Some(Limit::UVecRange(range)) => range.clone(),
                                    _ => Vec::new(),
                                };
                                Self::drag_value(ui, &mut v.x, range.get(0).and_then(|r| r.clone()));
                                Self::drag_value(ui, &mut v.y, range.get(1).and_then(|r| r.clone()));
                                Self::drag_value(ui, &mut v.z, range.get(2).and_then(|r| r.clone()));
                                Self::drag_value(ui, &mut v.w, range.get(3).and_then(|r| r.clone()));
                            });
                        }
                        Value::VecEntity(v) => {
                            ui.vertical(|ui| {
                                for e in v {
                                    Self::color_label(ui, egui::Color32::BLACK, format!("{e:?}")); // TODO: add/remove/change entity in editor
                                }
                            });
                        }
                    }
                }
            });
        }
    }

    fn color_label(ui: &mut egui::Ui, color: egui::Color32, text: impl Into<egui::WidgetText>) {
        egui::Frame::none()
            .inner_margin(egui::style::Margin::symmetric(3.0, 1.0))
            .rounding(egui::Rounding::same(3.0))
            .fill(color)
            .show(ui, |ui| ui.label(text));
    }

    fn drag_float32(ui: &mut egui::Ui, v: &mut f32, range: Option<RangeInclusive<f32>>) {
        let mut drag_value = egui::DragValue::new(v)
            .max_decimals(100)
            .speed(0.01);
        if let Some(range) = range {
            drag_value = drag_value.clamp_range(range);
        }
        ui.add(drag_value);
    }

    fn drag_value<V: egui::emath::Numeric>(ui: &mut egui::Ui, v: &mut V, range: Option<RangeInclusive<V>>) {
        let mut drag_value = egui::DragValue::new(v);
        if let Some(range) = range {
            drag_value = drag_value.clamp_range(range);
        }
        ui.add(drag_value);
    }

    fn unique_windows(&mut self, ctx: &egui::Context, world_data: &mut WorldData) {
        egui::Window::new("Uniques").show(&ctx, |ui| {
            self.uniques_view(ui, world_data);
        });

        if let Some(unique_data) = world_data.uniques.get_mut(&self.selected_unique) {
            egui::Window::new(&self.selected_unique).show(&ctx, |ui| {
                Self::data_view(ui, &self.selected_unique, unique_data);
            });
        }
    }

    fn uniques_view(&mut self, ui: &mut egui::Ui, world_data: &WorldData) {
        egui::Grid::new("Uniques").show(ui, |ui| {
            for unique_name in world_data.uniques.keys() {
                if ui.selectable_label(self.selected_unique == *unique_name, unique_name).clicked() {
                    self.selected_unique = unique_name.clone();
                }
                ui.end_row();
            }
        });
    }

    pub fn suspend(&mut self) {
        self.scene_window.close(None);
        self.game_window.close(None);
    }

    pub fn scene_window(&self) -> &ImageWindow {
        &self.scene_window
    }

    pub fn scene_focus(&self) -> bool {
        if self.use_dock {
            self.focused_tab.as_ref().is_some_and(|tab| tab == "Scene")
        } else {
            self.scene_window.layer.is_some_and(|this| !self.game_window.layer.is_some_and(|other| other > this))
        }
    }

    pub fn game_window(&self) -> &ImageWindow {
        &self.game_window
    }

    #[allow(unused)]
    pub fn game_focus(&self) -> bool {
        if self.use_dock {
            self.focused_tab.as_ref().is_some_and(|tab| tab == "Game")
        } else {
            self.game_window.layer.is_some_and(|this| !self.scene_window.layer.is_some_and(|other| other > this))
        }
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

/// A egui window which displays a image
pub struct ImageWindow {
    title: String,
    image_index: usize,
    images: Option<Vec<Arc<ImageView>>>,
    texture_ids: Option<Vec<egui::TextureId>>,
    pixel: UVec2,
    size: Vec2,
    position: Vec2,
    layer: Option<usize>, // Warning: the value of layer is undefined if !project.is_compiled()
}

impl ImageWindow {
    fn new(title: impl Into<String>) -> Self {
        ImageWindow { title: title.into(), image_index: 0, images: None, texture_ids: None, pixel: UVec2::ZERO, size: Vec2::ZERO, position: Vec2::ZERO, layer: None }
    }

    fn show(&mut self, ctx: &egui::Context, gui: &mut Gui, context: &VulkanoContext, renderer: &VulkanoWindowRenderer) {
        egui::Window::new(&self.title)
            .movable(ctx.input(|input| input.pointer.hover_pos())
                .is_some_and(|hover_pos| hover_pos.y < self.position.y ))
            .show(&ctx, |ui| self.ui(ui, gui, context, renderer));
    }

    fn ui(&mut self, ui: &mut egui::Ui, gui: &mut Gui, context: &VulkanoContext, renderer: &VulkanoWindowRenderer) {
        self.image_index = renderer.image_index() as usize;
        let available_size = ui.available_size();
        (self.size.x, self.size.y) = (available_size.x, available_size.y);
        let pixel = (self.size * ui.ctx().pixels_per_point()).as_uvec2();
        if self.images.is_none() || self.pixel.x != pixel.x || self.pixel.y != pixel.y {
            self.pixel = pixel;
            self.close(Some(gui));
            self.images = Some((0..renderer.image_count()).map(|_| {
                let image = Image::new(
                    context.memory_allocator().clone(),
                    ImageCreateInfo {
                        format: renderer.swapchain_format(),
                        extent: [self.pixel.x, self.pixel.y, 1],
                        usage: ImageUsage::SAMPLED | ImageUsage::COLOR_ATTACHMENT,
                        ..Default::default()
                    },
                    AllocationCreateInfo::default(),
                ).unwrap();
                ImageView::new_default(image).unwrap()
            }).collect());
            self.texture_ids = Some(self.images.as_ref().unwrap().iter().map(|image|
                gui.register_user_image_view(image.clone(), Default::default())).collect());
            log::debug!("ImageWindow({}): image created, pixel={}, size={}", self.title, self.pixel, self.size);
        }
        let texture_id = self.texture_ids.as_ref().unwrap()[self.image_index];
        let r = ui.image(egui::ImageSource::Texture(egui::load::SizedTexture::new(texture_id, available_size)));
        (self.position.x, self.position.y) = (r.rect.left(), r.rect.top());
        self.layer = ui.ctx().memory(|mem| {
            match mem.focus() {
                Some(_) => None, // We should not have focus if any widget has keyboard focus
                None => mem.layer_ids().position(|layer_id| layer_id == r.layer_id),
            }
        });
    }

    fn close(&mut self, gui: Option<&mut Gui>) {
        self.images = None;
        if let (Some(gui), Some(texture_ids)) = (gui, &self.texture_ids) {
            for texture_id in texture_ids {
                gui.unregister_user_image(*texture_id);
            }
        }
        self.texture_ids = None;
    }

    /// Get window image of current frame, return None if images are not created yet.
    pub fn image(&self) -> Option<&Arc<ImageView>> {
        self.images.as_ref().and_then(|images| images.get(self.image_index))
    }

    /// Get the exact pixel of window images
    pub fn pixel(&self) -> UVec2 {
        self.pixel
    }

    /// Get the window size which is scaled by window scale factor
    pub fn size(&self) -> Vec2 {
        self.size
    }

    /// Get the window position which is scaled by window scale factor
    pub fn position(&self) -> Vec2 {
        self.position
    }
}

pub fn create_dock_state() -> DockState<String> {
    let tabs = ["Scene", "Game"].map(str::to_string).into_iter().collect();
    let mut dock_state = DockState::new(tabs);
    let surface = dock_state.main_surface_mut();
    let [_old_node, entities_node] = surface.split_right(NodeIndex::root(), 0.7, vec!["Entities".to_string()]);
    let [_old_node, _entity_node] = surface.split_right(entities_node, 0.4, vec!["Entity".to_string()]);
    let [_old_node, uniques_node] = surface.split_below(entities_node, 0.6, vec!["Uniques".to_string()]);
    let [_old_node, _unique_node] = surface.split_right(uniques_node, 0.4, vec!["Unique".to_string()]);
    dock_state
}

struct MyTabViewer<'a> {
    add_contents: Box<dyn FnMut(&mut egui::Ui, &mut <MyTabViewer as TabViewer>::Tab) + 'a>,
}

impl TabViewer for MyTabViewer<'_> {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.as_str().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        (self.add_contents)(ui, tab);
    }

    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        false
    }
}

enum DropResult {
    Before,
    Into,
    After,
}
