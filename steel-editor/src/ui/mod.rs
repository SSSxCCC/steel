mod data_window;
mod image_window;
mod menu_bar;

use crate::{locale::Texts, project::Project, utils::LocalData};
use data_window::DataWindow;
use egui_dock::{DockArea, DockState, NodeIndex, TabViewer};
use egui_winit_vulkano::Gui;
use glam::{UVec2, Vec3};
use image_window::ImageWindow;
use menu_bar::MenuBar;
use shipyard::EntityId;
use std::path::PathBuf;
use steel_common::{
    app::{App, Command, EditorCamera, WindowIndex},
    data::{Value, WorldData},
};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};
use winit::event::VirtualKeyCode;
use winit_input_helper::WinitInputHelper;

pub struct Editor {
    #[allow(unused)]
    demo_windows: egui_demo_lib::DemoWindows,
    editor_state: EditorState,
    dock_state: DockState<String>,
    menu_bar: MenuBar,
    scene_window: ImageWindow,
    game_window: ImageWindow,
    data_window: DataWindow,
    texts: Texts,
}

impl Editor {
    pub fn new(local_data: &LocalData) -> Self {
        Editor {
            demo_windows: egui_demo_lib::DemoWindows::default(),
            editor_state: EditorState::new(local_data),
            dock_state: Self::create_dock_state(),
            menu_bar: MenuBar::new(),
            scene_window: ImageWindow::new("Scene"),
            game_window: ImageWindow::new("Game"),
            data_window: DataWindow::new(),
            texts: Texts::new(local_data.language),
        }
    }

    fn create_dock_state() -> DockState<String> {
        let tabs = ["Scene", "Game"].map(str::to_string).into_iter().collect();
        let mut dock_state = DockState::new(tabs);
        let surface = dock_state.main_surface_mut();
        let [_old_node, entities_node] =
            surface.split_right(NodeIndex::root(), 0.7, vec!["Entities".to_string()]);
        let [_old_node, _entity_node] =
            surface.split_right(entities_node, 0.4, vec!["Entity".to_string()]);
        let [_old_node, uniques_node] =
            surface.split_below(entities_node, 0.6, vec!["Uniques".to_string()]);
        let [_old_node, _unique_node] =
            surface.split_right(uniques_node, 0.4, vec!["Unique".to_string()]);
        dock_state
    }

    pub fn ui(
        &mut self,
        gui: &mut Gui,
        gui_game: &mut Option<Gui>,
        context: &VulkanoContext,
        renderer: &VulkanoWindowRenderer,
        project: &mut Project,
        world_data: &mut Option<WorldData>,
        local_data: &mut LocalData,
        input: &WinitInputHelper,
        editor_camera: &mut EditorCamera,
    ) {
        gui.immediate_ui(|gui| {
            let ctx = gui.context();

            // display egui demo windows
            //self.demo_windows.ui(&ctx);

            self.menu_bar.ui(
                &mut self.editor_state,
                &mut self.scene_window,
                &mut self.game_window,
                &mut self.data_window,
                &ctx,
                gui,
                gui_game,
                &mut self.dock_state,
                context,
                project,
                world_data,
                local_data,
                &mut self.texts,
            );

            if project.is_compiled() {
                self.update_editor_window(&ctx, project, world_data, input, editor_camera);
                self.scene_window.set_layer(None);
                self.game_window.set_layer(None);
                if !self.editor_state.use_dock {
                    self.scene_window
                        .show(&ctx, gui, context, renderer, &self.texts);
                    self.game_window
                        .show(&ctx, gui, context, renderer, &self.texts);
                }
            } else if project.is_open() {
                Self::compile_error_dialog(&ctx, &self.texts);
            }

            let asset_dir = project.asset_dir();

            if !self.editor_state.use_dock {
                if let Some(world_data) = world_data {
                    if let Some(app) = project.app() {
                        self.data_window.entity_component_windows(
                            &ctx,
                            world_data,
                            app,
                            asset_dir.as_ref().expect("project.asset_dir() must be some when project.app() is some"),
                            &self.texts,
                        );
                        self.data_window
                            .unique_windows(&ctx, world_data, app, asset_dir.as_ref().expect("project.asset_dir() must be some when project.app() is some"), &self.texts);
                    }
                }
            }

            if self.editor_state.use_dock {
                DockArea::new(&mut self.dock_state)
                    .style(egui_dock::Style::from_egui(gui.egui_ctx.style().as_ref()))
                    .show(
                        &ctx,
                        &mut MyTabViewer {
                            texts: &self.texts,
                            add_contents: Box::new(|ui, tab| match tab.as_str() {
                                "Scene" => {
                                    if project.is_compiled() {
                                        self.scene_window.ui(ui, gui, context, renderer);
                                    }
                                }
                                "Game" => {
                                    if project.is_compiled() {
                                        self.game_window.ui(ui, gui, context, renderer);
                                    }
                                }
                                "Entities" => {
                                    if let Some(world_data) = world_data {
                                        if let Some(app) = project.app() {
                                            self.data_window.entities_view(
                                                ui,
                                                world_data,
                                                app,
                                                &self.texts,
                                            );
                                        }
                                    }
                                }
                                "Entity" => {
                                    if let Some(world_data) = world_data {
                                        if let Some(app) = project.app() {
                                            if let Some(entity_data) = world_data
                                                .entities
                                                .get_mut(&self.data_window.selected_entity())
                                            {
                                                self.data_window.entity_view(
                                                    ui,
                                                    entity_data,
                                                    app,
                                                    asset_dir.as_ref().expect("project.asset_dir() must be some when project.app() is some"),
                                                    &self.texts,
                                                );
                                            }
                                        }
                                    }
                                }
                                "Uniques" => {
                                    if let Some(world_data) = world_data {
                                        self.data_window.uniques_view(ui, world_data);
                                    }
                                }
                                "Unique" => {
                                    if let Some(world_data) = world_data {
                                        if let Some(app) = project.app() {
                                            if let Some(unique_data) = world_data
                                                .uniques
                                                .get_mut(self.data_window.selected_unique())
                                            {
                                                DataWindow::data_view(
                                                    ui,
                                                    self.data_window.selected_unique(),
                                                    unique_data,
                                                    app,
                                                    asset_dir.as_ref().expect("project.asset_dir() must be some when project.app() is some"),
                                                    &self.texts,
                                                );
                                            }
                                        }
                                    }
                                }
                                _ => (),
                            }),
                        },
                    ); // DockArea shows inside egui::CentralPanel

                self.editor_state.focused_tab = self
                    .dock_state
                    .find_active_focused()
                    .map(|(_, tab)| tab.clone());
            }
        });
    }

