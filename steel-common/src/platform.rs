use std::{error::Error, path::Path};

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

pub struct Platform {
    #[cfg(target_os = "android")] android_app: AndroidApp,
}

impl Platform {
    #[cfg(not(target_os = "android"))]
    pub fn new() -> Self {
        Platform {}
    }

    #[cfg(target_os = "android")]
    pub fn new(android_app: AndroidApp) -> Self {
        Platform { android_app }
    }

    #[cfg(not(target_os = "android"))]
    pub fn read_asset_to_string(&self, path: impl AsRef<Path>) -> Result<String, Box<dyn Error>> {
        use std::path::PathBuf;

        Ok(std::fs::read_to_string(PathBuf::from("asset").join(path))?)
    }

    #[cfg(target_os = "android")]
    pub fn read_asset_to_string(&self, path: impl AsRef<Path>) -> Result<String, Box<dyn Error>> {
        use std::{io::Read, ffi::CString};

        let path = CString::new(path.as_ref().to_str().unwrap()).unwrap();
        let mut asset = match self.android_app.asset_manager().open(path.as_c_str()) {
            Some(asset) => asset,
            None => return Err(Box::new(PlatformError { message: "AssetManager::open returns None".into() })),
        };
        let mut buf = String::new();
        asset.read_to_string(&mut buf)?;
        Ok(buf)
    }
}

#[derive(Debug)]
struct PlatformError {
    message: String,
}

impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PlatformError({})", self.message)
    }
}

impl Error for PlatformError {}
