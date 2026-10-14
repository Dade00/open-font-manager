use std::{
    env, fs,
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

/// Returns whether `font_path` is physically located in one of Windows' font roots.
pub fn is_installed_font_path(font_path: &Path, roots: &[PathBuf]) -> bool {
    if !font_path.is_file() {
        return false;
    }
    let font_path = fs::canonicalize(font_path).unwrap_or_else(|_| font_path.to_path_buf());
    let font_path = normalized_path(&font_path);
    roots.iter().any(|root| {
        let root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        let root = normalized_path(&root);
        font_path == root || font_path.starts_with(&format!("{root}/"))
    })
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}
