pub use steel_common::asset::*;

use shipyard::Unique;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use steel_common::platform::Platform;

/// Asset info.
pub struct Asset {
    /// The path is relative to the root asset directory.
    path: PathBuf,
    /// Asset file content in bytes. We cache file content here to avoid reading file more than once.
    content: Option<Arc<Vec<u8>>>,
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
    ) -> Option<&Arc<Vec<u8>>> {
        if let Some(asset) = self.assets.get_mut(&asset_id) {
            if asset.content.is_none() {
                match platform.read_asset(&asset.path) {
                    Ok(asset_content) => asset.content = Some(Arc::new(asset_content)),
                    Err(e) => {
                        log::warn!("AssetManager::get_asset_content: failed to read asset: {e:?}");
                        return None;
                    }
                }
            }
            return asset.content.as_ref();
        }
        None
    }

    /// Get asset path by AssetId. The asset path is relative to the root asset directory.
    pub fn get_asset_path(&self, asset_id: AssetId) -> Option<&PathBuf> {
        self.assets.get(&asset_id).map(|asset| &asset.path)
    }

    /// Check if asset_id exists.
    pub fn contains_asset(&self, asset_id: AssetId) -> bool {
        self.assets.contains_key(&asset_id)
    }

    /// Get AssetId by asset path. The asset path is relative to the root asset directory.
    /// Note that this function has a time complexity of O(n), where n is the total number
    /// of assets. Because assets are stored with AssetId as the key, please use AssetId
    /// to search for assets whenever possible.
    pub fn get_asset_id(&self, asset_path: impl AsRef<Path>) -> Option<AssetId> {
        self.assets.iter().find_map(|(id, asset)| {
            if asset.path == asset_path.as_ref() {
                Some(*id)
            } else {
                None
            }
        })
    }

    /// Insert an asset with asset_id and path. If asset_id exists,
    /// this is equivalent to clear an asset cache by setting asset's content to None.
    pub(crate) fn insert_asset(&mut self, asset_id: AssetId, path: PathBuf) {
        self.assets.insert(asset_id, Asset::new(path));
    }

    /// Delete an asset with asset_id.
    pub(crate) fn delete_asset(&mut self, asset_id: AssetId) {
        self.assets.remove(&asset_id);
    }

    /// Delete all asset in an asset directory. dir is relative to the root asset directory.
    pub(crate) fn delete_asset_dir(&mut self, dir: impl AsRef<Path>) {
        let asset_ids_to_delete = self
            .assets
            .iter()
            .filter_map(|(id, asset)| {
                if asset.path.starts_with(&dir) {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for id in asset_ids_to_delete {
            self.assets.remove(&id);
        }
    }

    /// Update path of an asset.
    pub(crate) fn update_asset_path(&mut self, asset_id: AssetId, path: PathBuf) {
        if let Some(asset) = self.assets.get_mut(&asset_id) {
            asset.path = path;
        }
    }
}
