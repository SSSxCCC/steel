# Introduction

## Steel game engine introduction

Steel is an open-source cross-platform Rust game engine with an integrated editor. Its game client supports Windows and Android; the editor is currently for desktop builds. Features include:
* It is completely open source, and the engine layer code can be easily modified if there is a need for customization;
* With a visual editor, you can develop games efficiently;
* The game client can be built for Windows or Android; Android packaging requires an Android application wrapper;
* Using modern rust language, while ensuring code stability and game performance;
* Use widely used rust open source libraries, such as [shipyard][shipyard], [glam][glam], [egui][egui], [vulkano][vulkano], [rapier][rapier], etc., to speed up the speed of getting started;
* Uses Vulkan and supports rasterized and ray-traced rendering;
* It is modular, and complex game modules, such as the physics system, can be easily added to your game as plugins.

Steel game engine is implemented based on ECS architecture. The most basic unit of the game world is the entity. Each entity can have several components. Each component can store custom data structures. In addition to data on the components, there are also uniques that can also store custom data structures. The system reads and writes these data to drive the running of the entire game world.

Currently, the ECS architecture of the Steel game engine is implemented using [shipyard][shipyard]. It is recommended that you quickly browse [shipyard tutorial][shipyard guide] before reading this tutorial to familiarize yourself with how to use shipyard.

## Introduction to this tutorial

This tutorial uses the `examples/ball` application to demonstrate a simple catch-the-ball game. The player moves a board left and right to catch a ball bouncing off the walls. If the ball falls below the board, the game is lost. The tutorial covers:
* Running a Steel application with the integrated editor;
* Add or delete entities and components;
* Game scene building and switching;
* Use physics engine to control object behavior;
* Write systems to implement game logic;
* Write game menu.

The editor is enabled with the `steel-engine/editor` Cargo feature on desktop. Game code is compiled as part of the application; after changing Rust source, rebuild and restart the application. The editor's Project -> Compile menu does not compile source code.

## Resources

* Github：<https://github.com/SSSxCCC/steel>
* Api documentation：<https://docs.rs/steel-engine/latest/steel/>

[Next: Run Steel Editor][2]

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
[rapier]: https://rapier.rs/
[glam]: https://github.com/bitshifter/glam-rs
[egui]: https://github.com/emilk/egui
[vulkano]: https://github.com/vulkano-rs/vulkano
[shipyard]: https://github.com/leudz/shipyard
[shipyard guide]: https://leudz.github.io/shipyard/guide/master/
