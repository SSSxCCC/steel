use std::{path::PathBuf, process::Command, error::Error};
use steel_common::Engine;
use libloading::{Library, Symbol};
use log::{Log, LevelFilter, SetLoggerError};

struct ProjectCompileData {
    engine: Box<dyn Engine>,
    library: Library, // Library must be destroyed after Engine
}

struct ProjectData {
    path: PathBuf,
    data: Option<ProjectCompileData>,
}

pub struct Project {
    inner: Option<ProjectData>,
}

impl Project {
    pub fn new() -> Self {
        Project { inner: None }
    }

    pub fn open(&mut self, path: PathBuf) {
        self.inner = Some(ProjectData { path, data: None });
    }

    pub fn is_open(&self) -> bool {
        self.inner.is_some()
    }

    pub fn close(&mut self) {
        self.inner = None;
    }

    pub fn compile(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(project) = self.inner.as_mut() {
            let mut complie_process = Command::new("cargo")
                .arg("build")
                .current_dir(&project.path)
                .spawn()?;

            complie_process.wait()?;

            let lib_path = project.path.join("target/debug/steel.dll");
            let library: Library = unsafe { Library::new(&lib_path)? };

            let setup_logger_fn: Symbol<fn(&'static dyn Log, LevelFilter) -> Result<(), SetLoggerError>> = unsafe { library.get(b"setup_logger")? };
            setup_logger_fn(log::logger(), log::max_level())?;

            let create_engine_fn: Symbol<fn() -> Box<dyn Engine>> = unsafe { library.get(b"create")? };
            let mut engine = create_engine_fn();
            engine.init();

            project.data = Some(ProjectCompileData { engine, library });
            Ok(())
        } else {
            Err(Box::new(CompileError { message: "No open project".into() }))
        }
    }

    pub fn engine(&mut self) -> Option<&mut Box<dyn Engine>> {
        Some(&mut self.inner.as_mut()?.data.as_mut()?.engine)
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
