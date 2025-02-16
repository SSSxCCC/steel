use crate::utils::LocalData;
use egui_winit_vulkano::Gui;
use libloading::{Library, Symbol};
use log::{LevelFilter, Log, SetLoggerError};
use notify::{
    event::{ModifyKind, RenameMode},
    Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use regex::Regex;
use shipyard::EntityId;
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::{
        mpsc::{Receiver, TryRecvError},
        Arc,
    },
};
use steel_common::{
    app::{App, Command, InitInfo},
    asset::{AssetId, AssetIdType, AssetInfo},
    camera::SceneCamera,
    data::{SceneData, WorldData},
    platform::Platform,
    prefab::PrefabData,
};
use vulkano_util::context::VulkanoContext;

/// The project state.
pub struct Project {
    state: Option<ProjectState>,
    ray_tracing_supported: bool,
}

impl Project {
    /// Create a new project object that manages the state of the project.
    /// And open the last project according to the local data.
    pub fn new(
        ray_tracing_supported: bool,
        local_data: &mut LocalData,
        scene_camera: &mut SceneCamera,
        window_title: &mut Option<String>,
        context: &VulkanoContext,
        gui_game: &mut Option<Gui>,
    ) -> Self {
        let mut project = Project {
            state: None,
            ray_tracing_supported,
        };

        if local_data.open_last_project_on_start && local_data.last_open_project_path.is_dir() {
            log::info!(
                "Open last project: {}",
                local_data.last_open_project_path.display()
            );
            project.open(
                local_data.last_open_project_path.clone(),
                local_data,
                window_title,
                gui_game,
            );
            project.compile(local_data, scene_camera, gui_game, context);
        }

        project
    }

    /// Called when the editor is exiting.
    pub fn exit(&self, local_data: &mut LocalData, scene_camera: SceneCamera) {
        self.compiled_ref()
            .map(|compiled| compiled.exit(local_data, scene_camera));
    }

    /// Open or create a project at the given path.
    pub fn open(
        &mut self,
        path: PathBuf,
        local_data: &mut LocalData,
        window_title: &mut Option<String>,
        gui_game: &mut Option<Gui>,
    ) {
        match Self::open_inner(&path) {
            Err(error) => {
                local_data.open_last_project_on_start = false;
                local_data.save();

                log::error!(
                    "Failed to open project, path={}, error={error}",
                    path.display()
                );
            }
            Ok(_) => {
                local_data.last_open_project_path = path.clone();
                local_data.open_last_project_on_start = true;
                if self.state.is_some() {
                    local_data.scene_data = None; // we are closing previous project, just clear scene data
                }
                local_data.save();

                *window_title = Some(format!("Steel Editor ({})", path.display()));

                *gui_game = None; // destroy Gui struct before release dynlib to fix egui crash problem
                self.state = Some(ProjectState {
                    path,
                    compiled: None,
                });
            }
        }
    }

    fn open_inner(path: &PathBuf) -> Result<(), Box<dyn Error>> {
        if !path.is_dir() {
            fs::create_dir_all(&path)?;
            log::debug!("Created directory: {}", path.display());
        }

        let gitignore_file = path.join(".gitignore");
        if !gitignore_file.exists() {
            fs::write(&gitignore_file, GITIGNORE)?;
            log::debug!("Created: {}", gitignore_file.display());

            if let Err(error) = Self::init_git(path) {
                log::warn!("Failed to init git, error={error}");
            }
        }

        let cargo_toml_file = path.join("Cargo.toml");
        if !cargo_toml_file.exists() {
            if let Ok(mut steel_engine_dir) = fs::canonicalize("steel-engine") {
                crate::utils::delte_windows_path_prefix(&mut steel_engine_dir);
                let steel_engine_dir = steel_engine_dir
                    .to_str()
                    .ok_or(format!(
                        "{} to_str() returns None",
                        steel_engine_dir.display()
                    ))?
                    .replace("\\", "/");
                fs::write(
                    &cargo_toml_file,
                    CARGO_TOML.replacen("../../steel-engine", steel_engine_dir.as_str(), 1),
                )?;
            } else {
                // "steel-engine" folder not found, maybe steel-editor is running in exported executable, just write CARGO_TOML content.
                fs::write(&cargo_toml_file, CARGO_TOML)?;
            }
            log::debug!("Created: {}", cargo_toml_file.display());
        }

        let src_dir = path.join("src");
        if !src_dir.is_dir() {
            fs::create_dir(&src_dir)?;
            log::debug!("Created directory: {}", src_dir.display());
        }

        let lib_rs_file = src_dir.join("lib.rs");
        if !lib_rs_file.is_file() {
            fs::write(&lib_rs_file, LIB_RS)?;
            log::debug!("Created: {}", lib_rs_file.display());
        }

        let asset_dir = path.join("asset");
        if !asset_dir.is_dir() {
            fs::create_dir(&asset_dir)?;
            log::debug!("Created directory: {}", asset_dir.display());
        }

        Ok(())
    }

