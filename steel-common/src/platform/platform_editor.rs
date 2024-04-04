use std::{error::Error, path::{Path, PathBuf}};

pub struct Platform {
    pub project_path: PathBuf,
}

impl Platform {
    pub fn new(project_path: PathBuf) -> Self {
        Platform { project_path }
    }

    pub fn read_asset_to_string(&self, path: impl AsRef<Path>) -> Result<String, Box<dyn Error>> {
        Ok(std::fs::read_to_string(self.project_path.join("asset").join(path))?)
    }
}
