use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use open_font_manager_lib::AppBackend;
use rust_font_manager::{
    db,
    errors::AppError,
    models::{FontCategory, FontItem, FontQuery, FontSource, LibraryTab},
    services::font_install::FontRegistration,
};
use tempfile::tempdir;

#[derive(Clone, Default)]
struct FakeFontRegistration(Arc<Mutex<HashMap<String, PathBuf>>>);

impl FontRegistration for FakeFontRegistration {
    fn register(&self, name: &str, path: &Path) -> Result<(), AppError> {
        self.0
            .lock()
            .unwrap()
            .insert(name.to_owned(), path.to_path_buf());
        Ok(())
    }

    fn unregister(&self, name: &str) -> Result<(), AppError> {
        self.0.lock().unwrap().remove(name);
        Ok(())
    }
}

#[test]
fn backend_filters_fonts_updates_favorites_and_returns_preview_bytes() {
    let data = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let font_file = data.path().join("specimen.ttf");
    let specimen = b"deterministic font preview bytes";
    fs::write(&font_file, specimen).unwrap();

    let backend = AppBackend::from_roots(data.path(), cache.path()).unwrap();
    let item = FontItem {
        id: "fixture-regular".into(),
        family: "Fixture Serif".into(),
        subfamily: "Regular".into(),
        full_name: "Fixture Serif Regular".into(),
        weight: 400,
        is_italic: false,
        file_path: PathBuf::from(&font_file),
        source: FontSource::Local,
        source_family_id: None,
        source_variant_id: None,
        category: FontCategory::Serif,
        is_active: false,
        is_installed: false,
        is_favorite: false,
        file_size: specimen.len() as u64,
        file_modified_unix_ms: 0,
        missing_on_disk: false,
    };
    db::upsert_font(&backend.paths().database, &item).unwrap();

    let query = FontQuery {
        tab: LibraryTab::Local,
        search: "Fixture".into(),
        category: FontCategory::All,
        collection_id: None,
        page: 0,
        page_size: 50,
    };
    let page = backend.list_fonts(query.clone()).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].id, "fixture-regular");

    backend.set_font_favorite("fixture-regular", true).unwrap();
    let collection = backend.create_collection("Editorial").unwrap();
    backend
        .add_font_to_collection(collection.id, "fixture-regular")
        .unwrap();
    let collection_page = backend
        .list_fonts(FontQuery {
            collection_id: Some(collection.id),
            ..query
        })
        .unwrap();

    assert_eq!(collection_page.total, 1);
    assert!(collection_page.items[0].is_favorite);
    assert_eq!(
        backend.font_preview_bytes("fixture-regular").unwrap(),
        specimen
    );
}

#[test]
fn backend_marks_existing_system_font_files_as_installed() {
    let data = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let windows_fonts = std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:/Windows"))
        .join("Fonts");
    let source = ["segoeui.ttf", "arial.ttf", "calibri.ttf", "times.ttf"]
        .iter()
        .map(|name| windows_fonts.join(name))
        .find(|path| path.is_file())
        .expect("a Windows system font is available");
    let backend = AppBackend::from_roots(data.path(), cache.path()).unwrap();
    let item = FontItem {
        id: "system-font-fixture".into(),
        family: "System Fixture".into(),
        subfamily: "Regular".into(),
        full_name: "System Fixture Regular".into(),
        weight: 400,
        is_italic: false,
        file_path: source.clone(),
        source: FontSource::Local,
        source_family_id: None,
        source_variant_id: None,
        category: FontCategory::Serif,
        is_active: false,
        is_installed: false,
        is_favorite: false,
        file_size: fs::metadata(&source).unwrap().len(),
        file_modified_unix_ms: 0,
        missing_on_disk: false,
    };
    db::upsert_font(&backend.paths().database, &item).unwrap();

    let page = backend
        .list_fonts(FontQuery {
            tab: LibraryTab::Local,
            search: String::new(),
            category: FontCategory::All,
            collection_id: None,
            page: 0,
            page_size: 100,
        })
        .unwrap();

    assert!(page.items[0].is_installed);

    let active_page = backend
        .list_fonts(FontQuery {
            tab: LibraryTab::Active,
            search: String::new(),
            category: FontCategory::All,
            collection_id: None,
            page: 0,
            page_size: 100,
        })
        .unwrap();
    assert_eq!(active_page.total, 1);
    assert!(active_page.items[0].is_installed);
    assert!(backend
        .set_font_active("system-font-fixture", false)
        .unwrap_err()
        .contains("già installato in Windows"));
}

#[test]
fn startup_snapshot_uses_the_existing_app_data_roots_without_network_access() {
    let data = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let backend = AppBackend::from_roots(data.path(), cache.path()).unwrap();

    let snapshot = backend.startup_snapshot().unwrap();

    assert_eq!(snapshot.fonts.total, 0);
    assert!(snapshot.collections.is_empty());
    assert_eq!(snapshot.data_directory, data.path());
    assert!(snapshot.google_catalog.is_none());
}

