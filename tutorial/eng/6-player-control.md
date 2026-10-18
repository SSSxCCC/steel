# Player Control

In this chapter, we will write the control logic for the board.

## Player component

The board is controlled by the player. Add a component to mark it and store its movement speed:

```rust
#[derive(Component, Edit, Default)]
struct Player {
    move_speed: f32,
}
```

Register `Player` in the existing `SteelApp` builder chain in `main`:

```rust
    .register_component::<Player>()
```

Rebuild and restart the application with the editor feature enabled. Add `Player` to the Board entity, set `move_speed` to 10, and save the scene.

## player_control_system

Set the board's `RigidBody2D` body type to `KinematicVelocityBased` so its velocity is controlled by the game code rather than gravity. The game client exposes keyboard and touch input through the egui context:

```rust
fn player_control_system(
    player: View<Player>,
    mut transform: ViewMut<Transform>,
    rb2d: View<RigidBody2D>,
    mut physics2d_manager: UniqueViewMut<Physics2DManager>,
    egui_ctx: UniqueView<EguiContext>,
) {
    for (player, transform, rb2d) in (&player, &mut transform, &rb2d).iter() {
        if let Some(rb2d) = physics2d_manager.rigid_body_set.get_mut(rb2d.handle()) {
            let mut linvel = Vec2::ZERO;
            egui_ctx.input(|input| {
                if input.key_down(egui::Key::ArrowLeft) {
                    linvel = Vec2::new(-player.move_speed, 0.0);
                } else if input.key_down(egui::Key::ArrowRight) {
                    linvel = Vec2::new(player.move_speed, 0.0);
                }
                if steel::platform::BUILD_TARGET == BuildTarget::Android {
                    if let Some(press_origin) = input.pointer.press_origin() {
                        if press_origin.x < input.screen_rect.center().x {
                            linvel = Vec2::new(-player.move_speed, 0.0);
                        } else {
                            linvel = Vec2::new(player.move_speed, 0.0);
                        }
                    }
                }
            });
            rb2d.set_linvel(linvel.into(), true);

            if transform.position.x > 9.0 {
                transform.position.x = 9.0
            }
            if transform.position.x < -9.0 {
                transform.position.x = -9.0
            }
        }
    }
}
```

Register the system in `Schedule::Update`, which runs during gameplay:

```rust
    .add_system(Schedule::Update, 0, player_control_system)
```

The system sets a horizontal velocity while an arrow key is pressed. On Android, touching the left or right half of the screen selects the movement direction. The position checks keep the board inside the play area. These controls are also implemented in [`examples/ball/src/lib.rs`](../../examples/ball/src/lib.rs).

## Build for Android

The editor is not supported on Android. Android builds use the Steel client. Install Android Studio with the SDK and the NDK version required by the Gradle project, plus Java 17. Set `ANDROID_HOME`, `ANDROID_NDK_HOME`, and `JAVA_HOME`, then install the Rust target and cargo-ndk:

```
rustup target add aarch64-linux-android
cargo install cargo-ndk
```

Run the following steps from your own game project's root. First copy Steel's `examples/android-project` into your repository as `android-project`. Exclude the `.gradle`, `build`, and `app/build` cache directories.

Windows PowerShell (assuming the two repositories are siblings):

```
robocopy ..\steel\examples\android-project .\android-project /E /XD .gradle build app\build
```

Robocopy exit codes from 0 through 7 indicate that the copy completed without a fatal error.

macOS/Linux:

```
rsync -a --exclude='.gradle' --exclude='build' --exclude='app/build' ../steel/examples/android-project/ android-project/
```

Build the Android native library from your game project root:

```
cargo ndk -t arm64-v8a -o android-project/app/src/main/jniLibs build --lib
```

Copy the contents of your game's `assets` directory into `android-project/app/src/main/assets`.

Windows PowerShell:

```
New-Item -ItemType Directory -Force android-project\app\src\main\assets | Out-Null
Get-ChildItem assets -Force | Copy-Item -Destination android-project\app\src\main\assets -Recurse -Force
```

macOS/Linux:

```
mkdir -p android-project/app/src/main/assets
cp -R assets/. android-project/app/src/main/assets/
```

Then run `gradlew assembleDebug` (`gradlew.bat assembleDebug` on Windows) from `android-project`. The APK is created under `android-project/app/build/outputs/apk/debug`. The wrapper currently targets arm64 Android devices and uses the SDK/NDK versions configured in its Gradle files. Copy updated assets again before rebuilding after they change.

[Next: Push the Ball][7]

[Prev: Scene Building][5]

[Table of Contents][0]

[0]: table-of-contents.md
[1]: 1-introduction.md
[2]: 2-run-steel-editor.md
[3]: 3-create-project.md
[4]: 4-write-code.md
[5]: 5-scene-building.md
[6]: 6-player-control.md
[7]: 7-push-the-ball.md
[8]: 8-game-lost.md
[9]: 9-main-menu.md