    fn init_git(project_path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
        std::process::Command::new("git")
            .arg("init")
            .current_dir(project_path)
            .spawn()?
            .wait()?;
        Ok(())
    }

    /// Are any project open?
    pub fn is_open(&self) -> bool {
        self.state.is_some()
    }

    /// Close the current project.
    pub fn close(
        &mut self,
        local_data: &mut LocalData,
        window_title: &mut Option<String>,
        gui_game: &mut Option<Gui>,
    ) {
        local_data.open_last_project_on_start = false;
        local_data.scene_data = None;
        local_data.save();

        *window_title = Some("Steel Editor".into());

        *gui_game = None; // destroy Gui struct before release dynlib to fix egui crash problem
        self.state = None;
    }

    pub fn compile(
        &mut self,
        local_data: &mut LocalData,
        scene_camera: &mut SceneCamera,
        gui_game: &mut Option<Gui>,
        context: &VulkanoContext,
    ) {
        if let Some(state) = self.state.as_mut() {
            log::info!("Project::compile start");
            match state.compile(
                local_data,
                scene_camera,
                gui_game,
                context,
                self.ray_tracing_supported,
            ) {
                Err(error) => log::error!("Project::compile error: {error}"),
                Ok(_) => log::info!("Project::compile end"),
            }
        }
    }

    /// Maintain asset files in asset directory:
    /// 1. If an asset file is added/created, add/create asset info file, and insert the asset into AssetManager.
    /// 2. If an asset file is removed/deleted, remove/delete asset info file, and delete the asset in AssetManager.
    /// 3. When a file is moved, two events are generated: remove and create, so 1 and 2 have handled file moving.
    /// This will create a new asset id for the moved file! Therefore if user want to keep the asset id
    /// after moving a file, the user must move its corresponding ".asset" file at the same time!
    /// 4. If an asset file is renamed, rename asset info file, and update the asset path in AssetManager.
    /// 5. If an asset file is modified, clear the asset cache in AssetManager.
    pub fn maintain_asset_dir(&mut self) {
        let asset_dir = self.asset_dir();
        if let Some(compiled) = self.compiled_mut() {
            let asset_dir =
                asset_dir.expect("self.asset_dir() must be some when self.compiled_mut() is some");
            loop {
                match compiled.receiver.try_recv() {
                    Err(TryRecvError::Empty) => break, // no more events, just break
                    Err(e) => log::error!("Project::maintain_asset_dir: try receive error: {e}"),
                    Ok(event) => match event.kind {
                        EventKind::Remove(_) => {
                            for path in event.paths {
                                let asset_relative_path = path.strip_prefix(&asset_dir).unwrap();
                                // the path has been removed, so we do not know if it is file or directory
                                // for removed directory
                                compiled
                                    .app
                                    .command(Command::DeleteAssetDir(asset_relative_path));
                                // for removed file
                                let asset_info_path = AssetInfo::asset_path_to_asset_info_path(path);
                                match Self::read_asset_info(&asset_info_path) {
                                    Ok(asset_info) => {
                                        if let Some(asset_info) = asset_info {
                                            compiled
                                                .app
                                                .command(Command::DeleteAsset(asset_info.id));
                                            if let Err(e) = fs::remove_file(&asset_info_path) {
                                                log::warn!("Project::maintain_asset_dir: remove {} error: {e}", asset_info_path.display());
                                            }
                                        }
                                    }
                                    Err(e) => log::error!("Project::maintain_asset_dir: read asset info from {} error: {e}", asset_info_path.display()),
                                }
                            }
                        }
                        EventKind::Modify(ModifyKind::Name(mode)) => {
                            match mode {
                                RenameMode::From => {
                                    if let Some(last_rename_from_event) = &compiled.last_rename_from_event {
                                        log::warn!("Project::maintain_asset_dir: unexpected received another rename from event! previous: {last_rename_from_event:?}");
                                    }
                                    if event.paths.len() != 1 {
                                        log::warn!("Project::maintain_asset_dir: length of rename from event paths is not 1! event: {event:?}");
                                    }
                                    compiled.last_rename_from_event = Some(event);
                                }
                                RenameMode::To => {
                                    if let Some(last_rename_from_event) = compiled.last_rename_from_event.take() {
                                        if event.paths.len() != 1 {
                                            log::warn!("Project::maintain_asset_dir: length of rename to event paths is not 1! event: {event:?}");
                                        }
                                        if event.paths[0].is_file() && !event.paths[0].extension().is_some_and(|ext| ext == "asset") {
                                            let from_asset_info_file = AssetInfo::asset_path_to_asset_info_path(&last_rename_from_event.paths[0]);
                                            let to_asset_info_file = AssetInfo::asset_path_to_asset_info_path(&event.paths[0]);
                                            if from_asset_info_file.exists() && !to_asset_info_file.exists() {
                                                match fs::rename(&from_asset_info_file, &to_asset_info_file) {
                                                    Ok(_) => {
                                                        match Self::read_asset_info(&to_asset_info_file) {
                                                            Ok(asset_info) => {
                                                                if let Some(asset_info) = asset_info {
                                                                    let asset_relative_path = event.paths[0].strip_prefix(&asset_dir).unwrap();
                                                                    compiled.app.command(Command::UpdateAssetPath(asset_info.id, asset_relative_path.to_path_buf()));
                                                                } else {
                                                                    log::error!("Project::maintain_asset_dir: read asset info from {} returns None!", to_asset_info_file.display())
                                                                }
                                                            }
                                                            Err(e) => log::error!("Project::maintain_asset_dir: read asset info from {} error: {e}", to_asset_info_file.display()),
                                                        }
                                                    }
                                                    Err(e) => log::error!("Project::maintain_asset_dir: rename asset info file error: {e}, from: {}, to: {}", from_asset_info_file.display(), to_asset_info_file.display()),
                                                }
                                            }
                                        }
                                    } else {
                                        log::warn!("Project::maintain_asset_dir: unexpected received rename to event: {event:?}");
                                    }
                                }
                                _ => log::warn!("Project::maintain_asset_dir: receiving an unknown RenameMode event: {event:?}"),
                            }
                        }
                        EventKind::Modify(_) | EventKind::Create(_) => {
                            for path in event.paths {
                                if path.is_file() && !path.extension().is_some_and(|ext| ext == "asset") {
                                    let asset_relative_path =
                                        path.strip_prefix(&asset_dir).unwrap();
                                    if let Err(e) = Self::get_asset_info_and_insert(
                                        &asset_dir,
                                        asset_relative_path,
                                        &compiled.app,
                                        true,
                                    ) {
                                        log::error!("Project::maintain_asset_dir: get asset info and insert from {} error: {e}", path.display());
                                    }
                                }
                            }
                        }
                        _ => (),
                    },
                }
            }
        }
    }

