use std::{error::Error, fs, path::{Path, PathBuf}, process::Command};
use steel_common::{Engine, WorldData};
use libloading::{Library, Symbol};
use log::{Log, LevelFilter, SetLoggerError};

use crate::utils::LocalData;

struct ProjectCompiledState {
    engine: Box<dyn Engine>,
    #[allow(unused)] library: Library, // Library must be destroyed after Engine

    data: WorldData,
    running: bool,
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
            Err(error) => log::error!("Failed to open project, path={}, error={error}", path.display()),
            Ok(_) => {
                local_data.last_open_project_path = path.clone();
                local_data.save();
                self.state = Some(ProjectState { path, compiled: None });
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
            let mut steel_engine_dir = fs::canonicalize("steel-engine")?;
            crate::utils::delte_windows_path_prefix(&mut steel_engine_dir);
            let steel_engine_dir = steel_engine_dir.to_str()
                .ok_or(ProjectError::new(format!("{} to_str() returns None", steel_engine_dir.display())))?
                .replace("\\", "/");
            fs::write(&cargo_toml_file, CARGO_TOML.replacen("../../steel-engine", steel_engine_dir.as_str(), 1))?;
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

        Ok(())
    }

    fn _init_git(project_path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
        Command::new("git")
            .arg("init")
            .current_dir(project_path)
            .spawn()?
            .wait()?;
        Ok(())
    }

    pub fn is_open(&self) -> bool {
        self.state.is_some()
    }

    pub fn close(&mut self) {
        self.state = None;
    }

    pub fn compile(&mut self) {
        log::info!("Project::compile start");
        match self._compile() {
            Err(error) => log::error!("Project::compile error: {error}"),
            Ok(_) => log::info!("Project::compile end"),
        }
    }

    fn _compile(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(state) = self.state.as_mut() {
            state.compiled = None; // prevent steel.dll from being loaded twice at same time

            let lib_path = PathBuf::from("target/debug/steel.dll");
            if lib_path.exists() {
                fs::remove_file(&lib_path)?;
            }

            // There is a problem with compilation failure due to the pdb file being locked.
            // We avoid this problem by rename it so that compiler can generate a new pdb file.
            let pdb_dir = PathBuf::from("target/debug/deps");
            let pdb_file = pdb_dir.join("steel.pdb");
            if pdb_file.exists() {
                let pdb_files = fs::read_dir(&pdb_dir)?
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| entry.path().is_file())
                    .filter_map(|entry| entry.file_name().into_string().ok())
                    .filter(|file_name| file_name.starts_with("steel") && file_name.ends_with(".pdb"))
                    .filter_map(|file_name| file_name[5..(file_name.len() - 4)].parse::<u32>().ok().map(|n| (file_name, n)))
                    .filter(|(file_name, _)| fs::remove_file(pdb_dir.join(file_name)).is_err())
                    .collect::<Vec<_>>();
                let n = match pdb_files.iter().max_by(|(_, n1), (_, n2)| n1.cmp(n2)) {
                    Some((_, n)) => n + 1,
                    None => 1,
                };
                let to_pdb_file = pdb_dir.join(format!("steel{n}.pdb"));
                log::info!("Rename {} to {}, pdb_files={pdb_files:?}", pdb_file.display(), to_pdb_file.display());
                if let Err(error) = fs::rename(pdb_file, to_pdb_file) {
                    log::warn!("Failed to rename steel.pdb, error={error}");
                }
            }

            Self::_modify_cargo_toml_while_compiling(&state.path, Self::_build_steel_dynlib)?;

            let library: Library = unsafe { Library::new(&lib_path)? };

            let setup_logger_fn: Symbol<fn(&'static dyn Log, LevelFilter) -> Result<(), SetLoggerError>> = unsafe { library.get(b"setup_logger")? };
            setup_logger_fn(log::logger(), log::max_level())?;

            let create_engine_fn: Symbol<fn() -> Box<dyn Engine>> = unsafe { library.get(b"create")? };
            let mut engine = create_engine_fn();

            let data = Self::_load_from_file(state.path.join("scene.json"));
            match &data {
                Ok(_) => log::debug!("Loaded WorldData from scene.json"),
                Err(err) => log::debug!("Failed to load WorldData from scene.json because {err}"),
            }
            engine.init(data.as_ref().ok());
            let data = engine.save();

            state.compiled = Some(ProjectCompiledState { engine, library, data, running: false });
            Ok(())
        } else {
            Err(ProjectError::new("No open project").boxed())
        }
    }

    fn _build_steel_dynlib() -> Result<(), Box<dyn Error>> {
        log::info!("$ cargo build -p steel-dynlib");
        Command::new("cargo")
            .arg("build")
            .arg("-p").arg("steel-dynlib")
            .spawn()?
            .wait()?; // TODO: non-blocking wait
        Ok(())
    }

