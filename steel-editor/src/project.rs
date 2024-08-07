use crate::utils::LocalData;
use egui_winit_vulkano::Gui;
use libloading::{Library, Symbol};
use log::{LevelFilter, Log, SetLoggerError};
use regex::Regex;
use shipyard::EntityId;
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};
use steel_common::{
    app::{App, Command, InitInfo},
    data::{Data, EntityData, Limit, Value, WorldData},
    platform::Platform,
};
use vulkano_util::context::VulkanoContext;

struct ProjectCompiledState {
    app: Box<dyn App>,
    #[allow(unused)]
    library: Library, // Library must be destroyed after App

    data: WorldData,
    running: bool,
    /// The path of current opened scene file, which is relative to the asset directory of opened project
    scene: Option<PathBuf>,
}

struct ProjectState {
    path: PathBuf,
    compiled: Option<ProjectCompiledState>,
}

pub struct Project {
    state: Option<ProjectState>,
}

impl Project {
    pub fn new() -> Self {
        Project { state: None }
    }

    pub fn open(&mut self, path: PathBuf, local_data: &mut LocalData) {
        match Self::_open(&path) {
            Err(error) => log::error!(
                "Failed to open project, path={}, error={error}",
                path.display()
            ),
            Ok(_) => {
                local_data.last_open_project_path = path.clone();
                local_data.save();
                self.state = Some(ProjectState {
                    path,
                    compiled: None,
                });
            }
        }
    }

    fn _open(path: &PathBuf) -> Result<(), Box<dyn Error>> {
        if !path.is_dir() {
            fs::create_dir_all(&path)?;
            log::info!("Created directory: {}", path.display());
        }

        let gitignore_file = path.join(".gitignore");
        if !gitignore_file.exists() {
            fs::write(&gitignore_file, GITIGNORE)?;
            log::info!("Created: {}", gitignore_file.display());

            if let Err(error) = Self::_init_git(path) {
                log::warn!("Failed to init git, error={error}");
            }
        }

        let cargo_toml_file = path.join("Cargo.toml");
        if !cargo_toml_file.exists() {
            if let Ok(mut steel_engine_dir) = fs::canonicalize("steel-engine") {
                crate::utils::delte_windows_path_prefix(&mut steel_engine_dir);
                let steel_engine_dir = steel_engine_dir
                    .to_str()
                    .ok_or(ProjectError::new(format!(
                        "{} to_str() returns None",
                        steel_engine_dir.display()
                    )))?
                    .replace("\\", "/");
                fs::write(
                    &cargo_toml_file,
                    CARGO_TOML.replacen("../../steel-engine", steel_engine_dir.as_str(), 1),
                )?;
            } else {
                // "steel-engine" folder not found, maybe steel-editor is running in exported executable, just write CARGO_TOML content.
                fs::write(&cargo_toml_file, CARGO_TOML)?;
            }
            log::info!("Created: {}", cargo_toml_file.display());
        }

        let src_dir = path.join("src");
        if !src_dir.is_dir() {
            fs::create_dir(&src_dir)?;
            log::info!("Created directory: {}", src_dir.display());
        }

        let lib_rs_file = src_dir.join("lib.rs");
        if !lib_rs_file.is_file() {
            fs::write(&lib_rs_file, LIB_RS)?;
            log::info!("Created: {}", lib_rs_file.display());
        }

        let asset_dir = path.join("asset");
        if !asset_dir.is_dir() {
            fs::create_dir(&asset_dir)?;
            log::info!("Created directory: {}", asset_dir.display());
        }

        Ok(())
    }

    fn _init_git(project_path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
        std::process::Command::new("git")
            .arg("init")
            .current_dir(project_path)
            .spawn()?
            .wait()?;
        Ok(())
    }

    pub fn is_open(&self) -> bool {
        self.state.is_some()
    }

    pub fn close(&mut self, gui_game: &mut Option<Gui>) {
        *gui_game = None; // destroy Gui struct before release dynlib to fix egui crash problem
        self.state = None;
    }

    pub fn compile(&mut self, gui_game: &mut Option<Gui>, context: &VulkanoContext) {
        log::info!("Project::compile start");
        match self._compile(gui_game, context) {
            Err(error) => log::error!("Project::compile error: {error}"),
            Ok(_) => log::info!("Project::compile end"),
        }
    }

