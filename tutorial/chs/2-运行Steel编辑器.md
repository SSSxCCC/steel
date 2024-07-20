# 运行Steel编辑器

## 搭建环境

目前推荐在VSCode中运行，方便调试。
1. 在Windows系统安装[Visual Studio 2022][Visual Studio 2022]的"使用C++的桌面开发"，保持默认勾选
2. 为了[shaderc-rs][shaderc-rs]，安装[Rust][Rust]，[Git][Git]，[Python][Python]，[CMake][CMake]和[Ninja][Ninja]
3. 安装[VSCode][VSCode]及其"C/C++"和"rust-analyzer"插件
4. 下载Steel的源码，并切换到v0.2版本的分支，因为当前教程是基于这个版本写的：
```
git clone https://github.com/SSSxCCC/steel
cd steel
git checkout v0.2
```
5. 使用VSCode打开根目录“steel”，按F5即可编译运行

VSCode存在首次按F5时无法运行exe文件的bug。你可以在控制台中编译并运行exe文件：

```
cargo run -p steel-editor -F desktop
```

## Steel编辑器界面介绍

成功运行Steel编辑器后，你可以看到如下界面：

![image](../images/steel-editor.png)

整个界面的前面是编辑器窗口，在编辑器窗口中有顶部菜单功能按钮，和多个子页面。子页面主要包括：
* Scene：场景窗口，这个窗口用于查看场景内容；
* Game：游戏窗口，这个窗口用于查看实际游戏画面；
* Entities：显示当前场景所有的实体（Entity）；
* Entity：显示当前选中的实体的所有组件（Component）；
* Uniques：显示当前场景所有的单例（Unique）；
* Unique：显示当前选中的单例的内容。

编辑器窗口的后面是VSCode窗口。编辑器中输出的所有log都显示在VSCode下面的控制台中。目前所有Steel引擎代码都已经在VSCode中打开，可以随时查看和修改。同时VSCode也用于查看修改Steel编辑器创建的项目代码，具体方式将在后续章节中介绍。

Steel引擎源码目录下面有一个examples目录，其下面有一个ball目录，是本教程的示例项目接球游戏最终完成的内容，如果你在阅读本教程遇到了困难，可以随时打开examples目录下的ball目录来参考。

[下一章：创建项目][3]

[上一章：引言][1]

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
[Rust]: https://www.rust-lang.org/
[Git]: https://git-scm.com/
[Python]: https://www.python.org/
[CMake]: https://cmake.org/
[Ninja]: https://github.com/ninja-build/ninja/releases
[shaderc-rs]: https://github.com/google/shaderc-rs
[Visual Studio 2022]: https://visualstudio.microsoft.com/vs/
[VSCode]: https://code.visualstudio.com/