    /// Are current open project compiled?
    pub fn is_compiled(&self) -> bool {
        return self.compiled_ref().is_some();
    }

    /// Get the app of current open project.
    pub fn app(&self) -> Option<&Box<dyn App>> {
        Some(&self.compiled_ref()?.app)
    }

    fn compiled_ref(&self) -> Option<&ProjectCompiledState> {
        self.state.as_ref()?.compiled.as_ref()
    }

    fn compiled_mut(&mut self) -> Option<&mut ProjectCompiledState> {
        self.state.as_mut()?.compiled.as_mut()
    }

    /// Export current open project to windows platform.
    pub fn export_windows(&self) {
        if let Some(state) = self.state.as_ref() {
            log::info!("Project::export_windows start");
            match state.export_windows() {
                Err(error) => log::error!("Project::export_windows error: {error}"),
                Ok(_) => log::info!("Project::export_windows end"),
            }
        }
    }

    /// Export current open project to android platform.
    pub fn export_android(&self) {
        if let Some(state) = self.state.as_ref() {
            log::info!("Project::export_android start");
            match state.export_android() {
                Err(error) => log::error!("Project::export_android error: {error}"),
                Ok(_) => log::info!("Project::export_android end"),
            }
        }
    }

    /// Set the running state of current open project.
    pub fn set_running(&mut self, running: bool) {
        if let Some(compiled) = self.compiled_mut() {
            compiled.running = running;
        }
    }

    /// Is the current open project running?
    pub fn is_running(&self) -> bool {
        self.compiled_ref().is_some_and(|compiled| compiled.running)
    }

    /// Save scene data to memory, which can be loaded later by [Self::load_from_memory].
    pub fn save_to_memory(&mut self, prefab_data_override: Option<(EntityId, Arc<PrefabData>)>) {
        self.compiled_mut()
            .map(|compiled| compiled.save_to_memory(prefab_data_override));
    }

