pub use steel_common::asset::*;

use shipyard::Unique;
use std::{collections::HashMap, path::PathBuf};
use steel_common::platform::Platform;

/// Asset info.
pub struct Asset {
    path: PathBuf,
    content: Option<Vec<u8>>,
}

impl Asset {
    /// Create a new Asset with path.
    pub fn new(path: PathBuf) -> Self {
        Asset {
            path,
            content: None,
        }
    }
}

/// AssetManager stores all asset info.
#[derive(Unique, Default)]
pub struct AssetManager {
    assets: HashMap<AssetId, Asset>,
}

impl AssetManager {
    /// Get asset content of asset_id as bytes.
    /// This function will read from file for the first time and cache those bytes,
    /// so this function will return immediately for the next time.
    pub fn get_asset_content(
        &mut self,
        asset_id: AssetId,
        platform: &Platform,
    ) -> Option<&Vec<u8>> {
        if let Some(asset) = self.assets.get_mut(&asset_id) {
            if asset.content.is_none() {
                match platform.read_asset(&asset.path) {
                    Ok(asset_content) => asset.content = Some(asset_content),
                    Err(e) => {
                        log::warn!("AssetManager::get_asset_content: failed to read asset: {e:?}");
                        return None;
                    }
                }
            }
            return Some(asset.content.as_ref().unwrap());
        }
        None
    }

    pub fn get_asset_path(&self, asset_id: AssetId) -> Option<&PathBuf> {
        self.assets.get(&asset_id).map(|asset| &asset.path)
    }

    /// Check if asset_id exists.
    pub fn contains_asset(&self, asset_id: AssetId) -> bool {
        self.assets.contains_key(&asset_id)
    }

    /// Insert an asset with asset_id and path.
    pub(crate) fn insert_asset(&mut self, asset_id: AssetId, path: PathBuf) {
        self.assets.insert(asset_id, Asset::new(path));
    }

    /// Delete an asset with asset_id.
    pub(crate) fn delete_asset(&mut self, asset_id: AssetId) {
        self.assets.remove(&asset_id);
    }

    /// Clear an asset cache by setting asset's content to None.
    pub(crate) fn clear_asset_cache(&mut self, asset_id: AssetId) {
        if let Some(asset) = self.assets.get_mut(&asset_id) {
            asset.content = None;
        }
    }

    /// Update path of an asset.
    pub(crate) fn update_asset_path(&mut self, asset_id: AssetId, path: PathBuf) {
        if let Some(asset) = self.assets.get_mut(&asset_id) {
            asset.path = path;
        }
    }
}
