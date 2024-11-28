use crate::asset::AssetManager;
use image::{DynamicImage, ImageError, ImageReader};
use shipyard::Unique;
use std::{collections::HashMap, io::Cursor, sync::Arc};
use steel_common::{asset::AssetId, platform::Platform};

struct ImageAsset {
    bytes: Arc<Vec<u8>>,
    data: Arc<DynamicImage>,
}

#[derive(Unique, Default)]
/// Cache [DynamicImage] in assets.
pub struct ImageAssets {
    images: HashMap<AssetId, ImageAsset>,
}

impl ImageAssets {
    pub fn get_image(
        &mut self,
        asset_id: AssetId,
        asset_manager: &mut AssetManager,
        platform: &Platform,
    ) -> Option<Arc<DynamicImage>> {
        if let Some(bytes) = asset_manager.get_asset_content(asset_id, platform) {
            if let Some(image_asset) = self.images.get(&asset_id) {
                if Arc::ptr_eq(bytes, &image_asset.bytes) {
                    // cache is still valid
                    return Some(image_asset.data.clone());
                }
            }
            // cache is not valid, reload data
            match Self::get_image_from_bytes(&bytes) {
                Ok(data) => {
                    let image_data = Arc::new(data);
                    self.images.insert(
                        asset_id,
                        ImageAsset {
                            bytes: bytes.clone(),
                            data: image_data.clone(),
                        },
                    );
                    return Some(image_data);
                }
                Err(e) => log::error!("ImageAssets::get_image: error: {}", e),
            }
        }
        self.images.remove(&asset_id);
        None
    }

    fn get_image_from_bytes(bytes: &[u8]) -> Result<DynamicImage, ImageError> {
        ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()?
            .decode()
    }
}