    /// Load scene data from memory, which comes from [Self::save_to_memory].
    /// This will reload the scene and set it as current scene.
    /// Be remember to set load_world_data_this_frame to false after calling this function.
    pub fn load_from_memory(&self) {
        self.compiled_ref()
            .map(|compiled| compiled.load_from_memory());
    }

    /// Return the asset directory under the open project directory, or None if no project is open.
    pub fn asset_dir(&self) -> Option<PathBuf> {
        self.state.as_ref().map(|state| state.asset_dir())
    }

    /// Return the absolute path of current open scene, or None if no scene is open.
    pub fn scene_absolute_path(&self) -> Option<PathBuf> {
        self.compiled_ref()
            .and_then(|compiled| compiled.scene_absolute_path(self.asset_dir().unwrap()))
    }

    /// Return the relative path of current open scene, which is relative to
    /// the asset directory of open project, or None if no scene is open.
    pub fn scene_relative_path(&self) -> Option<PathBuf> {
        self.compiled_ref()
            .and_then(|compiled| compiled.scene_relative_path())
    }

    /// Convert scene absolute path to relative path, which is relative to the asset
    /// directory of open project, or return None if scene absolute path is invalid.
    /// A scene absolute path is valid if it is under asset folder of open project.
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

    /// Create a new scene and set current scene to None.
    pub fn new_scene(&mut self) {
        self.compiled_mut().map(|compiled| compiled.new_scene());
    }

    /// Convert current [WorldData] to [SceneData] and save to file, scene is the
    /// save file path, which is relative to the asset directory of open project.
    pub fn save_scene(&mut self, scene: impl AsRef<Path>) {
        let asset_dir = self.asset_dir();
        self.compiled_mut().map(|compiled| {
            if let Err(e) = compiled.save_scene(scene, asset_dir.unwrap()) {
                log::error!("Project::save_scene error: {e}");
            }
        });
    }

    /// Load [SceneData] from file and convert to [WorldData], scene is the load file path,
    /// which is relative to the asset directory of open project.
    pub fn load_scene(&mut self, scene: impl AsRef<Path>) {
        let asset_dir = self.asset_dir();
        self.compiled_mut().map(|compiled| {
            if let Err(e) = compiled.load_scene(scene, asset_dir.unwrap()) {
                log::error!("Project::load_scene error: {e}");
            }
        });
    }

    /// Get the asset info of file in asset_path.
    /// * If asset info file already exists:
    /// Read asset info file to get AssetInfo, and insert asset id to AssetManager if always_insert is true.
    /// * If asset info file not exists:
    /// Create a new AssetInfo, write AssetInfo into asset info file, and insert asset id to AssetManager.
    ///
    /// If you call this function to create asset info for newly created asset file, you can set always_insert to false,
    /// because we will receive a file create event and call this function with always_insert as true in the next frame.
    pub fn get_asset_info_and_insert(
        asset_dir: impl AsRef<Path>,
        asset_path: impl AsRef<Path>,
        app: &Box<dyn App>,
        always_insert: bool,
    ) -> Result<AssetInfo, Box<dyn Error>> {
        let abs_asset_path = asset_dir.as_ref().join(&asset_path);
        let abs_asset_info_path = AssetInfo::asset_path_to_asset_info_path(abs_asset_path);
        if let Some(asset_info) = Self::read_asset_info(&abs_asset_info_path)? {
            if always_insert {
                app.command(Command::InsertAsset(
                    asset_info.id,
                    asset_path.as_ref().to_path_buf(),
                ));
            }
            Ok(asset_info)
        } else {
            let asset_info = Self::create_asset_info(app);
            fs::write(
                abs_asset_info_path,
                serde_json::to_string_pretty(&asset_info)?,
            )?;
            app.command(Command::InsertAsset(
                asset_info.id,
                asset_path.as_ref().to_path_buf(),
            ));
            Ok(asset_info)
        }
    }

    /// Get the asset info from asset info file. Return None if asset info file not exists.
    fn read_asset_info(
        abs_asset_info_path: impl AsRef<Path>,
    ) -> Result<Option<AssetInfo>, Box<dyn Error>> {
        if abs_asset_info_path.as_ref().exists() {
            Ok(Some(serde_json::from_str(&fs::read_to_string(
                abs_asset_info_path,
            )?)?))
        } else {
            Ok(None)
        }
    }

