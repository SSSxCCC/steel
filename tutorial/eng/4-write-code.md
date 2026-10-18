# Write Code

Game code lives in the Rust application, not in a dynamically loaded library. Open `src/lib.rs` in the project you created in the previous chapter. The complete tutorial game is available for reference at `examples/ball/src/lib.rs` in the Steel repository.

## Application entry point

The `#[steel_main]` attribute generates the platform entry point used by Steel. `SteelApp::run()` starts the game client by default, or starts the integrated editor when the `steel-engine/editor` feature is enabled.

```rust
use steel::{
    app::{Schedule, SteelApp},
    log, steel_main,
};

#[steel_main]
fn main() {
    SteelApp::new()
        .add_system(Schedule::Init, 0, hello_world)
        .run();
}

fn hello_world() {
    log::info!("Hello world!");
}
```

`add_system` takes a schedule, an order within that schedule, and the system function. Run your application from its project root:

```
cargo run --features steel-engine/editor
```

The editor executes the application that was built with it. After changing Rust code, stop and rebuild/relaunch the application; Project -> Compile does not compile Rust source. Without the feature, `cargo run` starts the game client.

## Add Physics2DPlugin

The ball application uses a 2D physics engine. Add the plugin to the existing `SteelApp` builder chain:

```rust
use steel::physics2d::Physics2DPlugin;

SteelApp::new()
    .add_plugin(Physics2DPlugin)
    // Register components and add systems here.
    .run();
```

Plugins group components, uniques, and systems needed by a game module. `Physics2DPlugin` installs the 2D physics functionality used by the tutorial.

The following chapters show additions to the existing `SteelApp` builder and gameplay systems. They are snippets, not replacement application entry points. For complete imports and a working application, refer to [`examples/ball/src/lib.rs`](../../examples/ball/src/lib.rs).

[Next: Scene Building][5]

[Prev: Create Project][3]

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
