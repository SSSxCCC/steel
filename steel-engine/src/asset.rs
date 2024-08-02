use shipyard::Unique;
use std::{collections::HashMap, path::PathBuf};
use steel_common::platform::Platform;

/// Every asset has an unique AssetId.
pub type AssetId = u32;

pub struct Asset {
    path: PathBuf,
    content: Option<Vec<u8>>,
}

#[derive(Unique, Default)]
pub struct AssetManager {
    assets: HashMap<AssetId, Asset>,
}

impl AssetManager {
    pub fn get_asset_content(&mut self, asset_id: AssetId, platform: Platform) -> Option<&Vec<u8>> {
        if let Some(asset) = self.assets.get_mut(&asset_id) {
            if asset.content.is_none() {
                match platform.read_asset(asset.path.clone()) {
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

    pub fn contains_asset(&self, asset_id: AssetId) -> bool {
        self.assets.contains_key(&asset_id)
    }
}