    fn _compile(
        &mut self,
        gui_game: &mut Option<Gui>,
        context: &VulkanoContext,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(state) = self.state.as_mut() {
            // save scene data before unloading library
            let mut data = WorldData::new();
            let mut scene = None;
            if let Some(compiled) = &mut state.compiled {
                compiled.app.command(Command::Save(&mut data));
                scene = compiled.scene.take();
            }

            *gui_game = None; // destroy Gui struct before release dynlib to fix egui crash problem
            state.compiled = None; // prevent steel.dll from being loaded twice at same time

            let lib_path = PathBuf::from("target/debug/steel.dll");
            if lib_path.exists() {
                fs::remove_file(&lib_path)?;
            }

            Self::_handle_pdb_file(Regex::new(r"^steel\.pdb$").unwrap())?;
            Self::_handle_pdb_file(Regex::new(r"^steel_proc-.*\.pdb$").unwrap())?;

            Self::_modify_files_while_compiling(&state.path, None, Self::_build_steel_dynlib)?;

            let library: Library = unsafe { Library::new(&lib_path)? };

            let setup_logger_fn: Symbol<
                fn(&'static dyn Log, LevelFilter) -> Result<(), SetLoggerError>,
            > = unsafe { library.get(b"setup_logger")? };
            setup_logger_fn(log::logger(), log::max_level())?;

            let create_app_fn: Symbol<fn() -> Box<dyn App>> = unsafe { library.get(b"create")? };
            let mut app = create_app_fn();
            app.init(InitInfo {
                platform: Platform::new_editor(state.path.clone()),
                context,
                scene: None,
            });

            // restore game world from scene data
            app.command(Command::Reload(&data));
            app.command(Command::SetCurrentScene(scene.clone()));

            state.compiled = Some(ProjectCompiledState {
                app,
                library,
                data,
                running: false,
                scene,
            });
            Ok(())
        } else {
            Err(ProjectError::new("No open project").boxed())
        }
    }

    /// There is a problem with compilation failure due to the pdb file being locked:
    /// https://developercommunity.visualstudio.com/t/pdb-is-locked-even-after-dll-is-unloaded/690640
    /// We avoid this problem by rename it so that compiler can generate a new pdb file.
    fn _handle_pdb_file(pdb_file_regex: Regex) -> Result<(), Box<dyn Error>> {
        let pdb_dir = PathBuf::from("target/debug/deps");
        if !pdb_dir.exists() {
            return Ok(()); // currently no pdb file exists
        }
        let pdb_file_name = fs::read_dir(&pdb_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_file())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|file_name| pdb_file_regex.is_match(file_name))
            .collect::<Vec<_>>();
        if pdb_file_name.is_empty() {
            return Ok(()); // currently no pdb file exists
        } else if pdb_file_name.len() > 1 {
            log::warn!("Found more than one pdb file: {pdb_file_name:?}");
        }
        let pdb_file_name = &pdb_file_name[0];
        let pdb_file = pdb_dir.join(pdb_file_name);
        let pdb_files = fs::read_dir(&pdb_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_file())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|file_name| file_name.starts_with(pdb_file_name))
            .filter_map(|file_name| {
                file_name[pdb_file_name.len()..file_name.len()]
                    .parse::<u32>()
                    .ok()
                    .map(|n| (file_name, n))
            })
            .filter(|(file_name, _)| fs::remove_file(pdb_dir.join(file_name)).is_err())
            .collect::<Vec<_>>();
        let n = match pdb_files.iter().max_by(|(_, n1), (_, n2)| n1.cmp(n2)) {
            Some((_, n)) => n + 1,
            None => 1,
        };
        let to_pdb_file = pdb_dir.join(format!("{pdb_file_name}{n}"));
        log::info!(
            "Rename {} to {}, pdb_files={pdb_files:?}",
            pdb_file.display(),
            to_pdb_file.display()
        );
        if let Err(error) = fs::rename(pdb_file, to_pdb_file) {
            log::warn!("Failed to rename {pdb_file_name}, error={error}");
        }
        Ok(())
    }

    fn _build_steel_dynlib() -> Result<(), Box<dyn Error>> {
        log::info!("$ cargo build -p steel-dynlib");
        std::process::Command::new("cargo")
            .arg("build")
            .arg("-p")
            .arg("steel-dynlib")
            .spawn()?
            .wait()?; // TODO: non-blocking wait
        Ok(())
    }

    /// A helper function to temporily modify some files before compiling and restore them after compiling.
    /// 1. Modify "steel-project path" of steel-client/Cargo.toml and steel-dynlib/Cargo.toml to project_path.
    /// (Note: we must modify both of them at same time even when only one project is compiling,
    /// because cargo requires that all path of same dependency in a workspace must be same)
    /// 2. Modify "scene_path" of steel-client/src/lib.rs to scene_path, just pass None when not compile steel-client.
    fn _modify_files_while_compiling(
        project_path: impl AsRef<Path>,
        scene_path: Option<PathBuf>,
        compile_fn: fn() -> Result<(), Box<dyn Error>>,
    ) -> Result<(), Box<dyn Error>> {
        let cargo_toml_paths = [
            PathBuf::from("steel-build/steel-client/Cargo.toml"),
            PathBuf::from("steel-build/steel-dynlib/Cargo.toml"),
        ];
        let steel_client_src_path = PathBuf::from("steel-build/steel-client/src/lib.rs");

        let mut cargo_toml_original_contents = Vec::new();
        for path in &cargo_toml_paths {
            log::info!("Read {}", path.display());
            let original_content = fs::read_to_string(path)?;
            let num_match = original_content
                .matches(STEEL_PROJECT_PATH)
                .collect::<Vec<_>>()
                .len();
            if num_match != 1 {
                return Err(ProjectError::new(format!(
                    "Expected only 1 match '{STEEL_PROJECT_PATH}' in {}, \
                    actual number of match: {num_match}",
                    path.display()
                ))
                .boxed());
            }
            cargo_toml_original_contents.push(original_content);
        }
        let steel_client_src_original_content = if scene_path.is_some() {
            log::info!("Read {}", steel_client_src_path.display());
            let original_content = fs::read_to_string(&steel_client_src_path)?;
            let num_match = original_content
                .matches(SCENE_PATH)
                .collect::<Vec<_>>()
                .len();
            if num_match != 1 {
                return Err(ProjectError::new(format!(
                    "Expected only 1 match '{SCENE_PATH}' in {}, \
                    actual number of match: {num_match}",
                    steel_client_src_path.display()
                ))
                .boxed());
            }
            Some(original_content)
        } else {
            None
        };

        let open_project_path = project_path
            .as_ref()
            .to_str()
            .ok_or(ProjectError::new(format!(
                "{} to_str() returns None",
                project_path.as_ref().display()
            )))?
            .replace("\\", "/");
        let cargo_toml_new_contents = cargo_toml_original_contents
            .iter()
            .map(|original_content| {
                original_content.replacen(STEEL_PROJECT_PATH, open_project_path.as_str(), 1)
            })
            .collect::<Vec<_>>();

        let scene_path = if let Some(scene_path) = &scene_path {
            Some(
                scene_path
                    .to_str()
                    .ok_or(ProjectError::new(format!(
                        "{} to_str() returns None",
                        scene_path.display()
                    )))?
                    .replace("\\", "/"),
            )
        } else {
            None
        };
        let steel_client_src_new_content =
            steel_client_src_original_content
                .as_ref()
                .map(|original_content| {
                    original_content.replacen(SCENE_PATH, scene_path.unwrap().as_str(), 1)
                });

        let mut num_modified_cargo_toml = cargo_toml_paths.len();
        for i in 0..cargo_toml_paths.len() {
            log::info!("Modify {}", cargo_toml_paths[i].display());
            if let Err(error) = fs::write(&cargo_toml_paths[i], &cargo_toml_new_contents[i]) {
                log::error!(
                    "Failed to modify {}, error={error}",
                    cargo_toml_paths[i].display()
                );
                num_modified_cargo_toml = i;
                break;
            }
        }
        let mut modify_success = num_modified_cargo_toml == cargo_toml_paths.len();
        if modify_success {
            if let Some(steel_client_src_new_content) = steel_client_src_new_content {
                log::info!("Modify {}", steel_client_src_path.display());
                if let Err(error) = fs::write(&steel_client_src_path, steel_client_src_new_content)
                {
                    log::error!(
                        "Failed to modify {}, error={error}",
                        steel_client_src_path.display()
                    );
                    modify_success = false;
                }
            }
        }

        let compile_result = if modify_success {
            compile_fn()
        } else {
            Err(ProjectError::new("Not compile due to previous modify error").boxed())
        };

        for i in 0..num_modified_cargo_toml {
            log::info!("Restore {}", cargo_toml_paths[i].display());
            if let Err(error) = fs::write(&cargo_toml_paths[i], &cargo_toml_original_contents[i]) {
                log::error!(
                    "There is an error while writing original content back to {}! \
                    You have to restore this file by yourself, error={error}",
                    cargo_toml_paths[i].display()
                );
            }
        }
        if let Some(steel_client_src_original_content) = steel_client_src_original_content {
            log::info!("Restore {}", steel_client_src_path.display());
            if let Err(error) = fs::write(&steel_client_src_path, steel_client_src_original_content)
            {
                log::error!(
                    "There is an error while writing original content back to {}! \
                    You have to restore this file by yourself, error={error}",
                    steel_client_src_path.display()
                );
            }
        }

        compile_result
    }

    pub fn is_compiled(&self) -> bool {
        return self.compiled_ref().is_some();
    }

    pub fn app(&mut self) -> Option<&mut Box<dyn App>> {
        Some(&mut self.compiled_mut()?.app)
    }

    fn compiled_ref(&self) -> Option<&ProjectCompiledState> {
        self.state.as_ref()?.compiled.as_ref()
    }

    fn compiled_mut(&mut self) -> Option<&mut ProjectCompiledState> {
        self.state.as_mut()?.compiled.as_mut()
    }

    pub fn export_windows(&self) {
        log::info!("Project::export_windows start");
        match self._export_windows() {
            Err(error) => log::error!("Project::export_windows error: {error}"),
            Ok(_) => log::info!("Project::export_windows end"),
        }
    }

    fn _export_windows(&self) -> Result<(), Box<dyn Error>> {
        if let Some(state) = self.state.as_ref() {
            let scene = self.scene_relative_path().ok_or(ProjectError::new(
                "No scene is opened, you must open a scene as init scene before export",
            ))?;

            Self::_export_asset(
                self.asset_dir()
                    .expect("self.asset_dir() must be some if self.state is some"),
                state.path.join("build/windows/asset"),
            )?;

            let exe_path = PathBuf::from("target/debug/steel-client.exe");
            if exe_path.exists() {
                fs::remove_file(&exe_path)?;
            }
            Self::_modify_files_while_compiling(
                &state.path,
                Some(scene),
                Self::_build_steel_client_desktop,
            )?;
            if !exe_path.exists() {
                return Err(
                    ProjectError::new(format!("No output file: {}", exe_path.display())).boxed(),
                );
            }

            let exe_export_path = state.path.join("build/windows/steel-client.exe");
            fs::create_dir_all(exe_export_path.parent().unwrap())?;
            fs::copy(exe_path, &exe_export_path)?;
            log::info!("Exported: {}", exe_export_path.display());

            Ok(())
        } else {
            Err(ProjectError::new("No open project").boxed())
        }
    }

    fn _build_steel_client_desktop() -> Result<(), Box<dyn Error>> {
        log::info!("$ cargo build -p steel-client -F desktop");
        std::process::Command::new("cargo")
            .arg("build")
            .arg("-p")
            .arg("steel-client")
            .arg("-F")
            .arg("desktop")
            .spawn()?
            .wait()?; // TODO: non-blocking wait
        Ok(())
    }

    pub fn export_android(&self) {
        log::info!("Project::export_android start");
        match self._export_android() {
            Err(error) => log::error!("Project::export_android error: {error}"),
            Ok(_) => log::info!("Project::export_android end"),
        }
    }

    fn _export_android(&self) -> Result<(), Box<dyn Error>> {
        if let Some(state) = self.state.as_ref() {
            let scene = self.scene_relative_path().ok_or(ProjectError::new(
                "No scene is opened, you must open a scene as init scene before export",
            ))?;

            Self::_export_asset(
                self.asset_dir()
                    .expect("self.asset_dir() must be some if self.state is some"),
                PathBuf::from("steel-build/android-project/app/src/main/assets"),
            )?;

            // TODO: run following commands:
            // rustup target add aarch64-linux-android
            // cargo install cargo-ndk

            let so_path = PathBuf::from(
                "steel-build/android-project/app/src/main/jniLibs/arm64-v8a/libmain.so",
            );
            if so_path.exists() {
                fs::remove_file(&so_path)?;
            }
            Self::_modify_files_while_compiling(
                &state.path,
                Some(scene),
                Self::_build_steel_client_android,
            )?;
            if !so_path.exists() {
                return Err(
                    ProjectError::new(format!("No output file: {}", so_path.display())).boxed(),
                );
            }

            let apk_path = PathBuf::from(
                "steel-build/android-project/app/build/outputs/apk/debug/app-debug.apk",
            );
            if apk_path.exists() {
                fs::remove_file(&apk_path)?;
            }
            let mut android_project_dir = fs::canonicalize("steel-build/android-project").unwrap();
            // the windows path prefix "\\?\" makes bat fail to run in std::process::Command
            crate::utils::delte_windows_path_prefix(&mut android_project_dir);
            log::info!("{}$ ./gradlew.bat build", android_project_dir.display());
            std::process::Command::new("steel-build/android-project/gradlew.bat")
                .arg("build")
                .current_dir(&android_project_dir)
                .spawn()?
                .wait()?; // TODO: non-blocking wait
            if !apk_path.exists() {
                return Err(
                    ProjectError::new(format!("No output file: {}", apk_path.display())).boxed(),
                );
            }

            // TODO: not run installDebug if no android device connected
            log::info!(
                "{}$ ./gradlew.bat installDebug",
                android_project_dir.display()
            );
            std::process::Command::new("steel-build/android-project/gradlew.bat")
                .arg("installDebug")
                .current_dir(&android_project_dir)
                .spawn()?
                .wait()?; // TODO: non-blocking wait

            let apk_export_path = state.path.join("build/android/steel-client.apk");
            fs::create_dir_all(apk_export_path.parent().unwrap())?;
            fs::copy(apk_path, &apk_export_path)?;
            log::info!("Exported: {}", apk_export_path.display());
            Ok(())
        } else {
            Err(ProjectError::new("No open project").boxed())
        }
    }

    fn _build_steel_client_android() -> Result<(), Box<dyn Error>> {
        log::info!("$ cargo ndk -t arm64-v8a -o steel-build/android-project/app/src/main/jniLibs/ build -p steel-client");
        std::process::Command::new("cargo")
            .arg("ndk")
            .arg("-t")
            .arg("arm64-v8a")
            .arg("-o")
            .arg("steel-build/android-project/app/src/main/jniLibs/")
            .arg("build")
            .arg("-p")
            .arg("steel-client")
            .spawn()?
            .wait()?; // TODO: non-blocking wait
        Ok(())
    }

    fn _export_asset(src: PathBuf, dst: PathBuf) -> std::io::Result<()> {
        if dst.is_dir() {
            fs::remove_dir_all(&dst)?;
        }
        Self::_copy_dir_all(src, dst)?;
        Ok(())
    }

    fn _copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
        fs::create_dir_all(&dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                Self::_copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
            } else {
                let dst = dst.as_ref().join(entry.file_name());
                fs::copy(entry.path(), &dst)?;
                log::info!("Exported: {}", dst.display());
            }
        }
        Ok(())
    }

    pub fn set_running(&mut self, running: bool) {
        if let Some(compiled) = self.compiled_mut() {
            compiled.running = running;
        }
    }

    pub fn is_running(&self) -> bool {
        self.compiled_ref().is_some_and(|compiled| compiled.running)
    }

    pub fn save_to_memory(&mut self) {
        if let Some(compiled) = self.compiled_mut() {
            compiled.app.command(Command::Save(&mut compiled.data));
        }
    }

    pub fn load_from_memory(&mut self) {
        if let Some(compiled) = self.compiled_mut() {
            compiled.app.command(Command::Reload(&compiled.data));
            compiled
                .app
                .command(Command::SetCurrentScene(compiled.scene.clone()));
        }
    }

    /// Return the opened project directory, or None if no project is opened
    pub fn project_dir(&self) -> Option<&PathBuf> {
        self.state.as_ref().map(|state| &state.path)
    }

    /// Return the asset dir under the opened project directory, or None if no project is opened
    pub fn asset_dir(&self) -> Option<PathBuf> {
        self.project_dir().map(|path| path.join("asset"))
    }

    /// Return the absolute path of current opened scene, or None if no scene is opened
    pub fn scene_absolute_path(&self) -> Option<PathBuf> {
        if let Some(compiled) = self.compiled_ref() {
            compiled.scene.as_ref().map(|scene| {
                let asset_dir = self
                    .asset_dir()
                    .expect("self.asset_dir() must be some when self.compiled_ref() is some");
                asset_dir.join(scene)
            })
        } else {
            None
        }
    }

    /// Return the relative path of current opened scene, which is relative to
    /// the asset directory of opened project, or None if no scene is opened
    pub fn scene_relative_path(&self) -> Option<PathBuf> {
        if let Some(compiled) = self.compiled_ref() {
            compiled.scene.clone()
        } else {
            None
        }
    }

    /// Convert scene absolute path to relative path, which is relative to the asset
    /// directory of opened project, or return None if scene absolute path is invalid.
    /// A scene absolute path is valid if it is under asset folder of opened project.
    pub fn convert_to_scene_relative_path<'a>(
        &self,
        scene_absolute_path: &'a Path,
    ) -> Option<&'a Path> {
        if let Some(asset) = self.asset_dir() {
            scene_absolute_path.strip_prefix(asset).ok()
        } else {
            None
        }
    }

    pub fn new_scene(&mut self) {
        if let Some(compiled) = self.compiled_mut() {
            compiled.app.command(Command::ClearEntity);
            compiled.app.command(Command::SetCurrentScene(None));
            compiled.scene = None;
        }
    }

    /// Save current world_data to file, scene is the save file path,
    /// which is relative to the asset directory of opened project
    pub fn save_scene(&mut self, scene: impl Into<PathBuf>) {
        let asset_dir = self.asset_dir();
        if let Some(compiled) = self.compiled_mut() {
            compiled.app.command(Command::Save(&mut compiled.data));
            let world_data = Self::_cut_world_data(&compiled.data);
            let asset_dir =
                asset_dir.expect("self.asset_dir() must be some when self.compiled_mut() is some");
            let scene = scene.into();
            let scene_abs = asset_dir.join(&scene);
            match Self::_save_to_file(&world_data, &scene_abs) {
                Ok(_) => {
                    compiled
                        .app
                        .command(Command::SetCurrentScene(Some(scene.clone())));
                    compiled.scene = Some(scene);
                }
                Err(err) => log::warn!(
                    "Failed to save WorldData to {} because {err}",
                    scene_abs.display()
                ),
            }
        }
    }

    fn _save_to_file(data: &WorldData, path: &PathBuf) -> Result<(), Box<dyn Error>> {
        let s = serde_json::to_string_pretty(data)?;
        fs::write(path, s)?;
        Ok(())
    }

    /// Erase generation value of EntityId and skip read only values,
    /// because they are useless when loading from file.
    fn _cut_world_data(world_data: &WorldData) -> WorldData {
        let mut world_data_cut = WorldData::new();
        for (eid, entity_data) in &world_data.entities {
            let mut entity_data_cut = EntityData::new();
            for (comopnent_name, component_data) in &entity_data.components {
                let cut_read_only = comopnent_name != "Child" && comopnent_name != "Parent"; // TODO: use a more generic way to allow read-only values of some components to save to file
                let component_data_cut = Self::_cut_data(component_data, cut_read_only);
                entity_data_cut
                    .components
                    .insert(comopnent_name.clone(), component_data_cut);
            }
            world_data_cut
                .entities
                .insert(Self::_erase_generation(eid), entity_data_cut);
        }
        for (unique_name, unique_data) in &world_data.uniques {
            let cut_read_only = unique_name != "Hierarchy"; // TODO: use a more generic way to allow read-only values of some uniques to save to file
            let unique_data_cut = Self::_cut_data(unique_data, cut_read_only);
            world_data_cut
                .uniques
                .insert(unique_name.clone(), unique_data_cut);
        }
        world_data_cut
    }

    fn _cut_data(data: &Data, cut_read_only: bool) -> Data {
        let mut data_cut = Data::new();
        for (name, value) in &data.values {
            if !(cut_read_only && matches!(data.limits.get(name), Some(Limit::ReadOnly))) {
                let value = match value {
                    Value::Entity(e) => Value::Entity(Self::_erase_generation(e)),
                    Value::VecEntity(v) => {
                        Value::VecEntity(v.iter().map(|e| Self::_erase_generation(e)).collect())
                    }
                    _ => value.clone(),
                };
                data_cut.values.insert(name.clone(), value);
            }
        }
        data_cut
    }

    fn _erase_generation(eid: &EntityId) -> EntityId {
        if *eid == EntityId::dead() {
            *eid
        } else {
            EntityId::new_from_index_and_gen(eid.index(), 0)
        }
    }

    /// Load world_data from file, scene is the load file path,
    /// which is relative to the asset directory of opened project
    pub fn load_scene(&mut self, scene: impl Into<PathBuf>) {
        let asset_dir = self.asset_dir();
        if let Some(compiled) = self.compiled_mut() {
            let asset_dir =
                asset_dir.expect("self.asset_dir() must be some if self.compiled_mut() is some");
            let scene = scene.into();
            let scene_abs = asset_dir.join(&scene);
            match Self::_load_from_file(&scene_abs) {
                Ok(data) => {
                    compiled.data = data;
                    compiled.app.command(Command::Reload(&compiled.data));
                    compiled
                        .app
                        .command(Command::SetCurrentScene(Some(scene.clone())));
                    compiled.scene = Some(scene);
                }
                Err(err) => log::warn!(
                    "Failed to load WorldData from {} because {err}",
                    scene_abs.display()
                ),
            }
        }
    }

    fn _load_from_file(path: &PathBuf) -> Result<WorldData, Box<dyn Error>> {
        let s = fs::read_to_string(path)?;
        Ok(serde_json::from_str::<WorldData>(&s)?)
    }
}

