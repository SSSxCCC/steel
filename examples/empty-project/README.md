# empty-project example

empty-project is a simple cross-platform example that can be compiled and run directly on any platform supported by the Steel Engine. You can copy this directory as a newly created project.

## Run in desktop

In current directory:

```
cargo run
```

## Run editor in desktop

In current directory:

```
cargo run -F steel-engine/editor
```

## Run in android

For the first time, install cargo-ndk:

```
rustup target add aarch64-linux-android
cargo install cargo-ndk
```

In current directory:

```
cargo ndk -t arm64-v8a -o ../android-project/app/src/main/jniLibs/ build
```

Delete the assets folder under the ../android-project/app/src/main directory, and copy the assets folder from the current directory there.

In "../android-project":

```
.\gradlew build
.\gradlew installDebug
```

Note: before compiling the android apk, you need to install the Android SDK and Java.

1. Android SDK: You can install the Android SDK by installing [Android Studio](https://developer.android.com/studio). At the same time, install NDK in the SDK Manager of Android Studio. And set the system environment variables ANDROID_HOME and ANDROID_NDK_HOME to the installation directories of SDK and NDK respectively.

2. Java: Install [Java](https://www.oracle.com/java/technologies/javase/jdk17-archive-downloads.html), and be sure to select 64-bit Java-17 to download and install. And set the system environment variable JAVA_HOME to the installation directory of Java.
