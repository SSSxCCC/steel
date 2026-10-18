# Publishing Steel 0.3.0

Publish crates in dependency order. The four graphics packages must be available before Steel crates can resolve their registry dependencies.

## Before publishing

1. Confirm the proposed crates.io package names are available: `steel-vulkano-macros`, `steel-vulkano`, `steel-vulkano-util`, and `steel-egui-winit-vulkano`. Crates.io package names cannot be changed after publication.
2. Run `cargo check --workspace` and the desktop and Android checks described in the README.
3. Check package contents with `cargo package --list` and perform a `cargo publish --dry-run` for each package after its registry dependencies have been published.
4. Verify the generated documentation and the README examples for each package.

The editor discovers common CJK font families installed on the host. Users need a system CJK font for Chinese text; if none is found, the editor logs a warning and uses egui's default fonts.

## Publish order

Publish each package and wait for crates.io to index it before moving to the next dependent group:

1. `steel-vulkano-macros` 0.35.0:
   `cargo publish --manifest-path dependencies/vulkano-macros/Cargo.toml`
2. `steel-vulkano` 0.35.2:
   `cargo publish --manifest-path dependencies/vulkano/Cargo.toml`
3. `steel-vulkano-util` 0.35.0:
   `cargo publish --manifest-path dependencies/vulkano-util/Cargo.toml`
4. `steel-egui-winit-vulkano` 0.28.0:
   `cargo publish --manifest-path dependencies/egui_winit_vulkano/Cargo.toml`
5. `steel-common` and `steel-proc` 0.3.0:
   `cargo publish -p steel-common` and `cargo publish -p steel-proc`
6. `steel-client` and `steel-editor` 0.3.0:
   `cargo publish -p steel-client` and `cargo publish -p steel-editor`
7. `steel-internal` 0.3.0:
   `cargo publish -p steel-internal`
8. `steel-engine` 0.3.0:
   `cargo publish -p steel-engine`

`steel-server`, `steel-shader`, and the example applications are marked `publish = false`; they are not part of this release.

## Release notes

See [CHANGELOG.md](./CHANGELOG.md) for the 0.3.0 highlights and migration notes.