    fn _modify_cargo_toml_while_compiling(project_path: impl AsRef<Path>,
            compile_fn: fn() -> Result<(), Box<dyn Error>>) -> Result<(), Box<dyn Error>> {
        let cargo_toml_paths = [PathBuf::from("steel-client/Cargo.toml"), PathBuf::from("steel-dynlib/Cargo.toml")];

        let mut original_contents = Vec::new();
        for path in &cargo_toml_paths {
            log::info!("Read {}", path.display());
            let original_content = fs::read_to_string(path)?;
            let num_match = original_content.matches(TEST_PROJECT_PATH).collect::<Vec<_>>().len();
            if num_match != 1 {
                return Err(ProjectError::new(format!("Expected only 1 match '{TEST_PROJECT_PATH}' in {}, \
                    actual number of match: {num_match}", path.display())).boxed());
            }
            original_contents.push(original_content);
        }

        let open_project_path = project_path.as_ref().to_str()
            .ok_or(ProjectError::new(format!("{} to_str() returns None", project_path.as_ref().display())))?
            .replace("\\", "/");

        let new_contents = original_contents.iter()
            .map(|original_content| original_content.replacen(TEST_PROJECT_PATH, open_project_path.as_str(), 1))
            .collect::<Vec<_>>();

        let mut num_modified = cargo_toml_paths.len();
        for i in 0..cargo_toml_paths.len() {
            log::info!("Modify {}", cargo_toml_paths[i].display());
            if let Err(error) = fs::write(&cargo_toml_paths[i], &new_contents[i]) {
                log::error!("Failed to modify {}, error={error}", cargo_toml_paths[i].display());
                num_modified = i;
                break;
            }
        }
        let modify_success = num_modified == cargo_toml_paths.len();

        let compile_result = if modify_success { compile_fn() } else {
            Err(ProjectError::new("Not compile due to previous modify error").boxed())
        };

        for i in 0..num_modified {
            log::info!("Restore {}", cargo_toml_paths[i].display());
            if let Err(error) = fs::write(&cargo_toml_paths[i], &original_contents[i]) {
                log::error!("There is an error while writing original content back to {}! \
                    You have to restore this file by yourself, error={error}", cargo_toml_paths[i].display());
            }
        }

        compile_result
    }

    pub fn is_compiled(&self) -> bool {
        return self.compiled_ref().is_some();
    }

