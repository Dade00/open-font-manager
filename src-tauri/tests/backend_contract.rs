use std::{fs, path::PathBuf};

use open_font_manager_lib::AppBackend;
use rust_font_manager::{
    db,
    models::{FontCategory, FontItem, FontQuery, FontSource, LibraryTab},
};
use tempfile::tempdir;

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
