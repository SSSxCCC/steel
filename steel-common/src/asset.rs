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
}
