use super::{image_window::ImageWindow, EditorState};
use crate::{
    locale::{Language, Texts},
    project::Project,
    ui::data_window::DataWindow,
    utils::LocalData,
};
use egui_dock::DockState;
use egui_winit_vulkano::Gui;
use shipyard::EntityId;
use std::time::Instant;
use steel_common::{app::Command, data::WorldData};
use vulkano_util::context::VulkanoContext;

pub struct MenuBar {
    show_open_project_dialog: bool,
    show_asset_system_introduction_dialog: bool,
    switch_to_game_window_on_start: bool,
    fps_counter: FpsCounter,
}

impl MenuBar {
    pub fn new() -> Self {
        MenuBar {
            show_open_project_dialog: false,
            show_asset_system_introduction_dialog: false,
            switch_to_game_window_on_start: false,
            fps_counter: FpsCounter::new(),
        }
    }

    pub fn ui(
        &mut self,
        editor_state: &mut EditorState,
        scene_window: &mut ImageWindow,
        game_window: &mut ImageWindow,
        data_window: &mut DataWindow,
        ctx: &egui::Context,
        gui: &mut Gui,
        gui_game: &mut Option<Gui>,
        dock_state: &mut DockState<String>,
        context: &VulkanoContext,
        project: &mut Project,
        world_data: &mut Option<WorldData>,
        local_data: &mut LocalData,
        texts: &mut Texts,
    ) {
        self.open_project_dialog(
            editor_state,
            scene_window,
            game_window,
            ctx,
            gui,
            gui_game,
            context,
            project,
            local_data,
            texts,
        );

        self.asset_system_introduction_dialog(ctx, texts);

        egui::TopBottomPanel::top("my_top_panel").show(&ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button(texts.get("Project"), |ui| {
                    if ui.button(texts.get("Open")).clicked() {
                        log::info!("Menu->Project->Open");
                        self.show_open_project_dialog = true;
                        ui.close_menu();
                    }
                    if project.is_open() {
                        if ui.button(texts.get("Close")).clicked() {
                            log::info!("Menu->Project->Close");
                            scene_window.close(Some(gui));
                            game_window.close(Some(gui));
                            project.close(gui_game);
                            ui.close_menu();
                        }
                        if ui.button(texts.get("Compile")).clicked() {
                            log::info!("Menu->Project->Compile");
                            scene_window.close(Some(gui));
                            game_window.close(Some(gui));
                            project.compile(gui_game, context);
                            ui.close_menu();
                        }
                    }
                    if project.is_compiled() && !project.is_running() {
                        ui.menu_button(texts.get("Export"), |ui| {
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

                ui.add_enabled_ui(project.is_compiled(), |ui| {
                    ui.menu_button(texts.get("Edit"), |ui| {
                        ui.add_enabled_ui(
                            data_window.selected_entity() != EntityId::dead()
                                && world_data.is_some(),
                            |ui| {
                                let world_data = world_data.as_ref().unwrap();
                                if ui.button(texts.get("Duplicate")).clicked() {
                                    log::info!("Menu->Edit->Duplicate");
                                    DataWindow::duplicate_entity(
                                        data_window.selected_entity(),
                                        &world_data.entities,
                                        project.app().unwrap(),
                                    );
                                    ui.close_menu();
                                }
                                if ui.button(texts.get("Delete")).clicked() {
                                    log::info!("Menu->Edit->Delete");
                                    data_window.delete_entity(
                                        data_window.selected_entity(),
                                        project.app().unwrap(),
                                    );
                                    ui.close_menu();
                                }
                                if world_data
                                    .entities
                                    .get(&data_window.selected_entity())
                                    .and_then(|entity_data| entity_data.prefab_asset())
                                    .is_some()
                                {
                                    if ui.button(texts.get("Save Prefab")).clicked() {
                                        log::info!("Menu->Edit->Save Prefab");
                                        let asset_dir = project.asset_dir().unwrap();
                                        DataWindow::save_prefab(
                                            data_window.selected_entity(),
                                            &world_data.entities,
                                            project,
                                            asset_dir,
                                        );
                                        ui.close_menu();
                                    }
                                }
                                if ui.button(texts.get("Save As Prefab")).clicked() {
                                    log::info!("Menu->Edit->Save As Prefab");
                                    let asset_dir = project.asset_dir().unwrap();
                                    DataWindow::save_as_prefab(
                                        data_window.selected_entity(),
                                        &world_data.entities,
                                        project.app().unwrap(),
                                        asset_dir,
                                    );
                                    ui.close_menu();
                                }
                            },
                        );
                        ui.menu_button(texts.get("Create"), |ui| {
                            if ui.button(texts.get("New Entity")).clicked() {
                                log::info!("Menu->Edit->Create->New Entity");
                                DataWindow::create_new_entity(project.app().unwrap());
                                ui.close_menu();
                            }
                            if ui.button(texts.get("From Prefab")).clicked() {
                                log::info!("Menu->Edit->Create->From Prefab");
                                let asset_dir = project.asset_dir().unwrap();
                                DataWindow::create_entities_from_prefab(
                                    project.app().unwrap(),
                                    asset_dir,
                                );
                                ui.close_menu();
                            }
                        });
                    });

                    ui.add_enabled_ui(!project.is_running(), |ui| {
                        ui.menu_button(texts.get("Scene"), |ui| {
                            if let Some(scene_path) = project.scene_relative_path() {
                                if ui
                                    .button(format!(
                                        "{} ({})",
                                        texts.get("Save"),
                                        scene_path.display()
                                    ))
                                    .clicked()
                                {
                                    log::info!("Menu->Scene->Save");
                                    project.save_scene(scene_path);
                                    ui.close_menu();
                                }
                            }
                            let starting_dir =
                                if let Some(scene_path) = project.scene_absolute_path() {
                                    scene_path.parent().map(|p| p.to_path_buf()).expect(
                                        "scene file should be at least in the asset directory",
                                    )
                                } else {
                                    project.asset_dir().expect(
                                    "project.asset_dir() must be some when project.is_compiled()",
                                )
                                };
                            if ui.button(texts.get("Save As")).clicked() {
                                log::info!("Menu->Scene->Save As");
                                let file = rfd::FileDialog::new()
                                    .set_directory(&starting_dir)
                                    .save_file();
                                log::info!("Close FileDialog, file={file:?}");
                                if let Some(mut file) = file {
                                    file.set_extension("scene");
                                    let file = project.convert_to_scene_relative_path(&file);
                                    log::info!(
                                        "After convert_to_scene_relative_path, file={file:?}"
                                    );
                                    if let Some(file) = file {
                                        project.save_scene(file);
                                    }
                                }
                                ui.close_menu();
                            }
                            if ui.button(texts.get("Load")).clicked() {
                                log::info!("Menu->Scene->Load");
                                let file = rfd::FileDialog::new()
                                    .set_directory(starting_dir)
                                    .pick_file();
                                log::info!("Close FileDialog, file={file:?}");
                                if let Some(file) = file {
                                    let file = project.convert_to_scene_relative_path(&file);
                                    log::info!(
                                        "After convert_to_scene_relative_path, file={file:?}"
                                    );
                                    if let Some(file) = file {
                                        project.load_scene(file);
                                        // We set world_data to None to prevent app from loading outdated world_data later this frame.
                                        // But this will cause a splash screen problem due to the disappearance of the windows showing world_data for one frame.
                                        // TODO: find a way to avoid this splash screen problem.
                                        *world_data = None;
                                    }
                                }
                                ui.close_menu();
                            }
                            if ui.button(texts.get("New")).clicked() {
                                log::info!("Menu->Scene->New");
                                project.new_scene();
                                ui.close_menu();
                            }
                        });
                    });

                    // start running function
                    let start_running_fn =
                        |project: &mut Project,
                         switch_to_game_window_on_start: bool,
                         dock_state: &mut DockState<String>| {
                            project.save_to_memory(None);
                            project.app().unwrap().command(Command::ResetTime);
                            if switch_to_game_window_on_start {
                                if let Some(tab) = dock_state.find_tab(&"Game".to_string()) {
                                    dock_state.set_active_tab(tab);
                                }
                            }
                            project.set_running(true);
                        };

                    if !project.is_running() && ui.input(|i| i.key_pressed(egui::Key::F5)) {
                        log::info!("Start running by pressing F5");
                        start_running_fn(project, self.switch_to_game_window_on_start, dock_state);
                    }

                    ui.menu_button(texts.get("Run"), |ui| {
                        if project.is_running() {
                            if ui.button(texts.get("Stop")).clicked() {
                                log::info!("Menu->Run->Stop");
                                project.load_from_memory();
                                project.set_running(false);
                                ui.close_menu();
                            }
                        } else {
                            if ui.button(format!("{} (F5)", texts.get("Start"))).clicked() {
                                log::info!("Menu->Run->Start");
                                start_running_fn(
                                    project,
                                    self.switch_to_game_window_on_start,
                                    dock_state,
                                );
                                ui.close_menu();
                            }
                        }

                        ui.checkbox(
                            &mut self.switch_to_game_window_on_start,
                            texts.get("Switch to Game Window on Start"),
                        );
                    });
                });

                ui.add_enabled_ui(project.is_open(), |ui| {
                    ui.menu_button(texts.get("Asset"), |ui| {
                        if ui.button(texts.get("Open")).clicked() {
                            log::info!("Menu->Asset->Open");
                            if let Err(e) = open::that(project.asset_dir().unwrap()) {
                                log::error!("Failed to open asset folder: {e:?}");
                            }
                            ui.close_menu();
                        }
                        if ui.button(texts.get("Introduction")).clicked() {
                            log::info!("Menu->Asset->Introduction");
                            self.show_asset_system_introduction_dialog = true;
                            ui.close_menu();
                        }
                    });
                });

                ui.menu_button(texts.get("Ui"), |ui| {
                    ui.label(format!(
                        "{}{}",
                        texts.get("Current Scale: "),
                        ctx.pixels_per_point()
                    ));
                    egui::gui_zoom::zoom_menu_buttons(ui);
                    ui.menu_button(texts.get("Language"), |ui| {
                        if ui.button(texts.get("en-US")).clicked() {
                            texts.language = Language::Eng;
                            local_data.language = Some(texts.language);
                            local_data.save();
                            ui.close_menu();
                        }
                        if ui.button(texts.get("zh-CN")).clicked() {
                            texts.language = Language::Chs;
                            local_data.language = Some(texts.language);
                            local_data.save();
                            ui.close_menu();
                        }
                        if ui.button(texts.get("Follow System")).clicked() {
                            texts.language = crate::locale::system_language();
                            local_data.language = None;
                            local_data.save();
                            ui.close_menu();
                        }
                    });
                });

                self.fps_counter.update();
                ui.label(format!("{}{:.2}", texts.get("fps: "), self.fps_counter.fps));
            });
        });
    }

    fn open_project_dialog(
        &mut self,
        editor_state: &mut EditorState,
        scene_window: &mut ImageWindow,
        game_window: &mut ImageWindow,
        ctx: &egui::Context,
        gui: &mut Gui,
        gui_game: &mut Option<Gui>,
        context: &VulkanoContext,
        project: &mut Project,
        local_data: &mut LocalData,
        texts: &Texts,
    ) {
        let mut show = self.show_open_project_dialog;
        egui::Window::new(texts.get("Open Project"))
            .open(&mut show)
            .show(&ctx, |ui| {
                ui.horizontal(|ui| {
                    let mut path_str = editor_state.project_path.display().to_string();
                    ui.text_edit_singleline(&mut path_str);
                    editor_state.project_path = path_str.into();

                    if ui.button(texts.get("Browse")).clicked() {
                        log::info!("Open FileDialog");
                        let folder = rfd::FileDialog::new()
                            .set_directory(&editor_state.project_path)
                            .pick_folder();
                        log::info!("Close FileDialog, folder={folder:?}");
                        if let Some(folder) = folder {
                            editor_state.project_path = folder;
                        }
                    }
                });
                if ui.button(texts.get("Open")).clicked() {
                    if editor_state.project_path.display().to_string().is_empty() {
                        log::info!("Open project failed, path is empty");
                    } else {
                        log::info!("Open project, path={}", editor_state.project_path.display());
                        scene_window.close(Some(gui));
                        game_window.close(Some(gui));
                        project.open(editor_state.project_path.clone(), local_data);
                        project.compile(gui_game, context);
                        self.show_open_project_dialog = false;
                    }
                }
            });
        self.show_open_project_dialog &= show;
    }

    fn asset_system_introduction_dialog(&mut self, ctx: &egui::Context, texts: &Texts) {
        egui::Window::new(texts.get("Asset System Introduction"))
            .open(&mut self.show_asset_system_introduction_dialog)
            .show(ctx, |ui| {
                ui.label(texts.get("Asset Introduction"));
            });
    }
}

struct FpsCounter {
    start: Instant,
    frame: u32,
    fps: f32,
}

impl FpsCounter {
    fn new() -> Self {
        FpsCounter {
            start: Instant::now(),
            frame: 0,
            fps: 0.0,
        }
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
