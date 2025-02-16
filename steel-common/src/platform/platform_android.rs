use jni::{objects::JObject, JavaVM};
use ndk::asset::Asset;
use shipyard::Unique;
use std::{
    error::Error,
    ffi::CString,
    io::Read,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};
use winit::platform::android::activity::AndroidApp;

/// Platform struct stores some platform specific data,
/// and has methods that have different implementations in different platforms.
#[derive(Unique)]
pub struct Platform {
    android_app: AndroidApp,
}

impl Platform {
    /// Used by steel-client in android build.
    pub fn new(android_app: AndroidApp) -> Self {
        Platform { android_app }
    }

    /// Read an asset file to string, path is relative to the root asset directory.
    pub fn read_asset_to_string(&self, path: impl AsRef<Path>) -> Result<String, Box<dyn Error>> {
        let mut asset = self.get_asset(path)?;
        let mut buf = String::new();
        asset.read_to_string(&mut buf)?;
        Ok(buf)
    }

    /// Read an asset file to bytes, path is relative to the root asset directory.
    pub fn read_asset(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut asset = self.get_asset(path)?;
        let mut buf = vec![0; asset.get_length()];
        asset.read(&mut buf)?;
        Ok(buf)
    }

    fn get_asset(&self, path: impl AsRef<Path>) -> Result<Asset, Box<dyn Error>> {
        let path = CString::new(path.as_ref().as_os_str().as_bytes()).unwrap();
        self.android_app
            .asset_manager()
            .open(path.as_c_str())
            .ok_or_else(|| "AssetManager::open returns None".into())
    }

    /// List all files in asset directory.
    pub fn list_asset_files(&self) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        // android cannot use ndk to list all files in asset directory,
        // see https://partnerissuetracker.corp.google.com/issues/140538113
        // so we use java/kotlin to list all files.
        let vm = unsafe { JavaVM::from_raw(self.android_app.vm_as_ptr() as *mut _) }?;
        let mut env = vm.get_env()?;
        let activity = unsafe { JObject::from_raw(self.android_app.activity_as_ptr() as *mut _) };
        let ret = match env.call_method(
            activity,
            "listAssetFiles",
            "()Ljava/util/List;",
            &Vec::new(),
        )? {
            jni::objects::JValueGen::Object(o) => o,
            _ => return Err("listAssetFiles did not return JObject!".into()),
        };
        let list = env.get_list(&ret)?;
        let mut files = Vec::new();
        let mut iterator = list.iter(&mut env)?;
        while let Some(obj) = iterator.next(&mut env)? {
            let file: String = env.get_string(&obj.into())?.into();
            files.push(file.into());
        }
        Ok(files)
    }
}
