# empty-project example

`empty-project` is a starter Steel application. The editor is integrated into the application and is supported on desktop only; Android builds must use the game client.

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

This creates the Android native library. Package it and this project's `assets` directory in an Android application; do not enable `steel-engine/editor` for Android. The repository includes an example Gradle application at `../android-project`; copy the generated `libmain.so` into its `app/src/main/jniLibs/arm64-v8a` directory and the project's assets into `app/src/main/assets`, then run `gradlew assembleDebug` (`gradlew.bat assembleDebug` on Windows) from that example directory.

Before building, install Android Studio with the Android SDK and NDK, and configure `ANDROID_HOME` and `ANDROID_NDK_HOME`.

Install Java 17 and set `JAVA_HOME` to its installation directory.
