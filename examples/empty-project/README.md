# empty-project example

`empty-project` is a starter Steel application. The editor is integrated into the application and is supported on desktop only; Android builds must use the game client.

## Make this your own project

This example is a workspace member and uses a local path dependency so it can be built with the Steel source tree. After copying it into a standalone repository, remove its `steel-engine` line from `[dependencies]`, then add the latest published Steel release:

```
cargo add steel-engine
```

Rename `package.name` and `[[bin]].name` in `Cargo.toml` to your project name. Keep the library name `main` if you plan to use the included Android application template. Commit the generated `Cargo.lock` with your project.

`cargo add steel-engine` selects the latest stable release when creating the project. Existing projects do not automatically cross incompatible 0.x versions: for example, move from 0.3 to 0.4 with `cargo add steel-engine@0.4`, then run `cargo update -p steel-engine`.

## Run in desktop

In current directory:

```
cargo run
```

## Run with the editor (desktop only)

From this directory:

```
cargo run --features steel-engine/editor
```

The editor uses the process working directory for its project and `assets` directory. Rebuild and restart the application after changing Rust source; the editor's Project -> Compile menu does not compile source code.

The game client automatically loads `assets/main.scene`. The starter does not include a scene, so create one in the editor and save it as `main.scene` if you want `cargo run` to start with that scene. Keep the `.scene.asset` sidecar file created by the editor.

## Build the Android client

For the first time, install cargo-ndk:

```
rustup target add aarch64-linux-android
cargo install cargo-ndk
```

In current directory:

```
cargo ndk -t arm64-v8a -o target/android/jniLibs build --lib
```

This creates the Android native library. Do not enable `steel-engine/editor` for Android. For a standalone project, copy Steel's `examples/android-project` into your project as `android-project` (exclude `.gradle`, `build`, and `app/build` directories), then use this output path instead:

```
cargo ndk -t arm64-v8a -o android-project/app/src/main/jniLibs build --lib
```

Copy the contents of your `assets` directory into `android-project/app/src/main/assets`, then run `gradlew assembleDebug` (`gradlew.bat assembleDebug` on Windows) from `android-project`. See the [tutorial's Android chapter](../../tutorial/eng/6-player-control.md) for platform-specific copy commands.

Before building, install Android Studio with the Android SDK and NDK, and configure `ANDROID_HOME` and `ANDROID_NDK_HOME`.

Install Java 17 and set `JAVA_HOME` to its installation directory.
