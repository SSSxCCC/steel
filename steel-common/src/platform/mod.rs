#[cfg(all(not(target_os = "android"), not(feature = "editor")))] mod platform_desktop;
#[cfg(all(not(target_os = "android"), not(feature = "editor")))] pub use platform_desktop::*;

#[cfg(target_os = "android")] mod platform_android;
#[cfg(target_os = "android")] pub use platform_android::*;

#[cfg(feature = "editor")] mod platform_editor;
#[cfg(feature = "editor")] pub use platform_editor::*;

// TODO: add platform_editor

#[derive(Debug)]
struct PlatformError {
    message: String,
}

impl PlatformError {
    #[allow(unused)]
    fn new(message: impl Into<String>) -> PlatformError {
        PlatformError { message: message.into() }
    }

    #[allow(unused)]
    fn boxed(self) -> Box<dyn std::error::Error> {
        Box::new(self)
    }
}

impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PlatformError({})", self.message)
    }
}

impl std::error::Error for PlatformError {}