    /// Create a new AssetInfo with a new random id.
    fn create_asset_info(app: &Box<dyn App>) -> AssetInfo {
        loop {
            let asset_id = AssetId::new(rand::random::<AssetIdType>());
            if asset_id == AssetId::INVALID {
                continue;
            }
            let mut exists = false;
            app.command(Command::AssetIdExists(asset_id, &mut exists));
            if !exists {
                return AssetInfo::new(asset_id);
            }
        }
    }
}

/// The state of current open project.
struct ProjectState {
    path: PathBuf,
    compiled: Option<ProjectCompiledState>,
}

impl ProjectState {
    fn asset_dir(&self) -> PathBuf {
        self.path.join("asset")
    }

    fn compile(
        &mut self,
        local_data: &mut LocalData,
        scene_camera: &mut SceneCamera,
        gui_game: &mut Option<Gui>,
        context: &VulkanoContext,
        ray_tracing_supported: bool,
    ) -> Result<(), Box<dyn Error>> {
        let mut scene = None;
        let mut scene_data = SceneData::default();
        let mut local_scene_camera = None;
        if let Some(compiled) = &mut self.compiled {
            // save scene data before unloading library
            scene = compiled.scene.take();
            scene_data = compiled.get_scene_data(None);
        } else if let Some((scene_camera, local_scene, local_scene_data)) =
            local_data.scene_data.clone()
        {
            local_scene_camera = Some(scene_camera);
            scene = local_scene;
            scene_data = local_scene_data;
        }

        *gui_game = None; // destroy Gui struct before release dynlib to fix egui crash problem
        self.compiled = None; // prevent steel.dll from being loaded twice at same time

        let lib_path = PathBuf::from("target/debug/steel.dll");
        if lib_path.exists() {
            fs::remove_file(&lib_path)?;
        }

        Self::handle_pdb_file(Regex::new(r"^steel\.pdb$").unwrap())?;
        Self::handle_pdb_file(Regex::new(r"^steel_proc-.*\.pdb$").unwrap())?;

        Self::modify_files_while_compiling(&self.path, None, Self::build_steel_dynlib)?;

        let library: Library = unsafe { Library::new(&lib_path)? };

        let setup_logger_fn: Symbol<
            fn(&'static dyn Log, LevelFilter) -> Result<(), SetLoggerError>,
        > = unsafe { library.get(b"setup_logger")? };
        setup_logger_fn(log::logger(), log::max_level())?;

        let create_app_fn: Symbol<fn() -> Box<dyn App>> = unsafe { library.get(b"create")? };
        let mut app = create_app_fn();
        app.init(InitInfo {
            platform: Platform::new_editor(self.path.clone()),
            context,
            ray_tracing_supported,
            scene: None,
        });

        // init a watcher to monitor file changes for asset system
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |result| match result {
            Ok(event) => {
                if let Err(e) = sender.send(event) {
                    log::error!("Send watch event error: {e}");
                }
            }
            Err(e) => log::error!("Watch error: {e}"),
        })
        .unwrap();
        let abs_asset_dir = self.path.join("asset");
        watcher
            .watch(&abs_asset_dir, RecursiveMode::Recursive)
            .unwrap();

        // process assets
        if let Err(e) = Self::scan_asset_dir(abs_asset_dir, &app) {
            log::warn!("Project::_scan_asset_dir error: {e}");
        }

        // restore game world from scene data, this must be done after scanning
        // asset dir to ensure that all prefabs can be found when loading scene
        app.command(Command::Reload(&scene_data));
        app.command(Command::SetCurrentScene(scene.clone()));

        // create ProjectCompiledState
        self.compiled = Some(ProjectCompiledState {
            app,
            library,
            scene,
            scene_data,
            running: false,
            watcher,
            receiver,
            last_rename_from_event: None,
        });

        // if we are loading from local data
        if let Some(local_scene_camera) = local_scene_camera {
            // load scene camera
            *scene_camera = local_scene_camera;

            // ensure that we load from local data only once
            local_data.scene_data = None;
            local_data.save();
        }

        Ok(())
    }

    /// There is a problem with compilation failure due to the pdb file being locked:
    /// https://developercommunity.visualstudio.com/t/pdb-is-locked-even-after-dll-is-unloaded/690640
    /// We avoid this problem by rename it so that compiler can generate a new pdb file.
    fn handle_pdb_file(pdb_file_regex: Regex) -> Result<(), Box<dyn Error>> {
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
        log::debug!(
            "Rename {} to {}, pdb_files={pdb_files:?}",
            pdb_file.display(),
            to_pdb_file.display()
        );
        if let Err(error) = fs::rename(pdb_file, to_pdb_file) {
            log::warn!("Failed to rename {pdb_file_name}, error={error}");
        }
        Ok(())
    }

