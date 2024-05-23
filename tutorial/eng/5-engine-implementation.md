# Engine implementation

After completing the scene building, we can start writing the game logic. This chapter introduces and implements the core trait of the Steel game engine: Engine.

## Add game source code to VSCode

In the top menu of VSCode, click "File -> Add Folder to Workspace" and select our game project directory, for example "D:\steel-projects\ball". In the src directory, you can see the contents of our lib.rs file as follows:

```rust
use steel::engine::{Engine, EngineImpl};

#[no_mangle]
pub fn create() -> Box<dyn Engine> {
    Box::new(EngineImpl::new())
}
```

Currently there is only one create function, and the return type is Box\<dyn Engine\>. There is only one line of code in the function, and it returns an EngineImpl structure wrapped in a Box. Please do not modify the function name and return value type of the create function, because the editor needs to call the create function to generate an Engine object after dynamically loading the game code. If you modify the function name or return value type of the create function, the editor will not be able to find the create function and crash.

## Engine introduction

Engine is actually a trait in Rust. Engine defines many game lifecycle methods, so that each game can control the specific content of the game frame loop so that different games can implement different logic.

### Lifecycles

The Steel game engine has three lifecycles: Engine::init, Engine::frame, and Engine::draw.

#### Engine::init

Engine::init is called once when the game program starts. You can:
* Register components or uniques that need to be edited in the editor;
* Add uniques to the ECS world.

#### Engine::frame

Engine::frame is called three times per frame and is divided into three stages: FrameStage::Maintain, FrameStage::Update, and FrameStage::Finish. You can get the current stage from info.stage. Engine::frame is mainly used to run systems that need to be run in each frame of the game.

##### FrameStage::Maintain

This is the first stage of a game frame. The editor executes this stage even when the game is not running, so the logic you write here will affect every frame in the editor. For example, a physics maintain system should be run in FrameStage::Maintain so that when you add or remove physics components in the editor, the physics world is updated immediately.

##### FrameStage::Update

This is the second stage of the game frame. The Editor skips this stage when the game is not running, so you can implement game logic that should not affect the Editor. For example, the physics update system should be run in FrameStage::Update so that physics objects do not fall due to gravity when the game is not running in the Editor.

##### FrameStage::Finish

This is the third stage of the game frame. The editor executes this stage even when the game is not running. For example, the rendering system can run in the FrameStage::Finish stage to collect vertex data after all entities and components have been updated in the previous stages.

#### Engine::draw

In each frame of the game, Engine::draw is called after Engine::frame, and in the editor, it is called twice for the scene window and the game window. If info.editor_info is Some, it means that the scene window is being drawn this time.

#### Engine::command

Engine::command is not a lifecycle function. The editor uses Engine::command to modify the game world. You usually don't need to implement this function, EngineImpl::command has a complete implementation, you just need to call EngineImpl::command.

## Implement Engine

In order to create our own custom components and systems, we must implement Engine ourselves. We first write an EngineWrapper struct that wraps an EngineImpl and modify the create method to return our EngineWrapper:

```rust
#[no_mangle]
pub fn create() -> Box<dyn Engine> {
    Box::new(EngineWrapper { inner: EngineImpl::new() })
}

struct EngineWrapper {
    inner: EngineImpl,
}

impl Engine for EngineWrapper {
    fn init(&mut self, info: InitInfo) {
        self.inner.init(info);
    }

    fn frame(&mut self, info: &FrameInfo) {
        self.inner.frame(info);
        match info.stage {
            FrameStage::Maintain => (),
            FrameStage::Update => (),
            FrameStage::Finish => (),
        }
    }

    fn draw(&mut self, info: DrawInfo) -> Box<dyn GpuFuture> {
        self.inner.draw(info)
    }

    fn command(&mut self, cmd: Command) {
        self.inner.command(cmd);
    }
}
```

Note: Since there are many use statements required in the code, this tutorial will not show how to modify the use statements. You can refer to the use statements in examples/ball/src/lib.rs to add them, or just copy all the use statements.

[Next: Player Control][6]

[Prev: Scene Building][4]

[Table of Contents][0]

[0]: table-of-contents.md
[1]: 1-introduction.md
[2]: 2-run-steel-editor.md
[3]: 3-create-project.md
[4]: 4-scene-building.md
[5]: 5-engine-implementation.md
[6]: 6-player-control.md
[7]: 7-push-the-ball.md
[8]: 8-game-lost.md
[9]: 9-main-menu.md
