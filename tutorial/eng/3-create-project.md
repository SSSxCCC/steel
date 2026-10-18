# Create Project

In this chapter, create a Git repository of your own and start from Steel's `empty-project`. First clone the Steel repository as described in the previous chapter; the commands below assume the Steel checkout and your new project are sibling directories.

## Create your repository

1. Create an empty repository on GitHub. Do not initialize it with a README, `.gitignore`, or license.
2. Clone your repository:

```
git clone https://github.com/<your-user>/<your-repository>.git my-game
cd my-game
```

3. Copy the contents of Steel's `examples/empty-project` into the root of your new repository. Include hidden files such as `.gitignore`.

Windows PowerShell:

```
Get-ChildItem ..\steel\examples\empty-project -Force | Copy-Item -Destination . -Recurse -Force
```

macOS/Linux:

```
cp -R ../steel/examples/empty-project/. .
```

If your Steel checkout is not next to the new project, adjust the source path.

4. Change `package.name` and `[[bin]].name` in `Cargo.toml` to your project name. Keep `[lib] name = "main"` because the Android template loads `libmain.so`. Remove the existing `steel-engine` line under `[dependencies]` (it points to the Steel checkout), then add the latest stable Steel release from crates.io:

```
cargo add steel-engine
```

This writes the current latest version to `Cargo.toml`. This tutorial targets Steel 0.3, so the generated dependency should be `steel-engine = "0.3.0"`. Do not keep the template's `path = "../.."`.

Commit `Cargo.lock` with your application so builds resolve reproducibly. `cargo update -p steel-engine` updates the lockfile to the newest version allowed by the current requirement. Cargo follows semantic versioning, and 0.x minor versions may be incompatible: an existing `"0.3.0"` requirement will not automatically move to 0.4. When Steel 0.4 is released, update the requirement to `steel-engine = "0.4"` (for example, `cargo add steel-engine@0.4`) and run `cargo update -p steel-engine`. This tutorial will be updated for each stable Steel release; creating a project with `cargo add steel-engine` then selects the latest published version.

## Run the project and integrated editor

Run this from your new project root:

```
cargo run --features steel-engine/editor
```

Run `cargo run` without the feature to start the game client. The editor uses the `assets` directory under the process's current working directory. Create that directory and put your scenes and other game resources there.

Windows PowerShell:

```
New-Item -ItemType Directory -Force assets | Out-Null
```

macOS/Linux:

```
mkdir -p assets
```

The starter entry point is in `src/lib.rs` and `src/main.rs`.

The game client automatically loads `assets/main.scene` as its startup scene. Save the scene you want `cargo run` to start as `main.scene`; other scene files are available to your game code but are not selected automatically. The empty starter has no scene yet, so it starts with an empty world until you create and save `main.scene`.

After changing Rust source, rebuild and relaunch the application. The editor's Project -> Compile menu does not invoke Cargo. The complete catch-the-ball game from the tutorial remains available under `examples/ball` in the Steel repository.

## Push to your remote repository

```
git add .
git commit -m "Create Steel game project"
git push -u origin main
```

If GitHub created your repository with a different default branch, replace `main` with that branch name.

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
