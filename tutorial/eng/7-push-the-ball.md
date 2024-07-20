# Push the Ball

Currently, our ball falls freely due to gravity after the game starts running, which is too slow and the player can easily catch the ball. In this chapter, we will give the ball an initial velocity to increase the difficulty of the game a little.

## Ball component

First, write a Ball component:

```rust
#[derive(Component, Edit, Default)]
struct Ball {
    start_velocity: Vec2,
    #[edit(limit = "Limit::ReadOnly")]
    started: bool,
}
```

We store the initial velocity of the ball in the member variable start_velocity so that we can change it at any time in the editor. Its type is Vec2, because velocity is a vector, which defines both the direction and magnitude of the velocity. We also have a member variable started of type bool to record whether we have given the ball an initial velocity, to prevent us from setting the initial velocity of the ball multiple times. The started variable does not need to be modified in the editor, so we can add the edit attribute to it, in which we set limit to Limit::ReadOnly, so that this variable is read-only in the editor.

After writing the Ball component, remember to register it in SteelApp:

```rust
pub fn create() -> Box<dyn App> {
    SteelApp::new()
        ...
        .register_component::<Ball>()
        ...
}
```

Click the "Project -> Compile" button in the top menu again to compile. You can then add a Ball component to the Ball entity in the editor and set the value of start_velocity, for example (8, 16).

## push_ball_system

Next, write a push_ball_system that gives the ball its initial velocity:

```rust
fn push_ball_system(
    mut ball: ViewMut<Ball>,
    rb2d: View<RigidBody2D>,
    mut physics2d_manager: UniqueViewMut<Physics2DManager>,
) {
    for (ball, rb2d) in (&mut ball, &rb2d).iter() {
        if !ball.started {
            if let Some(rb2d) = physics2d_manager.rigid_body_set.get_mut(rb2d.handle()) {
                rb2d.set_linvel(ball.start_velocity.into(), true);
                ball.started = true;
            }
        }
    }
}
```

We find our ball by looking for entities that have both a Ball component and a RigidBody2D component. If started is false, it means we haven't set an initial velocity for it yet, so we set an initial velocity and set started to true to ensure that the velocity is not set next time.

push_ball_system can also be run in Schedule::Update:

```rust
#[no_mangle]
pub fn create() -> Box<dyn App> {
    SteelApp::new()
        ...
        .add_system(Schedule::Update, push_ball_system)
        ...
}
```

Compile and run the game, and you should see the ball fly out at the beginning of the game.

[Next: Game Lost][8]

[Prev: Player Control][6]

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