    pub fn engine(&mut self) -> Option<&mut Box<dyn Engine>> {
        Some(&mut self.compiled_mut()?.engine)
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
            let exe_path = PathBuf::from("target/debug/steel-client.exe");
            if exe_path.exists() {
                fs::remove_file(&exe_path)?;
            }

            Self::_modify_cargo_toml_while_compiling(&state.path, Self::_build_steel_client_desktop)?;

            if !exe_path.exists() {
                return Err(ProjectError::new(format!("No output file: {}", exe_path.display())).boxed());
            }

            let exe_export_path = state.path.join("build/windows/steel-client.exe");
            fs::create_dir_all(exe_export_path.parent().unwrap())?;
            fs::copy(exe_path, &exe_export_path)?;
            log::info!("Exported: {}", exe_export_path.display());

            let scene_export_path = state.path.join("build/windows/scene.json");
            if scene_export_path.exists() {
                fs::remove_file(&scene_export_path)?;
            }
            let scene_path = state.path.join("scene.json");
            if scene_path.exists() {
                fs::copy(scene_path, &scene_export_path)?;
                log::info!("Exported: {}", scene_export_path.display());
            }

            Ok(())
        } else {
            Err(ProjectError::new("No open project").boxed())
        }
    }

    fn _build_steel_client_desktop() -> Result<(), Box<dyn Error>> {
        log::info!("$ cargo build -p steel-client -F desktop");
        Command::new("cargo")
            .arg("build")
            .arg("-p").arg("steel-client")
            .arg("-F").arg("desktop")
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
            // TODO: run following commands:
            // rustup target add aarch64-linux-android
            // cargo install cargo-ndk

            let so_path = PathBuf::from("steel-client/android-project/app/src/main/jniLibs/arm64-v8a/libmain.so");
            if so_path.exists() {
                fs::remove_file(&so_path)?;
            }

            Self::_modify_cargo_toml_while_compiling(&state.path, Self::_build_steel_client_android)?;

            if !so_path.exists() {
                return Err(ProjectError::new(format!("No output file: {}", so_path.display())).boxed());
            }

            let assets_dir = PathBuf::from("steel-client/android-project/app/src/main/assets");
            if assets_dir.exists() {
                fs::remove_dir_all(&assets_dir)?;
            }
            fs::create_dir(&assets_dir)?;

            let scene_export_path = assets_dir.join("scene.json");
            let scene_path = state.path.join("scene.json");
            if scene_path.exists() {
                fs::copy(scene_path, &scene_export_path)?;
                log::info!("Exported: {}", scene_export_path.display());
            }

            let apk_path = PathBuf::from("steel-client/android-project/app/build/outputs/apk/debug/app-debug.apk");
            if apk_path.exists() {
                fs::remove_file(&apk_path)?;
            }

            let mut android_project_dir = fs::canonicalize("steel-client/android-project").unwrap();
            // the windows path prefix "\\?\" makes bat fail to run in std::process::Command
            crate::utils::delte_windows_path_prefix(&mut android_project_dir);

            log::info!("{}$ ./gradlew.bat build", android_project_dir.display());
            Command::new("steel-client/android-project/gradlew.bat")
                .arg("build")
                .current_dir(&android_project_dir)
                .spawn()?
                .wait()?; // TODO: non-blocking wait

            if !apk_path.exists() {
                return Err(ProjectError::new(format!("No output file: {}", apk_path.display())).boxed());
            }

            // TODO: not run installDebug if no android device connected
            log::info!("{}$ ./gradlew.bat installDebug", android_project_dir.display());
            Command::new("steel-client/android-project/gradlew.bat")
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
        log::info!("$ cargo ndk -t arm64-v8a -o steel-client/android-project/app/src/main/jniLibs/ build -p steel-client");
        Command::new("cargo")
            .arg("ndk")
            .arg("-t").arg("arm64-v8a")
            .arg("-o").arg("steel-client/android-project/app/src/main/jniLibs/")
            .arg("build")
            .arg("-p").arg("steel-client")
            .spawn()?
            .wait()?; // TODO: non-blocking wait
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
            compiled.data = compiled.engine.save();
        }
    }

    pub fn load_from_memory(&mut self) {
        if let Some(compiled) = self.compiled_mut() {
            compiled.engine.reload(&compiled.data);
        }
    }

    pub fn save_to_file(&mut self) {
        if let Some(state) = &mut self.state {
            let path = state.path.join("scene.json");
            if let Some(compiled) = &mut state.compiled {
                compiled.data = compiled.engine.save();
                if let Err(err) = Self::_save_to_file(&compiled.data, path) {
                    log::warn!("Failed to save WorldData to scene.json because {err}");
                }
            }
        }
    }

    fn _save_to_file(data: &WorldData, path: PathBuf) -> Result<(), Box<dyn Error>> {
        let s = serde_json::to_string_pretty(data)?;
        fs::write(path, s)?;
        Ok(())
    }

    pub fn load_from_file(&mut self) {
        if let Some(state) = &mut self.state {
            let path = state.path.join("scene.json");
            if let Some(compiled) = &mut state.compiled {
                match Self::_load_from_file(path) {
                    Ok(data) => {
                        compiled.data = data;
                        compiled.engine.reload(&compiled.data);
                    }
                    Err(err) => log::warn!("Failed to load WorldData from scene.json because {err}"),
                }
            }
        }
    }

    fn _load_from_file(path: PathBuf) -> Result<WorldData, Box<dyn Error>> {
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
        ProjectError { message: message.into() }
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

const GITIGNORE: &'static str =
"/target
/build
";

const CARGO_TOML: &'static str =
r#"[package]
name = "steel-project"
version = "0.1.0"
edition = "2021"

[lib]
name = "steel"

[dependencies]
steel = { path = "../../steel-engine" }

vulkano = "0.33.0"
vulkano-shaders = "0.33.0"
vulkano-win = "0.33.0"
vulkano-util = "0.33.0"
log = "0.4"
winit = { version = "0.28.6", features = [ "android-game-activity" ] }
winit_input_helper = "0.14.1"
shipyard = { version = "0.6.2", features = [ "serde1" ] }
rayon = "1.8.0"
rapier2d = { version = "0.17.2", features = [ "debug-render" ] }
glam = { version = "0.24.2", features = [ "serde" ] }
egui_winit_vulkano = "0.25.0"
egui = "0.22.0"
egui_demo_lib = "0.22.0"
serde = { version = "1.0", features = [ "derive" ] }
serde_json = "1.0"
indexmap = { version = "2.2.2", features = [ "serde" ] }

[target.'cfg(not(target_os = "android"))'.dependencies]
env_logger = "0.10.0"

[target.'cfg(target_os = "android")'.dependencies]
android_logger = "0.13.3"
"#;

const LIB_RS: &'static str =
"use steel::{Engine, engine::EngineImpl};

#[no_mangle]
pub fn create() -> Box<dyn Engine> {
    Box::new(EngineImpl::new())
}
";

const TEST_PROJECT_PATH: &'static str = "../examples/test-project";
