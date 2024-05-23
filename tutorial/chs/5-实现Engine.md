# 实现Engine

完成场景搭建后，我们可以开始编写游戏逻辑了，本章介绍并实现Steel游戏引擎的核心trait：Engine。

## 添加游戏源码到VSCode

在VSCode的顶部菜单点击“文件 -> 将文件夹添加到工作区”，选择我们的游戏项目目录，例如“D:\steel-projects\ball”。在src目录下，可以看到我们的lib.rs文件内容如下：

```rust
use steel::engine::{Engine, EngineImpl};

#[no_mangle]
pub fn create() -> Box<dyn Engine> {
    Box::new(EngineImpl::new())
}
```

目前只有一个create函数，返回类型是Box\<dyn Engine\>。函数中只有一行代码，返回了一个由Box装着的EngineImpl结构。请不要修改create函数的函数名和返回值类型，因为编辑器需要在动态加载游戏代码后调用create函数生成一个Engine对象，如果你修改了create函数的函数名或返回值类型，会导致编辑器找不到create函数而崩溃。

## Engine介绍

Engine其实是rust中的一个trait，Engine中定义了许多游戏生命周期方法，使得每个游戏可以控制游戏帧循环的具体内容，以便不同的游戏实现不同的逻辑。

### 生命周期

Steel游戏引擎有3个生命周期：Engine::init、Engine::frame和Engine::draw。

#### Engine::init

Engine::init在游戏程序启动时被调用一次，你可以：
* 注册需要在编辑器中编辑的组件（Component）或单例（Unique）；
* 为ECS世界添加单例。

#### Engine::frame

Engine::frame每帧被调用三次，分为三个阶段：FrameStage::Maintain、FrameStage::Update和FrameStage::Finish。你可以从info.stage获取当前阶段。Engine::frame主要用于运行需要在游戏的每帧中运行的系统（System）。

##### FrameStage::Maintain

这是游戏帧的第一阶段。编辑器即使在游戏没有运行的时候也会执行这个阶段，因此你在这里写的逻辑也会影响编辑器的每一帧。例如，物理维护系统应该在FrameStage::Maintain中运行，这样当你在编辑器中添加或删除物理组件时，物理世界就会立即更新。

##### FrameStage::Update

这是游戏帧的第二阶段。编辑器在游戏未运行时会跳过此阶段，这样你就可以实现不应该影响编辑器的游戏逻辑。例如，物理更新系统应在FrameStage::Update中运行，这样当游戏在编辑器中没有运行时，物理物体就不会因为重力而下落。

##### FrameStage::Finish

这是游戏帧的第三阶段。编辑器即使在游戏没有运行的时候也会执行这个阶段。例如，所有实体和组件在之前的阶段更新后，渲染系统可以在FrameStage::Finish阶段中运行以收集顶点数据。

#### Engine::draw

在游戏的每一帧Engine::draw会在Engine::frame之后调用，并且在编辑器中，会为了场景窗口和游戏窗口分别调用两次。如果info.editor_info是Some，则意味着本次是在为场景窗口绘制。

#### Engine::command

Engine::command不属于生命周期函数。编辑器使用Engine::command来对游戏世界做修改。你通常不需要实现这个函数，EngineImpl::command已经有完整的实现，你只需要调用EngineImpl::command即可。

## 实现Engine

为了创建我们自定义的组件和系统，我们必须自己实现Engine。我们首先写一个EngineWrapper的struct，其中包装了一个EngineImpl，并同时修改create方法使得其返回我们的EngineWrapper：

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

注意：由于代码中需要的use语句较多，本教程后续不再展示use语句的修改，你可以参考examples/ball/src/lib.rs中的use语句添加，或者直接复制其所有use语句即可。

[下一章：玩家控制][6]

[上一章：场景搭建][4]

[目录][0]

[0]: 目录.md
[1]: 1-引言.md
[2]: 2-运行Steel编辑器.md
[3]: 3-创建项目.md
[4]: 4-场景搭建.md
[5]: 5-实现Engine.md
[6]: 6-玩家控制.md
[7]: 7-推一下球.md
[8]: 8-游戏失败.md
[9]: 9-主菜单.md
