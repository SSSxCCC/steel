use std::{path::PathBuf, process::Command};
use steel_common::Engine;
use libloading::{Library, Symbol};
use log::{Log, LevelFilter, SetLoggerError};

struct ProjectData {
    engine: Box<dyn Engine>,
    library: Library, // Library must be destroyed after Engine
}

pub struct Project {
    path: PathBuf,
    data: Option<ProjectData>,
}

impl Project {
    pub fn new(path: PathBuf) -> Self {
        Project { path, data: None }
    }

    pub fn compile(&mut self) {
        let mut complie_process = Command::new("cargo")
            .arg("build")
            .current_dir(&self.path)
            .spawn()
            .unwrap();

        complie_process.wait().unwrap();

        let lib_path = self.path.join("target/debug/steel.dll");
        let library: Library = unsafe { Library::new(&lib_path) }.unwrap();

        let setup_logger_fn: Symbol<fn(&'static dyn Log, LevelFilter) -> Result<(), SetLoggerError>> = unsafe { library.get(b"setup_logger") }.unwrap();
        setup_logger_fn(log::logger(), log::max_level()).unwrap();

        let create_engine_fn: Symbol<fn() -> Box<dyn Engine>> = unsafe { library.get(b"create") }.unwrap();
        let mut engine = create_engine_fn();
        engine.init();

        self.data = Some(ProjectData { engine, library });
    }

    pub fn engine(&mut self) -> Option<&mut Box<dyn Engine>> {
        self.data.as_mut().and_then(|data| { Some(&mut data.engine) })
    }
}