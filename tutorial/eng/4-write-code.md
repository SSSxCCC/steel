# Write Code

This chapter explains how to view and write code.

## Add game source code to VSCode

In the top menu of VSCode, click "File -> Add Folder to Workspace" and select our game project directory, for example "D:\steel-projects\ball". In the src directory, you can see the contents of our lib.rs file as follows:

```rust
use steel::app::{App, SteelApp};

#[no_mangle]
pub fn create() -> Box<dyn App> {
    SteelApp::new().boxed()
}
```

Currently there is only one create function, and the return type is Box\<dyn App\>. There is only one line of code in the function, and it returns an SteelApp structure wrapped in a Box. Please do not modify the function name and return value type of the create function, because the editor needs to call the create function to generate an App object after dynamically loading the game code. If you modify the function name or return value type of the create function, the editor will not be able to find the create function and crash.

## SteelApp introduction

In the Steel engine, we use SteelApp to add/register entities, components, uniques, and systems to the ecs world to build our game program content.

For example, write a hello_world system that runs at the beginning of the game:

```rust
use steel::app::{App, SteelApp, Schedule};

#[no_mangle]
pub fn create() -> Box<dyn App> {
    SteelApp::new()
        .add_system(Schedule::Init, hello_world)
        .boxed()
}

fn hello_world() {
    log::info!("Hello world!");
}
```

After modifying the code, you need to recompile it to synchronize the changes to the editor. Return to the Steel Editor interface and click the "Project -> Compile" button in the top menu to initiate the compilation. The output of "Project::compile end" in the console indicates that the compilation is successful. After the compilation is complete, you should also be able to see the output of "Hello world!" in the console, indicating that the system is running successfully.

## Add Physics2DPlugin

Because our ball game needs to use the 2D physics engine, add the Physics2DPlugin to the code:

```rust
use steel::{
    app::{App, SteelApp},
    physics2d::Physics2DPlugin,
};

#[no_mangle]
pub fn create() -> Box<dyn App> {
    SteelApp::new()
        .add_plugin(Physics2DPlugin)
        .boxed()
}
```

Plugins are a collection of components, uniques, and systems that can add all the content needed for an entire game module at once. Plugins make the Steel engine modular. You can easily add any module written by others, or you can write a Steel engine module for others to use. Physics2DPlugin is a plugin that provides all the content needed for a 2D physics engine.

After completing the code modification, remember to return to the Steel editor interface and click the "Project -> Compile" button in the top menu to initiate the compilation.

Note: Since there are many use statements required in the code, this tutorial will not show how to modify the use statements. You can refer to the use statements in examples/ball/src/lib.rs to add them, or just copy all the use statements.

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
