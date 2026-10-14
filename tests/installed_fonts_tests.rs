use std::{fs, path::PathBuf};

use rust_font_manager::{
    installed_fonts::{font_roots_from, is_installed_font_path},
    services::local_library::LocalLibraryService,
};
use tempfile::tempdir;

#[test]
fn installed_roots_include_system_and_per_user_font_directories() {
    let dir = tempdir().expect("temporary directory");
    let windows = dir.path().join("Windows");
    let local_app_data = dir.path().join("User/AppData/Local");
    let system_fonts = windows.join("Fonts");
    let user_fonts = local_app_data.join("Microsoft/Windows/Fonts");
    fs::create_dir_all(&system_fonts).expect("system font directory");
    fs::create_dir_all(&user_fonts).expect("per-user font directory");

    let roots = font_roots_from(Some(&windows), Some(&local_app_data));

    assert_eq!(
        roots,
        vec![
            system_fonts.canonicalize().expect("canonical system root"),
            user_fonts.canonicalize().expect("canonical user root"),
        ]
    );
}

#[test]
fn installed_font_path_is_recognized_only_inside_a_windows_font_root() {
    let dir = tempdir().expect("temporary directory");
    let root = dir.path().join("Windows/Fonts");
    let nested = root.join("Managed/Example.ttf");
    let outside = dir.path().join("Downloads/Example.ttf");
    fs::create_dir_all(nested.parent().unwrap()).expect("nested font directory");
    fs::create_dir_all(outside.parent().unwrap()).expect("outside directory");
    fs::write(&nested, b"font").expect("installed font");
    fs::write(&outside, b"font").expect("uninstalled font");

    assert!(is_installed_font_path(&nested, std::slice::from_ref(&root)));
    assert!(!is_installed_font_path(
        &outside,
        std::slice::from_ref(&root)
    ));
}

#[test]
fn installed_root_scan_imports_fonts_from_each_root() {
    let Some(source) = installed_font() else {
        return;
    };
    let dir = tempdir().expect("temporary directory");
    let system_root = dir.path().join("system");
    let user_root = dir.path().join("user");
    fs::create_dir_all(&system_root).expect("system font directory");
    fs::create_dir_all(&user_root).expect("user font directory");
    fs::copy(&source, system_root.join("system-font.ttf")).expect("copy system font");
    fs::copy(&source, user_root.join("user-font.ttf")).expect("copy user font");
    let service = LocalLibraryService::new(dir.path().join("library.sqlite3"));

    let reports = service
        .scan_font_roots([system_root.clone(), user_root.clone()])
        .expect("scan installed roots");

    assert_eq!(reports.len(), 2);
    assert!(reports.iter().all(|report| report.imported == 1));
}

#[test]
fn installed_font_scan_discovers_the_current_windows_font_roots() {
    let dir = tempdir().expect("temporary directory");
    let service = LocalLibraryService::new(dir.path().join("library.sqlite3"));

    let reports = service
        .scan_installed_fonts()
        .expect("scan installed Windows fonts");
    let discovered = reports
        .iter()
        .map(|report| report.discovered_candidates)
        .sum::<usize>();
    let imported = reports.iter().map(|report| report.imported).sum::<usize>();

    assert!(
        !reports.is_empty(),
        "expected at least one installed font root"
    );
    assert!(discovered > 0, "expected installed font files");
    assert!(
        imported > 0,
        "expected installed fonts to enter the catalog"
    );
}

fn installed_font() -> Option<PathBuf> {
    let root = std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:/Windows"))
        .join("Fonts");
    ["arial.ttf", "segoeui.ttf", "calibri.ttf", "times.ttf"]
        .iter()
        .map(|name| root.join(name))
        .find(|path| path.is_file())
}
