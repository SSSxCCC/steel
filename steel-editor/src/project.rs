use std::{path::PathBuf, process::Command, error::Error, fs};
use steel_common::{Engine, WorldData};
use libloading::{Library, Symbol};
use log::{Log, LevelFilter, SetLoggerError};

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

    pub fn open(&mut self, path: PathBuf) {
        match Self::_open(&path) {
            Err(error) => log::error!("Failed to open project, path={}, error={error}", path.display()),
            Ok(_) => self.state = Some(ProjectState { path, compiled: None }),
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
        }

        let cargo_toml_file = path.join("Cargo.toml");
        if !cargo_toml_file.exists() {
            let mut steel_engine_dir = fs::canonicalize("steel-engine")?;
            crate::utils::delte_windows_path_prefix(&mut steel_engine_dir);
            let steel_engine_dir = match steel_engine_dir.to_str() {
                Some(s) => s,
                None => return Err(Box::new(ProjectError { message: format!("{steel_engine_dir:?} to_str() returns None") })),
            };
            let steel_engine_dir = steel_engine_dir.replace("\\", "/");
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

        // TODO: git init

        Ok(())
    }

    pub fn is_open(&self) -> bool {
        self.state.is_some()
    }

    pub fn close(&mut self) {
        self.state = None;
    }

    pub fn compile(&mut self) {
        if let Some(error) = self._compile().err() {
            log::error!("Project compile failed: {error}");
        }
    }

    fn _compile(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(state) = self.state.as_mut() {
            state.compiled = None; // prevent steel.dll from being loaded twice at same time
            let lib_path = state.path.join("target/debug/steel.dll");
            if lib_path.exists() {
                fs::remove_file(&lib_path)?;
            }

            log::info!("{}$ cargo rustc --crate-type=cdylib", state.path.display());
            Command::new("cargo")
                .arg("rustc")
                .arg("--crate-type=cdylib")
                .current_dir(&state.path)
                .spawn()?
                .wait()?; // TODO: non-blocking wait

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
            Err(Box::new(ProjectError { message: "No open project".into() }))
        }
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

            log::info!("$ cargo build -p steel-client -F desktop");
            Command::new("cargo")
                .arg("build")
                .arg("-p").arg("steel-client")
                .arg("-F").arg("desktop")
                .spawn()?
                .wait()?; // TODO: non-blocking wait

            if !exe_path.exists() {
                return Err(Box::new(ProjectError { message: format!("No output file: {exe_path:?}") }));
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
            Err(Box::new(ProjectError { message: "No open project".into() }))
        }
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

            log::info!("$ cargo ndk -t arm64-v8a -o steel-client/android-project/app/src/main/jniLibs/ build -p steel-client");
            Command::new("cargo")
                .arg("ndk")
                .arg("-t").arg("arm64-v8a")
                .arg("-o").arg("steel-client/android-project/app/src/main/jniLibs/")
                .arg("build")
                .arg("-p").arg("steel-client")
                .spawn()?
                .wait()?; // TODO: non-blocking wait

            if !so_path.exists() {
                return Err(Box::new(ProjectError { message: format!("No output file: {so_path:?}") }));
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
                return Err(Box::new(ProjectError { message: format!("No output file: {apk_path:?}") }));
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
            log::info!("Exported: {apk_export_path:?}");
            Ok(())
        } else {
            Err(Box::new(ProjectError { message: "No open project".into() }))
        }
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
                if let Some(err) = Self::_save_to_file(&compiled.data, path).err() {
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
serde = { version = "1.0", features = ["derive"] }
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
