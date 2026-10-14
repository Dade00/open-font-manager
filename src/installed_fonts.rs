use std::{
    env,
    path::{Path, PathBuf},
};

/// Returns the standard Windows font directories that currently exist.
pub fn installed_font_roots() -> Vec<PathBuf> {
    let windows_dir = env::var_os("WINDIR").map(PathBuf::from);
    let local_app_data = env::var_os("LOCALAPPDATA").map(PathBuf::from);
    font_roots_from(windows_dir.as_deref(), local_app_data.as_deref())
}

/// Resolves the system-wide and per-user Windows font directories.
pub fn font_roots_from(windows_dir: Option<&Path>, local_app_data: Option<&Path>) -> Vec<PathBuf> {
    let candidates = windows_dir
        .map(|path| path.join("Fonts"))
        .into_iter()
        .chain(local_app_data.map(|path| path.join("Microsoft").join("Windows").join("Fonts")));

    let mut roots = Vec::new();
    let mut seen = Vec::new();
    for candidate in candidates {
        if !candidate.is_dir() {
            continue;
        }
        let root = candidate.canonicalize().unwrap_or(candidate);
        let identity = root.to_string_lossy().replace('\\', "/").to_lowercase();
        if !seen.contains(&identity) {
            seen.push(identity);
            roots.push(root);
        }
    }
    roots
}
