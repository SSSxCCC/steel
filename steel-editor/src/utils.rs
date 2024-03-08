use std::path::PathBuf;

pub fn delte_windows_path_prefix(path: &mut PathBuf) {
    const WINDOWS_PATH_PREFIX: &str = r#"\\?\"#;
    let path_string = path.display().to_string();
    if path_string.starts_with(WINDOWS_PATH_PREFIX) {
        // TODO: convert PathBuf to String and back to PathBuf may lose data, find a better way to do this
        *path = PathBuf::from(&path_string[WINDOWS_PATH_PREFIX.len()..]);
    };
}
