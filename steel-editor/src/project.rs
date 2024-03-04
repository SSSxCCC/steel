use std::{path::PathBuf, process::Command, error::Error, fs};
use steel_common::{Engine, WorldData};
use libloading::{Library, Symbol};
use log::{Log, LevelFilter, SetLoggerError};

struct ProjectCompiledState {
    engine: Box<dyn Engine>,
    library: Library, // Library must be destroyed after Engine

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
        self.state = Some(ProjectState { path, compiled: None });
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

            let mut complie_process = Command::new("cargo")
                .arg("rustc")
                .arg("--crate-type=cdylib")
                .current_dir(&state.path)
                .spawn()?;

            complie_process.wait()?; // TODO: non-blocking wait

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
        if let Some(error) = self._export_windows().err() {
            log::error!("Project export windows failed: {error}");
        }
    }

    fn _export_windows(&self) -> Result<(), Box<dyn Error>> {
        if let Some(state) = self.state.as_ref() {
            let exe_path = PathBuf::from("target/debug/steel-client.exe");
            if exe_path.exists() {
                fs::remove_file(&exe_path)?;
            }

            // cargo build -p steel-client -F desktop
            Command::new("cargo")
                .arg("build")
                .arg("-p").arg("steel-client")
                .arg("-F").arg("desktop")
                .spawn()?
                .wait()?; // TODO: non-blocking wait

            if !exe_path.exists() {
                return Err(Box::new(ProjectError { message: format!("No file: {exe_path:?}") }));
            }

            let exe_export_path = state.path.join("build/windows/steel-client.exe");
            fs::create_dir_all(exe_export_path.parent().unwrap())?;
            fs::copy(exe_path, exe_export_path)?;
            Ok(())
        } else {
            Err(Box::new(ProjectError { message: "No open project".into() }))
        }
    }

    pub fn export_android(&self) {
        if let Some(error) = self._export_android().err() {
            log::error!("Project export android failed: {error}");
        }
    }

    fn _export_android(&self) -> Result<(), Box<dyn Error>> {
        if let Some(state) = self.state.as_ref() {
            let so_path = PathBuf::from("steel-client/android-project/app/src/main/jniLibs/arm64-v8a/libmain.so");
            if so_path.exists() {
                fs::remove_file(&so_path)?;
            }

            // cargo ndk -t arm64-v8a -o steel-client/android-project/app/src/main/jniLibs/ build -p steel-client
            Command::new("cargo")
                .arg("ndk")
                .arg("-t").arg("arm64-v8a")
                .arg("-o").arg("steel-client/android-project/app/src/main/jniLibs/")
                .arg("build")
                .arg("-p").arg("steel-client")
                .spawn()?
                .wait()?; // TODO: non-blocking wait

            if !so_path.exists() {
                return Err(Box::new(ProjectError { message: format!("No file: {so_path:?}") }));
            }

            let apk_path = PathBuf::from("steel-client/android-project/app/build/outputs/apk/debug/app-debug.apk");
            if apk_path.exists() {
                fs::remove_file(&apk_path)?;
            }

            let mut android_project_dir = fs::canonicalize("steel-client/android-project").unwrap();
            // the windows path prefix "\\?\" makes bat fail to run in std::process::Command
            const WINDOWS_PATH_PREFIX: &str = r#"\\?\"#;
            if android_project_dir.display().to_string().starts_with(WINDOWS_PATH_PREFIX) {
                // TODO: convert PathBuf to String and back to PathBuf may lose data, find a better way to do this
                android_project_dir = PathBuf::from(&android_project_dir.display().to_string()[WINDOWS_PATH_PREFIX.len()..]);
            };

            Command::new("steel-client/android-project/gradlew.bat")
                .arg("build")
                .current_dir(&android_project_dir)
                .spawn()?
                .wait()?; // TODO: non-blocking wait

            if !apk_path.exists() {
                return Err(Box::new(ProjectError { message: format!("No file: {apk_path:?}") }));
            }

            Command::new("steel-client/android-project/gradlew.bat")
                .arg("installDebug")
                .current_dir(&android_project_dir)
                .spawn()?
                .wait()?; // TODO: non-blocking wait

            let apk_export_path = state.path.join("build/android/steel-client.apk");
            fs::create_dir_all(apk_export_path.parent().unwrap())?;
            fs::copy(apk_path, apk_export_path)?;
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
