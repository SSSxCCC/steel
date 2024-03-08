use std::{error::Error, fs::File, io::{BufReader, BufWriter}, path::PathBuf};
use serde::{Deserialize, Serialize};

pub fn delte_windows_path_prefix(path: &mut PathBuf) {
    const WINDOWS_PATH_PREFIX: &str = r#"\\?\"#;
    let path_string = path.display().to_string();
    if path_string.starts_with(WINDOWS_PATH_PREFIX) {
        // TODO: convert PathBuf to String and back to PathBuf may lose data, find a better way to do this
        *path = PathBuf::from(&path_string[WINDOWS_PATH_PREFIX.len()..]);
    };
}

/// Some data stores in local machine and can be used in the next time steel-editor is run
#[derive(Serialize, Deserialize)]
pub struct LocalData {
    pub last_open_project_path: PathBuf,
}

impl LocalData {
    const PATH: &'static str = "local_data.json";

    pub fn load() -> Self {
        match Self::_load() {
            Ok(local_data) => local_data,
            Err(error) => {
                log::warn!("Failed to load LocalData, error={error}");

                let mut last_open_project_path = std::fs::canonicalize("examples/test-project").unwrap();
                // the windows path prefix "\\?\" makes cargo build fail in std::process::Command
                delte_windows_path_prefix(&mut last_open_project_path);

                LocalData { last_open_project_path }
            }
        }
    }

    fn _load() -> Result<Self, Box<dyn Error>> {
        let file = File::open(Self::PATH)?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }

    pub fn save(&self) {
        if let Some(error) = self._save().err() {
            log::warn!("Failed to save LocalData, error={error}");
        }
    }

    fn _save(&self) -> Result<(), Box<dyn Error>> {
        let file = File::create(Self::PATH)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)?;
        Ok(())
    }
}
