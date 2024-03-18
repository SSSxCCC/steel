# The Steel Game Engine

The Steel Game Engine is an open source cross-platform rust game engine with the following features:
* It is completely open source, and the engine layer code can be easily modified if there is a need for customization;
* With a visual editor, you can develop games efficiently;
* The game can be compiled into a Windows program or an Android application with one click;
* Using modern rust language, while ensuring code stability and game performance;
* Use widely used rust open source libraries, such as rapier, glam, egui, shipyard, vulkano, etc., to speed up the speed of getting started;
* Using vulkan, an advanced graphics API, can achieve any modern graphics effect.

## Build and run Steel Editor

Currently, it is recommended to run in VSCode, convenient for debugging.
1. Install Visual Studio 2022 with "Desktop development with C++", keep the default check
2. For [shaderc-rs][shaderc-rs], install [CMake][CMake], [Git][Git], [Python][Python] and [Ninja][Ninja]
3. Install VSCode with "C/C++" and "rust-analyzer" extensions in win10/win11
4. Download the code of this project and use VSCode to open the root directory of this project
5. Press F5 to compile and run

## Development Roadmap

- [x] Game core module
- [x] Visual editor
- [x] Vulkan render pipeline
- [x] 2D rendering basic
- [ ] 2D texture
- [ ] 3D rendering basic
- [ ] 3D model
- [ ] Ray traced rendering
- [x] 2D physics
- [ ] 3D physics
- [x] Build Windows program
- [x] Build Android application
- [ ] Customize build
- [ ] Write a tutorial
- [ ] Tests

---

Steel引擎是一个开源跨平台rust游戏引擎，主要有以下特性：
* 是完全开源的，如果有定制需要可以方便的修改引擎层代码；
* 具有可视化编辑器，可以高效的开发游戏；
* 制作的游戏可以一键编译成Windows程序或Android应用；
* 使用现代rust语言，同时保证了代码稳定性与游戏运行性能；
* 使用了被广泛使用的rust开源库，例如rapier，glam，egui，shipyard，vulkano等，加快上手速度；
* 使用了vulkan这种先进的图形api，可以实现任何现代图形效果。

## 编译并运行Steel引擎编辑器

目前推荐在VSCode中运行，方便调试。
1. 安装Visual Studio 2022的"使用C++的桌面开发"，保持默认勾选
2. 为了[shaderc-rs][shaderc-rs]，安装[CMake][CMake]，[Git][Git]，[Python][Python]和[Ninja][Ninja]
3. 在win10/win11安装VSCode及其"C/C++"和"rust-analyzer"插件
4. 下载本项目代码，使用VSCode打开本项目根目录
5. 按F5即可编译运行

## 开发路线图

- [x] 游戏核心模块
- [x] 可视化编辑器
- [x] Vulkan渲染管线
- [x] 2D渲染基础
- [ ] 2D纹理
- [ ] 3D渲染基础
- [ ] 3D模型
- [ ] 光线追踪渲染
- [x] 2D物理
- [ ] 3D物理
- [x] 编译Windows程序
- [x] 编译Android应用
- [ ] 定制编译
- [ ] 写一个教程
- [ ] 测试

[shaderc-rs]: (https://github.com/google/shaderc-rs)
[CMake]: (https://cmake.org/)
[Git]: (https://git-scm.com/)
[Python]: (https://www.python.org/)
[Ninja]: (https://github.com/ninja-build/ninja/releases)
