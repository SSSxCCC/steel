# Player Control

In this chapter, we will write the control logic of the board.

## Player component

The board is the object controlled by the player. We can create a component Player to mark this object and add a member variable speed to set the control speed:

```rust
#[derive(Component, Edit, Default)]
struct Player {
    move_speed: f32,
}
```

As a component, Player first needs to implement Component. In order for our editor to add/delete/modify the Player component, we need to implement Edit. After implementing the Edit component, we must also implement Default, so that the editor can set a default value for it when adding this component.

After the Player component is written, you only need to register it in SteelApp and you can use it in the editor:

```rust
#[no_mangle]
pub fn create() -> Box<dyn App> {
    SteelApp::new()
        .add_plugin(Physics2DPlugin)
        .register_component::<Player>()
        .boxed()
}
```

After modifying the code, click the "Project -> Compile" button in the top menu to compile. After the compilation is complete, you can add a Player component to our Board entity, set the move_speed to 10, and then click "Scene -> Save" in the top menu to save the scene.

## player_control_system

Next, we want to control the Board entity by changing its movement speed through the keyboard. Then it should not fall due to gravity, nor should it be affected by any other force that changes its speed. At this point, we can change the body_type of its RigidBody2D component to KinematicVelocityBased, which ensures that its speed is controlled by our code.

Implement player_control_system:

```rust
fn player_control_system(
    player: View<Player>,
    mut transform: ViewMut<Transform>,
    rb2d: View<RigidBody2D>,
    mut physics2d_manager: UniqueViewMut<Physics2DManager>,
    input: UniqueView<Input>,
) {
    for (player, transform, rb2d) in (&player, &mut transform, &rb2d).iter() {
        if let Some(rb2d) = physics2d_manager.rigid_body_set.get_mut(rb2d.handle()) {
            let mut linvel = Vec2::ZERO;
            if input.key_held(VirtualKeyCode::Left) {
                linvel = Vec2::new(-player.move_speed, 0.0);
            } else if input.key_held(VirtualKeyCode::Right) {
                linvel = Vec2::new(player.move_speed, 0.0);
            }
            rb2d.set_linvel(linvel.into(), true);
        }
    }
}
```

We iterate over all entities that have Player components, Transform components, and RigidBody2D components. Such an entity in the scene must be our Board, so the for loop should only be executed once. By passing in the handle of the RigidBody2D component through the physics2d_manager.rigid_body_set.get_mut method, we can get the RigidBody object in the physical world. The Input unique provides reading of the current key status, and its key_held function can query whether a key is currently pressed. If the left or right key is currently pressed on the keyboard, we set a left or right speed through the set_linvel method of RigidBody. This achieves the control of the Board.

We can run player_control_system in the Schedule::Update, because this schedule will be skipped when the game is not running in the editor, and we don't want to be able to control the board when the game is not running:

```rust
#[no_mangle]
pub fn create() -> Box<dyn App> {
    SteelApp::new()
        .add_plugin(Physics2DPlugin)
        .register_component::<Player>()
        .add_system(Schedule::Update, player_control_system)
        .boxed()
}
```

Try running the game now, and press the left and right buttons on the keyboard to control the board to move left and right. But you will soon find a problem, the board can move outside the screen. To solve this problem, we can add a range limit to the position of the board:

```rust
fn player_control_system(
    player: View<Player>,
    mut transform: ViewMut<Transform>,
    rb2d: View<RigidBody2D>,
    mut physics2d_manager: UniqueViewMut<Physics2DManager>,
    input: UniqueView<Input>,
) {
    for (player, transform, rb2d) in (&player, &mut transform, &rb2d).iter() {
        if let Some(rb2d) = physics2d_manager.rigid_body_set.get_mut(rb2d.handle()) {
            ...
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

## Adapt to Android system

There is another problem with player_control_system. If our game runs on an Android phone, generally speaking, the Android system is not connected to a keyboard. The Android system is accustomed to using the touch screen for control. We also need to customize a set of control methods for Android. Here we make a simple implementation. If the left half of the screen is touched, the Board moves to the left. If the right half of the screen is touched, the Board moves to the right:

```rust
fn player_control_system(
    player: View<Player>,
    mut transform: ViewMut<Transform>,
    rb2d: View<RigidBody2D>,
    mut physics2d_manager: UniqueViewMut<Physics2DManager>,
    input: UniqueView<Input>,
    egui_ctx: UniqueView<EguiContext>,
) {
    for (player, transform, rb2d) in (&player, &mut transform, &rb2d).iter() {
        if let Some(rb2d) = physics2d_manager.rigid_body_set.get_mut(rb2d.handle()) {
            let mut linvel = Vec2::ZERO;
            ...
            if steel::platform::BUILD_TARGET == BuildTarget::Android {
                egui_ctx.input(|input| {
                    if let Some(press_origin) = input.pointer.press_origin() {
                        if press_origin.x < input.screen_rect.center().x {
                            linvel = Vec2::new(-player.move_speed, 0.0);
                        } else {
                            linvel = Vec2::new(player.move_speed, 0.0);
                        }
                    }
                });
            }
            rb2d.set_linvel(linvel.into(), true);
            ...
        }
    }
}
```

We use the constant steel::platform::BUILD_TARGET to determine whether the current compilation is for the Android system. If it is for the Android system, we add the operation logic of the Android system. We use another unique EguiContext this time to implement touch screen judgment for the left and right half of the screen, because Input currently does not support the processing of touch screen events.

Before compiling the game into an Android apk, you need to install the Android SDK. You can install the Android SDK by installing [Android Studio](https://developer.android.com/studio). Then you also need to execute the following command to install cargo-ndk:

```
rustup target add aarch64-linux-android
cargo install cargo-ndk
```

Insert an Android phone that can be debugged with adb, click "Project -> Export -> Android" in the top menu of the editor to compile to the Android phone to view the effect.

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