    fn compile_error_dialog(ctx: &egui::Context, texts: &Texts) {
        egui::Window::new(texts.get("Compile error!")).show(&ctx, |ui| {
            ui.label(texts.get("Compile error message"));
        });
    }

    fn update_editor_window(
        &mut self,
        ctx: &egui::Context,
        project: &mut Project,
        world_data: &mut Option<WorldData>,
        input: &WinitInputHelper,
        editor_camera: &mut EditorCamera,
    ) {
        if self.scene_focus() {
            self.update_editor_camera(input, editor_camera);
            if let Some(app) = project.app() {
                self.click_entity(ctx, app, input);
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
            let screen_to_world = editor_camera.height / self.scene_window.pixel().y as f32;
            let mouse_diff = input.mouse_diff();
            editor_camera.position.x -= mouse_diff.0 * screen_to_world;
            editor_camera.position.y += mouse_diff.1 * screen_to_world;
        }
    }

    fn click_entity(
        &mut self,
        ctx: &egui::Context,
        app: &mut Box<dyn App>,
        input: &WinitInputHelper,
    ) {
        if input.mouse_pressed(0) {
            if let Some((x, y)) = input.mouse() {
                let x = x - self.scene_window.position().x * ctx.pixels_per_point();
                let y = y - self.scene_window.position().y * ctx.pixels_per_point();
                let screen_position = UVec2::new(x as u32, y as u32);
                let mut eid = EntityId::dead();
                app.command(Command::GetEntityAtScreen(
                    WindowIndex::SCENE,
                    screen_position,
                    &mut eid,
                ));
                if eid != EntityId::dead() {
                    self.data_window.set_selected_entity(eid);
                }
                self.editor_state.pressed_entity = eid;
            }
        }
    }

    fn drag_entity(
        &mut self,
        world_data: &mut WorldData,
        input: &WinitInputHelper,
        editor_camera: &EditorCamera,
    ) {
        if input.mouse_held(0)
            && self.data_window.selected_entity() == self.editor_state.pressed_entity
        {
            if let Some(entity_data) = world_data
                .entities
                .get_mut(&self.data_window.selected_entity())
            {
                if let Some(data) = entity_data.components.get_mut("Transform") {
                    if let Some(Value::Vec3(position)) = data.values.get_mut("position") {
                        let screen_to_world =
                            editor_camera.height / self.scene_window.pixel().y as f32;
                        let mouse_diff = input.mouse_diff();
                        position.x += mouse_diff.0 * screen_to_world;
                        position.y -= mouse_diff.1 * screen_to_world;
                    }
                }
            }
        }
    }

    fn scene_focus(&self) -> bool {
        if self.editor_state.use_dock {
            self.editor_state
                .focused_tab
                .as_ref()
                .is_some_and(|tab| tab == "Scene")
        } else {
            self.scene_window
                .layer()
                .is_some_and(|this| !self.game_window.layer().is_some_and(|other| other > this))
        }
    }

    #[allow(unused)]
    fn game_focus(&self) -> bool {
        if self.editor_state.use_dock {
            self.editor_state
                .focused_tab
                .as_ref()
                .is_some_and(|tab| tab == "Game")
        } else {
            self.game_window
                .layer()
                .is_some_and(|this| !self.scene_window.layer().is_some_and(|other| other > this))
        }
    }

    pub fn suspend(&mut self) {
        self.scene_window.close(None);
        self.game_window.close(None);
    }

    pub fn scene_window(&self) -> &ImageWindow {
        &self.scene_window
    }

    pub fn game_window(&self) -> &ImageWindow {
        &self.game_window
    }
}

struct EditorState {
    use_dock: bool,
    focused_tab: Option<String>,
    project_path: PathBuf,
    pressed_entity: EntityId,
}

impl EditorState {
    fn new(local_data: &LocalData) -> Self {
        EditorState {
            use_dock: true,
            focused_tab: None,
            project_path: local_data.last_open_project_path.clone(),
            pressed_entity: EntityId::dead(),
        }
    }
}

struct MyTabViewer<'a> {
    texts: &'a Texts,
    add_contents: Box<dyn FnMut(&mut egui::Ui, &mut <MyTabViewer as TabViewer>::Tab) + 'a>,
}

impl TabViewer for MyTabViewer<'_> {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        self.texts.get(tab.as_str()).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        (self.add_contents)(ui, tab);
    }

    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        false
    }
}
