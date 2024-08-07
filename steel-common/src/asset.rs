use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// The inner type of AssetId.
pub type AssetIdType = u32;

/// Every asset has an unique AssetId.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct AssetId(AssetIdType);

impl std::ops::Deref for AssetId {
    type Target = AssetIdType;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for AssetId {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AssetId {
    /// The always invalid asset id.
    pub const INVALID: AssetId = AssetId(0);

    /// Create an new AssetId with id.
    pub fn new(id: AssetIdType) -> Self {
        AssetId(id)
    }

    /// The value of inner type.
    pub fn value(&self) -> AssetIdType {
        self.0
    }
}

impl Default for AssetId {
    fn default() -> Self {
        AssetId::INVALID
    }
}

/// The info of an asset.
#[derive(Serialize, Deserialize)]
pub struct AssetInfo {
    pub id: AssetId,
}

impl AssetInfo {
    /// Create a new AssetInfo with asset_id.
    pub fn new(asset_id: AssetId) -> Self {
        AssetInfo { id: asset_id }
    }

    /// Helper function to get the corresponding asset file path from an asset info file path.
    /// # Example
    /// ```rust
    /// let asset_file = PathBuf::from("texts/test.txt");
    /// let asset_info_file = PathBuf::from("texts/test.txt.asset");
    /// assert_eq!(asset_file, AssetInfo::asset_info_path_to_asset_path(asset_info_file));
    /// ```
    pub fn asset_info_path_to_asset_path(asset_info_file: impl AsRef<Path>) -> PathBuf {
        let asset_info_file_name = asset_info_file
            .as_ref()
            .file_name()
            .unwrap()
            .to_string_lossy(); // TODO: not convert OsStr to str
        let asset_file_name = &asset_info_file_name[0..asset_info_file_name.len() - ".asset".len()];
        asset_info_file
            .as_ref()
            .parent()
            .unwrap()
            .join(asset_file_name)
    }

    /// Helper function to get the corresponding asset info file path from an asset file path.
    /// # Example
    /// ```rust
    /// let asset_file = PathBuf::from("texts/test.txt");
    /// let asset_info_file = PathBuf::from("texts/test.txt.asset");
    /// assert_eq!(AssetInfo::asset_path_to_asset_info_path(asset_file), asset_info_file);
    /// ```
    pub fn asset_path_to_asset_info_path(asset_file: impl AsRef<Path>) -> PathBuf {
        let mut asset_info_file_name = asset_file.as_ref().file_name().unwrap().to_os_string();
        asset_info_file_name.push(".asset");
        asset_file
            .as_ref()
            .parent()
            .unwrap()
            .join(asset_info_file_name)
    }
}
