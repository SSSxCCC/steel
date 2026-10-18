# Changelog

## 0.3.0

### Highlights

- Integrated the editor into Steel applications. Enable the `editor` feature when building a desktop application; there is no standalone editor executable.
- Added ordered system registration with `SteelApp::add_system(schedule, order, system)`.
- Expanded the renderer with 3D models, textures, rasterization improvements, and real-time ray tracing support.
- Added asset ID management and prefab workflows, including nested prefabs.
- Android builds use `steel-client`; enabling the editor on Android now fails at compile time.
- Vendored graphics dependencies to carry the local Vulkano fix for [vulkano-rs/vulkano#2711](https://github.com/vulkano-rs/vulkano/issues/2711).

### Migration notes

- Add an `order` argument to every `SteelApp::add_system` call. Smaller values run earlier within a schedule; systems registered at the same order retain registration order.
- Run editor-enabled games from their application package with `--features steel-engine/editor` (or `--features editor` for the root hello-world example). Do not use the retired standalone `steel-editor` command.
- Rebuild and restart the application after changing Rust source; the editor's Project -> Compile action does not run Cargo.
- Build Android applications with the client and do not enable the editor feature. The included Android wrapper currently targets arm64 devices.
- Review the updated bilingual tutorials for the current application entry point, input handling, scene workflow, and asset directory.
