use std::{
    fs,
    path::{Path, PathBuf},
};

use rust_font_manager::{
    db,
    font_scanner::{font_id, is_supported_font_path, parse_font_file, scan_directory},
    models::{FontCategory, FontQuery, LibraryTab},
    services::local_library::LocalLibraryService,
};
use tempfile::tempdir;

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

fn copy_font(root: &Path) -> Option<PathBuf> {
    let source = installed_font()?;
    let destination = root.join("copied-font.ttf");
    fs::copy(source, &destination).ok()?;
    Some(destination)
}

#[test]
fn supported_extensions_are_case_insensitive_and_reject_web_fonts() {
    assert!(is_supported_font_path(Path::new("SomeFont.TTF")));
    assert!(is_supported_font_path(Path::new("SomeFont.otf")));
    assert!(!is_supported_font_path(Path::new("SomeFont.woff2")));
    assert!(!is_supported_font_path(Path::new("SomeFont.ttf.bak")));
}

#[test]
fn malformed_bytes_are_reported_without_panicking() {
    let dir = tempdir().expect("temporary directory");
    let path = dir.path().join("broken.ttf");
    fs::write(&path, b"not an OpenType font").expect("write malformed font");
    assert!(parse_font_file(&path).is_err());

    let report = scan_directory(dir.path()).expect("directory scan");
    assert_eq!(report.discovered_candidates, 1);
    assert_eq!(report.failed.len(), 1);
}

#[test]
fn parser_extracts_names_and_file_metadata_from_a_system_font() {
    let Some(source) = installed_font() else {
        return;
    };
    let metadata = parse_font_file(&source).expect("valid installed font parses");
    assert!(!metadata.family.trim().is_empty());
    assert!(!metadata.subfamily.trim().is_empty());
    assert!(!metadata.full_name.trim().is_empty());
    assert!((1..=1000).contains(&metadata.weight));
    assert!(metadata.file_size > 0);
}

#[test]
fn font_identity_is_stable_and_variant_sensitive() {
    let dir = tempdir().expect("temporary directory");
    let path = dir.path().join("identity.ttf");
    fs::write(&path, b"identity input").expect("create identity target");
    let first = font_id(&path, None).expect("font identity");
    assert_eq!(first, font_id(&path, None).expect("repeat font identity"));
    assert_ne!(
        first,
        font_id(&path, Some("regular")).expect("variant identity")
    );
}

#[test]
fn scanner_continues_after_bad_file_and_ignores_unsupported_extensions() {
    let dir = tempdir().expect("temporary directory");
    fs::write(dir.path().join("broken.TTF"), b"bad").expect("write broken font");
    fs::write(dir.path().join("web.woff2"), b"ignored").expect("write web font");
    let valid = copy_font(dir.path());

    let report = scan_directory(dir.path()).expect("directory scan");
    assert_eq!(
        report.discovered_candidates,
        1 + usize::from(valid.is_some())
    );
    assert_eq!(report.skipped_unsupported, 1);
    assert_eq!(report.failed.len(), 1);
}

#[test]
fn library_service_persists_fonts_and_updates_scan_counts() {
    let dir = tempdir().expect("temporary directory");
    let Some(_) = copy_font(dir.path()) else {
        return;
    };
    let database = dir.path().join("library.sqlite3");
    let service = LocalLibraryService::new(&database);

    let first = service.scan_directory(dir.path()).expect("first scan");
    assert_eq!(first.imported, 1);
    let second = service.scan_directory(dir.path()).expect("second scan");
    assert_eq!(second.skipped_unchanged, 1);

    let page = db::list_fonts(
        &database,
        FontQuery {
            tab: LibraryTab::Local,
            search: String::new(),
            category: FontCategory::All,
            collection_id: None,
            page: 0,
            page_size: 10,
        },
    )
    .expect("read imported font");
    assert_eq!(page.total, 1);
}
