# Create Project

The current editor uses the Steel application that was built to launch it; it does not create or compile a separate Rust project from the Project menu. Start from the included example project instead.

## Use the ball example

The repository's `examples/ball` package contains the tutorial game source and assets. Launch it with `examples/ball` as the working directory so the editor finds the example's `assets` directory:

```
cd examples/ball
cargo run --features steel-engine/editor
```

## Start a new project

Copy `examples/empty-project` to a directory under the repository's `examples` folder, then rename the package in its `Cargo.toml`. Its `steel-engine` path dependency is relative to the repository root; if you place the project elsewhere, adjust that path to point to the checkout. Run `cargo run --features steel-engine/editor` from the new project directory. The editor uses that process working directory and its `assets` directory for game resources.

The editor's Project -> Compile menu initializes/reloads the app state; it does not invoke Cargo to compile changed Rust source. Rebuild and restart the application after changing game code.

[Next: Write Code][4]

[Prev: Run Steel Editor][2]

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
