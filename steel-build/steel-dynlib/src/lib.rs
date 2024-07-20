//! The dynamic library for the [steel game engine](https://github.com/SSSxCCC/steel) to reload at runtime.

// TODO: find out why 'create' funtion must be 'pub use' to be able to be loaded in steel-editor
// while 'setup_logger' function can be loaded in steel-editor without 'pub use'
pub use steel::create;