    /// A helper function to temporily modify some files before compiling and restore them after compiling.
    /// 1. Modify "steel-project path" of steel-client/Cargo.toml and steel-dynlib/Cargo.toml to project_path.
    /// (Note: we must modify both of them at same time even when only one project is compiling,
    /// because cargo requires that all path of same dependency in a workspace must be same)
    /// 2. Modify "init_scene" of steel-client/src/lib.rs to init_scene, just pass None when not compile steel-client.
    fn modify_files_while_compiling(
        project_path: impl AsRef<Path>,
        init_scene: Option<AssetId>,
        compile_fn: fn() -> Result<(), Box<dyn Error>>,
    ) -> Result<(), Box<dyn Error>> {
        let cargo_toml_paths = [
            PathBuf::from("steel-build/steel-client/Cargo.toml"),
            PathBuf::from("steel-build/steel-dynlib/Cargo.toml"),
        ];
        let steel_client_src_path = PathBuf::from("steel-build/steel-client/src/lib.rs");

        let mut cargo_toml_original_contents = Vec::new();
        for path in &cargo_toml_paths {
            log::debug!("Read {}", path.display());
            let original_content = fs::read_to_string(path)?;
            let num_match = original_content
                .matches(STEEL_PROJECT_PATH)
                .collect::<Vec<_>>()
                .len();
            if num_match != 1 {
                return Err(format!(
                    "Expected only 1 match '{STEEL_PROJECT_PATH}' in {}, \
                    actual number of match: {num_match}",
                    path.display()
                )
                .into());
            }
            cargo_toml_original_contents.push(original_content);
        }
        let steel_client_src_original_content = if init_scene.is_some() {
            log::debug!("Read {}", steel_client_src_path.display());
            let original_content = fs::read_to_string(&steel_client_src_path)?;
            let num_match = original_content
                .matches(INIT_SCENE)
                .collect::<Vec<_>>()
                .len();
            if num_match != 1 {
                return Err(format!(
                    "Expected only 1 match '{INIT_SCENE}' in {}, \
                    actual number of match: {num_match}",
                    steel_client_src_path.display()
                )
                .into());
            }
            Some(original_content)
        } else {
            None
        };

        let open_project_path = project_path
            .as_ref()
            .to_str()
            .ok_or(format!(
                "{} to_str() returns None",
                project_path.as_ref().display()
            ))?
            .replace("\\", "/");
        let cargo_toml_new_contents = cargo_toml_original_contents
            .iter()
            .map(|original_content| {
                original_content.replacen(STEEL_PROJECT_PATH, open_project_path.as_str(), 1)
            })
            .collect::<Vec<_>>();

        let steel_client_src_new_content =
            steel_client_src_original_content
                .as_ref()
                .map(|original_content| {
                    original_content.replacen(INIT_SCENE, &init_scene.unwrap().to_string(), 1)
                });

        let mut num_modified_cargo_toml = cargo_toml_paths.len();
        for i in 0..cargo_toml_paths.len() {
            log::debug!("Modify {}", cargo_toml_paths[i].display());
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
                log::debug!("Modify {}", steel_client_src_path.display());
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
            Err("Not compile due to previous modify error".into())
        };

        for i in 0..num_modified_cargo_toml {
            log::debug!("Restore {}", cargo_toml_paths[i].display());
            if let Err(error) = fs::write(&cargo_toml_paths[i], &cargo_toml_original_contents[i]) {
                log::error!(
                    "There is an error while writing original content back to {}! \
                    You have to restore this file by yourself, error={error}",
                    cargo_toml_paths[i].display()
                );
            }
        }
        if let Some(steel_client_src_original_content) = steel_client_src_original_content {
            log::debug!("Restore {}", steel_client_src_path.display());
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

    fn build_steel_dynlib() -> Result<(), Box<dyn Error>> {
        log::info!("$ cargo build -p steel-dynlib");
        std::process::Command::new("cargo")
            .arg("build")
            .arg("-p")
            .arg("steel-dynlib")
            .spawn()?
            .wait()?; // TODO: non-blocking wait
        Ok(())
    }

    /// Scan asset folder to:
    /// 1. Create ".asset" file for normal asset files.
    /// 2. Delete ".asset" file if its corresponding asset file not exists.
    /// 3. Collect asset info to insert into AssetManager.
    fn scan_asset_dir(
        asset_dir: impl AsRef<Path>,
        app: &Box<dyn App>,
    ) -> Result<(), Box<dyn Error>> {
        Self::scan_asset_dir_recursive(&asset_dir, &asset_dir, app)
    }

    fn scan_asset_dir_recursive(
        asset_dir: impl AsRef<Path>,
        dir: impl AsRef<Path>,
        app: &Box<dyn App>,
    ) -> Result<(), Box<dyn Error>> {
        let dir = dir.as_ref();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                Self::scan_asset_dir_recursive(asset_dir.as_ref(), path, app)?;
            } else if entry.file_type()?.is_file() {
                if path
                    .extension()
                    .is_some_and(|extension| extension == "asset")
                {
                    // path is asset info file
                    let asset_file = AssetInfo::asset_info_path_to_asset_path(&path);
                    if !asset_file.exists() {
                        // corresponding asset file not exists, remove asset info file
                        log::trace!("Project::_scan_asset_dir_recursive: remove independent asset info file: {path:?}");
                        fs::remove_file(path)?;
                    }
                } else {
                    // path is normal asset file
                    let relative_path = path.strip_prefix(&asset_dir)?;
                    // Read/Create AssetInfo and insert into AssetManager
                    Project::get_asset_info_and_insert(&asset_dir, relative_path, app, true)?;
                }
            }
        }
        Ok(())
    }

