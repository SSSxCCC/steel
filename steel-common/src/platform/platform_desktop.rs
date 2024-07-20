use shipyard::Unique;
use std::{
    error::Error,
    path::{Path, PathBuf},
};

/// Platform struct stores some platform specific data,
/// and has methods that have different implementations in different platforms.
#[derive(Unique)]
pub struct Platform {
    project_path: PathBuf,
}

impl Platform {
    pub fn new_editor(project_path: PathBuf) -> Self {
        Platform { project_path }
    }

    /// steel-client can use relative path to access asset folder, so that we set project_path to empty.
    pub fn new_client() -> Self {
        Platform {
            project_path: PathBuf::from(""),
        }
    }

    pub fn read_asset_to_string(&self, path: impl AsRef<Path>) -> Result<String, Box<dyn Error>> {
        Ok(std::fs::read_to_string(
            self.project_path.join("asset").join(path),
        )?)
    }
}
