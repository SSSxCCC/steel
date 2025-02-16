use crate::locale::Language;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};
use steel_common::{asset::AssetId, camera::SceneCamera, data::SceneData};

/// Delete windows path prefix:
/// ```
/// \\?\
/// ```
/// This windows path prefix makes cargo build fail in std::process::Command.
pub fn delte_windows_path_prefix(path: &mut PathBuf) {
    const WINDOWS_PATH_PREFIX: &str = r#"\\?\"#;
    let path_string = path.display().to_string();
    if path_string.starts_with(WINDOWS_PATH_PREFIX) {
        // TODO: convert PathBuf to String and back to PathBuf may lose data, find a better way to do this
        *path = PathBuf::from(&path_string[WINDOWS_PATH_PREFIX.len()..]);
    };
}

/// Serialize to json and save to file in path.
pub fn save_to_file(object: &impl Serialize, path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    Ok(serde_json::to_writer_pretty(writer, object)?)
}

/// Load from json file in path.
pub fn load_from_file<T: for<'de> Deserialize<'de>>(
    path: impl AsRef<Path>,
) -> Result<T, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(serde_json::from_reader(reader)?)
}

/// Some data stores in local machine and can be used in the next time steel-editor run.
#[derive(Serialize, Deserialize)]
pub struct LocalData {
    pub language: Option<Language>,
    pub last_open_project_path: PathBuf,
    pub open_last_project_on_start: bool,
    pub scene_data: Option<(SceneCamera, Option<AssetId>, SceneData)>,
}

impl LocalData {
    const PATH: &'static str = "local-data.json";

    pub fn load() -> Self {
        match load_from_file(Self::PATH) {
            Ok(local_data) => local_data,
            Err(error) => {
                log::warn!("Failed to load LocalData, error={error}");

                let mut last_open_project_path =
                    std::fs::canonicalize("examples/demo").unwrap_or_default();
                delte_windows_path_prefix(&mut last_open_project_path);

                LocalData {
                    language: None,
                    last_open_project_path,
                    open_last_project_on_start: false,
                    scene_data: None,
                }
            }
        }
    }

    pub fn save(&self) {
        if let Some(error) = save_to_file(self, Self::PATH).err() {
            log::warn!("Failed to save LocalData, error={error}");
        }
    }
}
