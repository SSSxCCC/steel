pub mod engine;
pub mod edit;
pub mod transform;
pub mod camera;
pub mod physics2d;
pub mod render;
pub mod shape;
pub mod entityinfo;
pub mod data;
pub mod scene;
pub mod platform {
    pub use steel_common::platform::*;
}

use log::{Log, LevelFilter, SetLoggerError};

#[no_mangle]
pub fn setup_logger(logger: &'static dyn Log, level: LevelFilter) -> Result<(), SetLoggerError> {
    log::set_max_level(level);
    log::set_logger(logger)
}
