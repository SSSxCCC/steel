#[cfg(not(target_os = "android"))]
mod platform_desktop;
#[cfg(not(target_os = "android"))]
pub use platform_desktop::*;

#[cfg(target_os = "android")]
mod platform_android;
#[cfg(target_os = "android")]
pub use platform_android::*;
