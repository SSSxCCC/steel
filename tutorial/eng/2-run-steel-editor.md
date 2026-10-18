# Run Steel Editor

## Set up the environment

The editor is integrated into a Steel application and is not a standalone executable. The editor feature is for desktop builds; Android applications should use `steel-client`.
1. Install [Visual Studio 2022][Visual Studio 2022] with "Desktop development with C++" in Windows, keep the default check.
2. Install [Rust][Rust], [Git][Git], [Python][Python], [CMake][CMake] and [Ninja][Ninja] for [shaderc-rs][shaderc-rs].
3. Install [VSCode][VSCode] with "C/C++" and "rust-analyzer" extensions.

## Run Steel Editor

Open the repository in VSCode if desired, then run an example application with the editor feature enabled.

### Run from source

1. Clone the repository:
```
git clone https://github.com/SSSxCCC/steel
cd steel
```
2. Start the ball example with the editor:
```
cd examples/ball
cargo run --features steel-engine/editor
```

To use the starter project instead, run this from the repository root:
```
cargo run --manifest-path examples/empty-project/Cargo.toml --features steel-engine/editor
```

The editor uses the `assets` directory under the process's current working directory. For the ball example, launch the command from `examples/ball` so its project assets are used.

To run without the editor, run `cargo run` from the example directory. Game code changes require rebuilding and relaunching the application; the editor's Project -> Compile menu does not compile Rust source code.

## Steel editor UI introduction

After successfully running the Steel editor, you can see the following interface:

![image](../images/steel-editor.png)

In front of the entire interface is the editor window. In the editor window there are top menu function buttons and multiple tabs. Tabs mainly include:
* Scene: Scene window, this window is used to view scene content;
* Game: Game window, this window is used to view the actual game screen;
* Entities: Displays all entities in the current scene;
* Entity: Displays all components of the currently selected entity;
* Uniques: Display all uniques in the current scene;
* Unique: Displays the contents of the currently selected unique.

Run the command from a terminal to see log output there. VSCode can be used to edit both the engine and example application source.

The `examples/ball` directory contains the source and assets for the catch-the-ball game used in this tutorial.

[Next: Create Project][3]

[Prev: Introduction][1]

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
[Rust]: https://www.rust-lang.org/
[Git]: https://git-scm.com/
[Python]: https://www.python.org/
[CMake]: https://cmake.org/
[Ninja]: https://github.com/ninja-build/ninja/releases
[shaderc-rs]: https://github.com/google/shaderc-rs
[Visual Studio 2022]: https://visualstudio.microsoft.com/vs/
[VSCode]: https://code.visualstudio.com/
