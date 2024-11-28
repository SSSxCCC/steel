use crate::asset::AssetManager;
use obj::{Obj, ObjError, TexturedVertex};
use shipyard::Unique;
use std::{collections::HashMap, io::Cursor, sync::Arc};
use steel_common::{asset::AssetId, platform::Platform};

struct ModelAsset {
    bytes: Arc<Vec<u8>>,
    data: Arc<Obj<TexturedVertex>>,
}

#[derive(Unique, Default)]
/// Cache [Obj<TexturedVertex>] in assets.
pub struct ModelAssets {
    models: HashMap<AssetId, ModelAsset>,
}

impl ModelAssets {
    pub fn get_model(
        &mut self,
        asset_id: AssetId,
        asset_manager: &mut AssetManager,
        platform: &Platform,
    ) -> Option<Arc<Obj<TexturedVertex>>> {
        if let Some(bytes) = asset_manager.get_asset_content(asset_id, platform) {
            if let Some(model_asset) = self.models.get(&asset_id) {
                if Arc::ptr_eq(bytes, &model_asset.bytes) {
                    // cache is still valid
                    return Some(model_asset.data.clone());
                }
            }
            // cache is not valid, reload data
            match Self::get_model_from_bytes(&bytes) {
                Ok(data) => {
                    let model_data = Arc::new(data);
                    self.models.insert(
                        asset_id,
                        ModelAsset {
                            bytes: bytes.clone(),
                            data: model_data.clone(),
                        },
                    );
                    return Some(model_data);
                }
                Err(e) => log::error!("ModelAssets::get_model: error: {}", e),
            }
        }
        self.models.remove(&asset_id);
        None
    }

    fn get_model_from_bytes(bytes: &[u8]) -> Result<Obj<TexturedVertex>, ObjError> {
        obj::load_obj(Cursor::new(bytes))
    }
}