#[test]
fn installing_and_uninstalling_font_changes_windows_registration_and_keeps_library_file() {
    let data = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let install_dir = data.path().join("windows-fonts");
    let source = data.path().join("specimen.ttf");
    let specimen = b"deterministic font fixture";
    fs::write(&source, specimen).unwrap();
    let registration = FakeFontRegistration::default();
    let backend = AppBackend::from_roots_with_registration(
        data.path(),
        cache.path(),
        &install_dir,
        Arc::new(registration.clone()),
    )
    .unwrap();
    let item = FontItem {
        id: "fixture-font".into(),
        family: "Fixture Serif".into(),
        subfamily: "Regular".into(),
        full_name: "Fixture Serif Regular".into(),
        weight: 400,
        is_italic: false,
        file_path: source.clone(),
        source: FontSource::Local,
        source_family_id: None,
        source_variant_id: None,
        category: FontCategory::Serif,
        is_active: false,
        is_installed: false,
        is_favorite: false,
        file_size: specimen.len() as u64,
        file_modified_unix_ms: 0,
        missing_on_disk: false,
    };
    db::upsert_font(&backend.paths().database, &item).unwrap();

    backend.set_font_active("fixture-font", true).unwrap();

    let installed_path = registration
        .0
        .lock()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .clone();
    assert!(installed_path.starts_with(&install_dir));
    assert_eq!(fs::read(&installed_path).unwrap(), specimen);
    assert!(
        db::font_by_id(&backend.paths().database, "fixture-font")
            .unwrap()
            .is_active
    );

    backend.set_font_active("fixture-font", false).unwrap();

    assert!(!installed_path.exists());
    assert!(registration.0.lock().unwrap().is_empty());
    assert!(
        !db::font_by_id(&backend.paths().database, "fixture-font")
            .unwrap()
            .is_active
    );
    assert!(source.is_file());
}

#[test]
fn imports_font_files_from_a_zip_into_the_local_library() {
    let data = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let windows_fonts = std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:/Windows"))
        .join("Fonts");
    let font_path = ["segoeui.ttf", "arial.ttf", "calibri.ttf", "times.ttf"]
        .iter()
        .map(|name| windows_fonts.join(name))
        .find(|path| path.is_file())
        .expect("a Windows system font is available for the ZIP fixture");
    let font_bytes = fs::read(font_path).unwrap();
    let archive_path = data.path().join("specimen.zip");
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    archive.start_file("Fonts/Specimen.TTF", options).unwrap();
    archive.write_all(&font_bytes).unwrap();
    archive
        .start_file("__MACOSX/._Specimen.TTF", options)
        .unwrap();
    archive.write_all(&font_bytes).unwrap();
    archive.start_file("fonts/Specimen.woff2", options).unwrap();
    archive
        .write_all(b"not an installable desktop font")
        .unwrap();
    archive.start_file("../../../escaped.ttf", options).unwrap();
    archive.write_all(&font_bytes).unwrap();
    archive.finish().unwrap();

    let backend = AppBackend::from_roots(data.path(), cache.path()).unwrap();
    let report = backend.import_zip_font(&archive_path).unwrap();
    let page = backend
        .list_fonts(FontQuery {
            tab: LibraryTab::Local,
            search: String::new(),
            category: FontCategory::All,
            collection_id: None,
            page: 0,
            page_size: 50,
        })
        .unwrap();

    assert_eq!(report.imported, 1, "{report:?}");
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].source, FontSource::Local);
    assert!(!page.items[0].is_active);
    let imported_path = PathBuf::from(&page.items[0].file_path);
    assert!(imported_path.is_file());
    assert!(fs::canonicalize(imported_path)
        .unwrap()
        .starts_with(fs::canonicalize(data.path()).unwrap()));
    assert!(!data.path().join("escaped.ttf").exists());
}

#[test]
fn rejects_a_zip_that_contains_no_valid_desktop_fonts() {
    let data = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let archive_path = data.path().join("unsupported.zip");
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    archive.start_file("webfont.woff2", options).unwrap();
    archive.write_all(b"not a desktop font").unwrap();
    archive.start_file("README.txt", options).unwrap();
    archive.write_all(b"license and documentation").unwrap();
    archive.finish().unwrap();

    let backend = AppBackend::from_roots(data.path(), cache.path()).unwrap();
    let error = backend.import_zip_font(&archive_path).unwrap_err();

    assert!(error.contains("non contiene font TTF o OTF validi"));
    let imports = data.path().join("imported-fonts");
    assert!(imports.is_dir());
    assert_eq!(fs::read_dir(imports).unwrap().count(), 0);
}
