use std::{error::Error, path::Path, io::Read, ffi::CString};
use winit::platform::android::activity::AndroidApp;
use shipyard::Unique;
use crate::platform::PlatformError;

/// Platform struct stores some platform specific data,
/// and has methods that have different implementations in different platforms.
#[derive(Unique)]
pub struct Platform {
    android_app: AndroidApp,
}

impl Platform {
    pub fn new(android_app: AndroidApp) -> Self {
        Platform { android_app }
    }

    pub fn read_asset_to_string(&self, path: impl AsRef<Path>) -> Result<String, Box<dyn Error>> {
        let path = CString::new(path.as_ref().to_str().unwrap()).unwrap();
        let mut asset = match self.android_app.asset_manager().open(path.as_c_str()) {
            Some(asset) => asset,
            None => return Err(PlatformError::new("AssetManager::open returns None").boxed()),
        };
        let mut buf = String::new();
        asset.read_to_string(&mut buf)?;
        Ok(buf)
    }
}
