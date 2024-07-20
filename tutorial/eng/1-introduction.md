# Introduction

## Steel game engine introduction

Steel is an open source cross-platform rust game engine with the following features:
* It is completely open source, and the engine layer code can be easily modified if there is a need for customization;
* With a visual editor, you can develop games efficiently;
* The game can be compiled into a Windows program or an Android application with one click;
* Using modern rust language, while ensuring code stability and game performance;
* Use widely used rust open source libraries, such as [shipyard][shipyard], [glam][glam], [egui][egui], [vulkano][vulkano], [rapier][rapier], etc., to speed up the speed of getting started;
* Using vulkan, an advanced graphics API, can achieve any modern graphics effect;
* It is modular, and complex game modules, such as the physics system, can be easily added to your game as plugins.

Steel game engine is implemented based on ECS architecture. The most basic unit of the game world is the entity. Each entity can have several components. Each component can store custom data structures. In addition to data on the components, there are also uniques that can also store custom data structures. The system reads and writes these data to drive the running of the entire game world.

Currently, the ECS architecture of the Steel game engine is implemented using [shipyard][shipyard]. It is recommended that you quickly browse [shipyard tutorial][shipyard guide] before reading this tutorial to familiarize yourself with how to use shipyard.

## Introduction to this tutorial

This tutorial uses the Steel editor to gradually build a simple catching ball game to lead users to learn to use the Steel engine. In this tutorial game, the user controls the board to move left and right to catch the ball that bounces back and forth on the wall. If the ball falls under the board, the game fails. With this simple 2D game you will learn to use the Steel engine:
* Use Steel editor to create projects;
* Add or delete entities and components;
* Game scene building and switching;
* Use physics engine to control object behavior;
* Write systems to implement game logic;
* Write game menu.

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
