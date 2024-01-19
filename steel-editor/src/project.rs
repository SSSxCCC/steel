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
            log::error!("Project compile failed: {}", error);
        }
    }

    fn _compile(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(state) = self.state.as_mut() {
            state.compiled = None; // prevent steel.dll from being loaded twice at same time
            let lib_path = state.path.join("target/debug/steel.dll");
            if lib_path.exists() {
                std::fs::remove_file(&lib_path)?;
            }

            let mut complie_process = Command::new("cargo")
                .arg("build")
                .current_dir(&state.path)
                .spawn()?;

            complie_process.wait()?;

            let library: Library = unsafe { Library::new(&lib_path)? };

            let setup_logger_fn: Symbol<fn(&'static dyn Log, LevelFilter) -> Result<(), SetLoggerError>> = unsafe { library.get(b"setup_logger")? };
            setup_logger_fn(log::logger(), log::max_level())?;

            let create_engine_fn: Symbol<fn() -> Box<dyn Engine>> = unsafe { library.get(b"create")? };
            let mut engine = create_engine_fn();

            let data = Self::read_world_data(state.path.join("scene.json"));
            match &data {
                Ok(_) => log::debug!("Loaded WorldData from scene.json"),
                Err(e) => log::debug!("Failed to load WorldData from scene.json because {e}"),
            }
            engine.init(data.as_ref().ok());
            let data = engine.save();

            state.compiled = Some(ProjectCompiledState { engine, library, data, running: false });
            Ok(())
        } else {
            Err(Box::new(CompileError { message: "No open project".into() }))
        }
    }

    pub fn is_compiled(&self) -> bool {
        return self.compiled_ref().is_some();
    }

    fn read_world_data(path: PathBuf) -> Result<WorldData, Box<dyn Error>> {
        let s = fs::read_to_string(path)?;
        Ok(serde_json::from_str::<WorldData>(&s)?)
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

    pub fn set_running(&mut self, running: bool) {
        if let Some(compiled) = self.compiled_mut() {
            compiled.running = running;
        }
    }

    pub fn is_running(&self) -> bool {
        self.compiled_ref().is_some_and(|compiled| compiled.running)
    }

    pub fn save(&mut self) {
        if let Some(compiled) = self.compiled_mut() {
            compiled.data = compiled.engine.save();
        }
    }

    pub fn load(&mut self) {
        if let Some(compiled) = self.compiled_mut() {
            compiled.engine.load(&compiled.data);
        }
    }
}

#[derive(Debug)]
struct CompileError {
    message: String,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CompileError({})", self.message)
    }
}

impl Error for CompileError {

}
