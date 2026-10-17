use crate::asset::ASSET_DIR;
use shipyard::Unique;
use std::{
    error::Error,
    path::{Path, PathBuf},
};

/// Platform struct stores some platform specific data,
/// and has methods that have different implementations in different platforms.
#[derive(Unique)]
pub struct Platform;

impl Platform {
    /// Used by steel-client and steel-editor in desktop build.
    pub fn new() -> Self {
        Platform
    }

    /// Read an asset file to string, path is relative to the root asset directory.
    pub fn read_asset_to_string(&self, path: impl AsRef<Path>) -> Result<String, Box<dyn Error>> {
        Ok(std::fs::read_to_string(Self::asset_dir().join(path))?)
    }

    /// Read an asset file to bytes, path is relative to the root asset directory.
    pub fn read_asset(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(std::fs::read(Self::asset_dir().join(path))?)
    }

    /// List all files in asset directory.
    pub fn list_asset_files(&self) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        let mut out_vec = Vec::new();
        let asset_dir = Self::asset_dir();
        if asset_dir.is_dir() {
            self.list_asset_files_recursive(asset_dir, &mut out_vec)?;
        }
        Ok(out_vec)
    }

    fn list_asset_files_recursive(
        &self,
        dir: PathBuf,
        out_vec: &mut Vec<PathBuf>,
    ) -> Result<(), Box<dyn Error>> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                self.list_asset_files_recursive(entry.path(), out_vec)?;
            } else if entry.file_type()?.is_file() {
                let relative_path = entry.path().strip_prefix(Self::asset_dir())?.to_path_buf();
                out_vec.push(relative_path);
            }
        }
        Ok(())
    }

    fn asset_dir() -> PathBuf {
        ASSET_DIR.into()
    }
}