    fn export_windows(&self) -> Result<(), Box<dyn Error>> {
        let scene = self
            .compiled
            .as_ref()
            .and_then(|compiled| compiled.scene)
            .ok_or("No scene is open, you must open a scene as init scene before export")?;

        Self::export_asset(self.asset_dir(), self.path.join("build/windows/asset"))?;

        let exe_path = PathBuf::from("target/debug/steel-client.exe");
        if exe_path.exists() {
            fs::remove_file(&exe_path)?;
        }
        Self::modify_files_while_compiling(
            &self.path,
            Some(scene),
            Self::build_steel_client_desktop,
        )?;
        if !exe_path.exists() {
            return Err(format!("No output file: {}", exe_path.display()).into());
        }

        let exe_export_path = self.path.join("build/windows/steel-client.exe");
        fs::create_dir_all(exe_export_path.parent().unwrap())?;
        fs::copy(exe_path, &exe_export_path)?;
        log::debug!("Exported: {}", exe_export_path.display());

        Ok(())
    }

    fn export_asset(src: PathBuf, dst: PathBuf) -> std::io::Result<()> {
        if dst.is_dir() {
            fs::remove_dir_all(&dst)?;
        }
        Self::copy_dir_all(src, dst)?;
        Ok(())
    }

    fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
        fs::create_dir_all(&dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                Self::copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
            } else {
                let dst = dst.as_ref().join(entry.file_name());
                fs::copy(entry.path(), &dst)?;
                log::debug!("Exported: {}", dst.display());
            }
        }
        Ok(())
    }

    fn build_steel_client_desktop() -> Result<(), Box<dyn Error>> {
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

    fn export_android(&self) -> Result<(), Box<dyn Error>> {
        let scene = self
            .compiled
            .as_ref()
            .and_then(|compiled| compiled.scene)
            .ok_or("No scene is open, you must open a scene as init scene before export")?;

        Self::export_asset(
            self.asset_dir(),
            PathBuf::from("steel-build/android-project/app/src/main/assets"),
        )?;

        // TODO: run following commands:
        // rustup target add aarch64-linux-android
        // cargo install cargo-ndk

        let so_path =
            PathBuf::from("steel-build/android-project/app/src/main/jniLibs/arm64-v8a/libmain.so");
        if so_path.exists() {
            fs::remove_file(&so_path)?;
        }
        Self::modify_files_while_compiling(
            &self.path,
            Some(scene),
            Self::build_steel_client_android,
        )?;
        if !so_path.exists() {
            return Err(format!("No output file: {}", so_path.display()).into());
        }

        let apk_path =
            PathBuf::from("steel-build/android-project/app/build/outputs/apk/debug/app-debug.apk");
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
            return Err(format!("No output file: {}", apk_path.display()).into());
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

        let apk_export_path = self.path.join("build/android/steel-client.apk");
        fs::create_dir_all(apk_export_path.parent().unwrap())?;
        fs::copy(apk_path, &apk_export_path)?;
        log::debug!("Exported: {}", apk_export_path.display());
        Ok(())
    }

    fn build_steel_client_android() -> Result<(), Box<dyn Error>> {
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
}

/// The state of current compiled project.
struct ProjectCompiledState {
    app: Box<dyn App>,
    #[allow(unused)]
    library: Library, // Library must be destroyed after App

    /// The asset id of current open scene
    scene: Option<AssetId>,
    scene_data: SceneData,
    running: bool,

    /// Watch for file changes in asset folder.
    #[allow(unused)]
    watcher: RecommendedWatcher,
    /// Receiver for file change events from watcher.
    receiver: Receiver<Event>,
    /// Last EventKind::Modify(ModifyKind::Name(RenameMode::From)),
    /// to be used with upcoming EventKind::Modify(ModifyKind::Name(RenameMode::To)).
    last_rename_from_event: Option<Event>,
}

impl ProjectCompiledState {
    fn exit(&self, local_data: &mut LocalData, scene_camera: SceneCamera) {
        assert!(local_data.open_last_project_on_start);
        let scene_data = self.get_scene_data(None);
        local_data.scene_data = Some((scene_camera, self.scene, scene_data));
        local_data.save();
    }

    /// Convert the current scene to a [SceneData] object.
    fn get_scene_data(
        &self,
        prefab_data_override: Option<(EntityId, Arc<PrefabData>)>,
    ) -> SceneData {
        let mut world_data = WorldData::default();
        self.app.command(Command::Save(&mut world_data));
        let get_prefab_data_fn = |prefab_asset: AssetId| {
            let mut prefab_data = None;
            self.app
                .command(Command::GetPrefabData(prefab_asset, &mut prefab_data));
            prefab_data
        };
        let mut scene_data = SceneData::new_with_prefab_data_override(
            &world_data,
            get_prefab_data_fn,
            prefab_data_override,
        );
        scene_data.cut();
        scene_data
    }

    fn save_to_memory(&mut self, prefab_data_override: Option<(EntityId, Arc<PrefabData>)>) {
        self.scene_data = self.get_scene_data(prefab_data_override);
    }

    fn load_from_memory(&self) {
        self.app.command(Command::Reload(&self.scene_data));
        self.app
            .command(Command::SetCurrentScene(self.scene.clone()));
    }

    fn scene_absolute_path(&self, asset_dir: impl AsRef<Path>) -> Option<PathBuf> {
        self.scene.as_ref().and_then(|scene| {
            let mut scene_path = None;
            self.app
                .command(Command::GetAssetPath(*scene, &mut scene_path));
            scene_path.map(|scene_path| asset_dir.as_ref().join(scene_path))
        })
    }

    fn scene_relative_path(&self) -> Option<PathBuf> {
        self.scene.as_ref().and_then(|scene| {
            let mut scene_path = None;
            self.app
                .command(Command::GetAssetPath(*scene, &mut scene_path));
            scene_path
        })
    }

    fn new_scene(&mut self) {
        self.app.command(Command::Reload(&SceneData::default()));
        self.app.command(Command::SetCurrentScene(None));
        self.scene = None;
    }

    fn save_scene(
        &mut self,
        scene: impl AsRef<Path>,
        asset_dir: impl AsRef<Path>,
    ) -> Result<(), Box<dyn Error>> {
        self.scene_data = self.get_scene_data(None);
        let scene_abs = asset_dir.as_ref().join(&scene);
        crate::utils::save_to_file(&self.scene_data, &scene_abs)?;
        let asset_info = Project::get_asset_info_and_insert(asset_dir, scene, &self.app, false)?;
        self.app
            .command(Command::SetCurrentScene(Some(asset_info.id)));
        self.scene = Some(asset_info.id);
        Ok(())
    }

    fn load_scene(
        &mut self,
        scene: impl AsRef<Path>,
        asset_dir: impl AsRef<Path>,
    ) -> Result<(), Box<dyn Error>> {
        let scene_abs = asset_dir.as_ref().join(&scene);
        let scene_data = crate::utils::load_from_file::<SceneData>(&scene_abs)?;
        self.scene_data = scene_data;
        self.app.command(Command::Reload(&self.scene_data));
        let asset_info = Project::get_asset_info_and_insert(asset_dir, scene, &self.app, false)?;
        self.app
            .command(Command::SetCurrentScene(Some(asset_info.id)));
        self.scene = Some(asset_info.id);
        Ok(())
    }
}

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
parry2d = "0.17"
parry3d = "0.17"
rapier2d = { version = "0.22", features = [ "debug-render" ] }
rapier3d = { version = "0.22", features = [ "debug-render" ] }
nalgebra = { version = "0.33", features = [ "convert-glam024" ] }
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

const INIT_SCENE: &'static str = "init_scene";
