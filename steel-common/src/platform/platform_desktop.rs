use std::{error::Error, path::{Path, PathBuf}};

pub struct Platform {
}

impl Platform {
    pub fn new() -> Self {
        Platform {}
    }

    pub fn read_asset_to_string(&self, path: impl AsRef<Path>) -> Result<String, Box<dyn Error>> {
        Ok(std::fs::read_to_string(PathBuf::from("asset").join(path))?)
    }
}
