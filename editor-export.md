# Editor Export

Steps to export the steel-editor executable file:

1. Modify steel-engine version of CARGO_TOML in "steel-editor/src/project.rs" to the current supported version.

2. Use cargo build to generate executable file:
```
cargo build -p steel-editor -F desktop
```

3. Create steel-editor exported folder, copy following folders and files to the exported folder:
* steel-build
* Cargo.toml
* .gitignore
* target/debug/steel-editor.exe

4. Leave only the copied folders in Cargo.toml workspace:
```
members = [
    "steel-build/steel-dynlib",
    "steel-build/steel-client",
    "steel-build/steel-server",
]
exclude = [
    "steel-build/steel-project",
]
```

5. Modify steel-engine version and steel-common version of other Cargo.toml files not in the root directory to the current supported version.

6. Remove files and folders that are listed in .gitignore:
```
git init
git clean -Xdf
```

7. Remove folder ".git" and file ".gitignore".

8. Compress the exported folder into zip.

9. Test if the steel-editor executable can run.
* If fail, fix it and recompress into zip.
* If pass, you have the final output zip! Remember to reset steel-engine version of CARGO_TOML in "steel-editor/src/project.rs".
