use serde::{Deserialize, Serialize};

/// Every asset has an unique AssetId.
pub type AssetId = u32;

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
}
