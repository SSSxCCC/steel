//! The [steel game engine](https://github.com/SSSxCCC/steel) core library.

pub mod app;
pub mod asset;
pub mod camera;
pub mod data;
pub mod edit;
pub mod hierarchy;
pub mod input;
pub mod name;
pub mod physics2d;
pub mod platform;
pub mod prefab;
pub mod render;
pub mod scene;
pub mod shape;
pub mod shape2d;
pub mod time;
pub mod transform;
pub mod ui;
pub mod ext {
    pub use steel_common::ext::*;
}

use log::{LevelFilter, Log, SetLoggerError};

/// This function is used by steel-dynlib to enable log output of log crate.
#[no_mangle]
pub fn setup_logger(logger: &'static dyn Log, level: LevelFilter) -> Result<(), SetLoggerError> {
    log::set_max_level(level);
    log::set_logger(logger)
}
