//! Provides Android functionality for Steel Engine.

pub use winit::platform::android::activity;

use activity::AndroidApp;

/// [`AndroidApp`] provides an interface to query the application state as well as monitor events
/// (for example lifecycle and input events).
pub static ANDROID_APP: std::sync::OnceLock<AndroidApp> = std::sync::OnceLock::new();