#[derive(Debug)]
struct ProjectError {
    message: String,
}

impl ProjectError {
    fn new(message: impl Into<String>) -> ProjectError {
        ProjectError {
            message: message.into(),
        }
    }

    fn boxed(self) -> Box<dyn Error> {
        Box::new(self)
    }
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProjectError({})", self.message)
    }
}

impl Error for ProjectError {}

const GITIGNORE: &'static str = "/target
/build
";

const CARGO_TOML: &'static str = r#"[package]
name = "steel-project"
version = "0.1.0"
edition = "2021"

[lib]
name = "steel"

[dependencies]
steel-engine = { version = "0.2.0", path = "../../steel-engine" }

vulkano = "0.34.1"
vulkano-shaders = "0.34.0"
vulkano-util = "0.34.1"
egui_winit_vulkano = "0.27.0"
egui = "0.24.1"
log = "0.4"
winit = { version = "0.28.6", features = [ "android-game-activity" ] }
winit_input_helper = "0.14.1"
shipyard = { version = "0.7.1", features = [ "serde1" ] }
rayon = "1.8.0"
parry2d = "0.13.5"
rapier2d = { version = "0.17.2", features = [ "debug-render" ] }
nalgebra = { version = "0.32.3", features = [ "convert-glam024" ] }
glam = { version = "0.24.2", features = [ "serde" ] }
serde = { version = "1.0", features = [ "derive" ] }
serde_json = "1.0"
indexmap = { version = "2.2.2", features = [ "serde" ] }
"#;

const LIB_RS: &'static str = "use steel::app::{App, SteelApp};

#[no_mangle]
pub fn create() -> Box<dyn App> {
    SteelApp::new().boxed()
}
";

const STEEL_PROJECT_PATH: &'static str = "../steel-project";

const SCENE_PATH: &'static str = "scene_path";
