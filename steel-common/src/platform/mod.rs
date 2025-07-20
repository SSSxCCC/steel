#[cfg(not(target_os = "android"))]
mod platform_desktop;
#[cfg(not(target_os = "android"))]
pub use platform_desktop::*;

#[cfg(target_os = "android")]
mod platform_android;
#[cfg(target_os = "android")]
pub use platform_android::*;

/// The build target platform, currently only contains desktop and android.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BuildTarget {
    /// Desktop currently only contains windows platform.
    Desktop,
    /// The android platform.
    Android,
}

/// Current build target platform. Use this to write different logic for different platforms.
#[cfg(not(target_os = "android"))]
pub const BUILD_TARGET: BuildTarget = BuildTarget::Desktop;

/// Current build target platform. Use this to write different logic for different platforms.
#[cfg(target_os = "android")]
pub const BUILD_TARGET: BuildTarget = BuildTarget::Android;
