//! The [steel game engine](https://github.com/SSSxCCC/steel) core library.

pub mod app;
pub mod asset;
pub mod camera;
pub mod data;
pub mod edit;
pub mod hierarchy;
pub mod name;
pub mod physics2d;
pub mod prefab;
pub mod render;
pub mod scene;
pub mod shape2d;
pub mod shape3d;
pub mod time;
pub mod transform;
pub mod ui;
pub mod platform {
    pub use steel_common::platform::*;
}
pub use steel_proc::steel_main;

// Re-export third-party crates for easier access
pub use egui;
pub use glam;
pub use log;
