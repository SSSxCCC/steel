mod data_window;
mod image_window;
mod menu_bar;

use crate::{locale::Texts, project::Project, utils::LocalData};
use data_window::DataWindow;
use egui_dock::{DockArea, DockState, NodeIndex, TabViewer};
use egui_winit_vulkano::Gui;
use glam::{IVec2, Quat, Vec3};
use image_window::ImageWindow;
use menu_bar::MenuBar;
use shipyard::EntityId;
use std::path::PathBuf;
use steel_common::{
    app::{App, Command, WindowIndex},
    camera::{CameraSettings, OrthographicCameraSize, SceneCamera},
    data::{Value, WorldData},
};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};

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
        scene_camera: &mut SceneCamera,
        window_title: &mut Option<String>,
    ) {
        gui.immediate_ui(|gui| {
            let mut load_world_data_this_frame = true;
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
                scene_camera,
                window_title,
                &mut self.texts,
                &mut load_world_data_this_frame,
            );

            if project.is_compiled() {
                self.update_editor_window(&ctx, project, world_data, scene_camera);
            } else if project.is_open() {
                Self::compile_error_dialog(&ctx, &self.texts);
            }

            let asset_dir = project.asset_dir();
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
                                if project.is_compiled() {
                                    if let Some(world_data) = world_data {
                                        self.data_window.entities_view(
                                            ui,
                                            world_data,
                                            project,
                                            asset_dir.as_ref().expect("project.asset_dir() must be some when project.app() is some"),
                                            &self.texts,
                                            &mut load_world_data_this_frame,
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
                                            self.data_window.data_view(
                                                ui,
                                                &self.data_window.selected_unique().clone(), // TODO: remove clone
                                                unique_data,
                                                app,
                                                asset_dir.as_ref().expect("project.asset_dir() must be some when project.app() is some"),
                                                &self.texts,
                                                false,
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

            if !load_world_data_this_frame {
                // We set world_data to None to prevent app from loading outdated world_data later this frame.
                // Be remember to set load_world_data_this_frame to false after loading scene data by calling Command::Reload.
                *world_data = None;
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
        scene_camera: &mut SceneCamera,
    ) {
        if self.scene_focus() {
            self.update_scene_camera(ctx, scene_camera);
            if let Some(app) = project.app() {
                self.click_entity(ctx, app);
                if let Some(world_data) = world_data {
                    self.drag_entity(ctx, world_data, scene_camera);
                }
            }
        }
    }

    fn update_scene_camera(&self, ctx: &egui::Context, scene_camera: &mut SceneCamera) {
        if ctx.input(|i| i.key_pressed(egui::Key::Home)) {
            scene_camera.reset();
        }

        match &mut scene_camera.settings {
            CameraSettings::Orthographic {
                width,
                height,
                size,
                ..
            } => {
                if ctx.input(|i| i.key_down(egui::Key::A) || i.key_down(egui::Key::ArrowLeft)) {
                    scene_camera.position.x -= 1.0; // TODO: * move_speed * delta_time
                }
                if ctx.input(|i| i.key_down(egui::Key::D) || i.key_down(egui::Key::ArrowRight)) {
                    scene_camera.position.x += 1.0;
                }
                if ctx.input(|i| i.key_down(egui::Key::W) || i.key_down(egui::Key::ArrowUp)) {
                    scene_camera.position.y += 1.0;
                }
                if ctx.input(|i| i.key_down(egui::Key::S) || i.key_down(egui::Key::ArrowDown)) {
                    scene_camera.position.y -= 1.0;
                }

                let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
                if scroll_delta > 0.0 {
                    *width /= 1.1;
                    *height /= 1.1;
                } else if scroll_delta < 0.0 {
                    *width *= 1.1;
                    *height *= 1.1;
                }

                if ctx.input(|i| i.pointer.secondary_down()) {
                    let screen_to_world = match size {
                        OrthographicCameraSize::FixedWidth => {
                            *width / self.scene_window.pixel().x as f32
                        }
                        OrthographicCameraSize::FixedHeight => {
                            *height / self.scene_window.pixel().y as f32
                        }
                        OrthographicCameraSize::MinWidthHeight => {
                            if *width / *height
                                > self.scene_window.pixel().x as f32
                                    / self.scene_window.pixel().y as f32
                            {
                                *width / self.scene_window.pixel().x as f32
                            } else {
                                *height / self.scene_window.pixel().y as f32
                            }
                        }
                    };
                    let mouse_diff = ctx.input(|i| i.pointer.delta() * i.pixels_per_point());
                    scene_camera.position.x -= mouse_diff.x * screen_to_world;
                    scene_camera.position.y += mouse_diff.y * screen_to_world;
                }
            }
            CameraSettings::Perspective { .. } => {
                if ctx.input(|i| i.pointer.secondary_down()) {
                    let mut rotation = scene_camera.rotation.to_scaled_axis();
                    let mouse_diff = ctx.input(|i| i.pointer.delta() * i.pixels_per_point());
                    rotation.y += mouse_diff.x / 1000.0;
                    rotation.x -= mouse_diff.y / 1000.0;
                    rotation.x = rotation
                        .x
                        .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
                    scene_camera.rotation = Quat::from_scaled_axis(rotation);
                    ctx.set_cursor_icon(egui::CursorIcon::None);
                } else {
                    ctx.set_cursor_icon(egui::CursorIcon::Default);
                }

                let rotation = scene_camera.rotation.to_scaled_axis(); // x: pitch, y: yaw, z: roll
                let direction = Vec3::new(
                    rotation.y.sin() * rotation.x.cos(),
                    rotation.x.sin(),
                    -rotation.y.cos() * rotation.x.cos(),
                );
                let right = direction.cross(Vec3::Y).normalize();
                let up = right.cross(direction).normalize();
                if ctx.input(|i| i.key_down(egui::Key::A) || i.key_down(egui::Key::ArrowLeft)) {
                    scene_camera.position -= right; // TODO: * move_speed * delta_time
                }
                if ctx.input(|i| i.key_down(egui::Key::D) || i.key_down(egui::Key::ArrowRight)) {
                    scene_camera.position += right;
                }
                if ctx.input(|i| i.key_down(egui::Key::W) || i.key_down(egui::Key::ArrowUp)) {
                    scene_camera.position += direction;
                }
                if ctx.input(|i| i.key_down(egui::Key::S) || i.key_down(egui::Key::ArrowDown)) {
                    scene_camera.position -= direction;
                }
                if ctx.input(|i| i.key_down(egui::Key::Space)) {
                    scene_camera.position += up;
                }
                if ctx.input(|i| i.key_down(egui::Key::C)) {
                    scene_camera.position -= up;
                }
            }
        }
    }

    fn click_entity(&mut self, ctx: &egui::Context, app: &Box<dyn App>) {
        if ctx.input(|i| i.pointer.primary_pressed()) {
            if let Some(press_origin) = ctx.input(|i| i.pointer.press_origin()) {
                let x = (press_origin.x - self.scene_window.position().x) * ctx.pixels_per_point();
                let y = (press_origin.y - self.scene_window.position().y) * ctx.pixels_per_point();
                let screen_position = IVec2::new(x as i32, y as i32);
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
        ctx: &egui::Context,
        world_data: &mut WorldData,
        scene_camera: &SceneCamera,
    ) {
        if let CameraSettings::Orthographic {
            width,
            height,
            size,
            ..
        } = scene_camera.settings
        {
            if ctx.input(|i| i.pointer.primary_down())
                && self.data_window.selected_entity() == self.editor_state.pressed_entity
            {
                if let Some(entity_data) = world_data
                    .entities
                    .get_mut(&self.data_window.selected_entity())
                {
                    if let Some(data) = entity_data.components.get_mut("Transform") {
                        if let Some(Value::Vec3(position)) = data.values.get_mut("position") {
                            let screen_to_world = match size {
                                OrthographicCameraSize::FixedWidth => {
                                    width / self.scene_window.pixel().x as f32
                                }
                                OrthographicCameraSize::FixedHeight => {
                                    height / self.scene_window.pixel().y as f32
                                }
                                OrthographicCameraSize::MinWidthHeight => {
                                    if width / height
                                        > self.scene_window.pixel().x as f32
                                            / self.scene_window.pixel().y as f32
                                    {
                                        width / self.scene_window.pixel().x as f32
                                    } else {
                                        height / self.scene_window.pixel().y as f32
                                    }
                                }
                            };
                            let mouse_diff =
                                ctx.input(|i| i.pointer.delta() * i.pixels_per_point());
                            position.x += mouse_diff.x * screen_to_world;
                            position.y -= mouse_diff.y * screen_to_world;
                        }
                    }
                }
            }
        }
    }

    fn scene_focus(&self) -> bool {
        self.editor_state
            .focused_tab
            .as_ref()
            .is_some_and(|tab| tab == "Scene")
    }

    #[allow(unused)]
    fn game_focus(&self) -> bool {
        self.editor_state
            .focused_tab
            .as_ref()
            .is_some_and(|tab| tab == "Game")
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
    focused_tab: Option<String>,
    project_path: PathBuf,
    pressed_entity: EntityId,
}

impl EditorState {
    fn new(local_data: &LocalData) -> Self {
        EditorState {
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
