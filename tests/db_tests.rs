use std::path::{Path, PathBuf};

use rust_font_manager::{
    db,
    models::{
        Collection, FontActivation, FontCategory, FontItem, FontQuery, FontSource, LibraryTab,
        UpsertOutcome,
    },
};
use tempfile::TempDir;

fn temp_db() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("library.db");
    (dir, path)
}

fn font(id: &str, family: &str, path: &Path) -> FontItem {
    FontItem {
        id: id.to_owned(),
        family: family.to_owned(),
        subfamily: "Regular".to_owned(),
        full_name: format!("{family} Regular"),
        weight: 400,
        is_italic: false,
        file_path: path.to_path_buf(),
        source: FontSource::Local,
        source_family_id: None,
        source_variant_id: None,
        category: FontCategory::Unknown,
        is_active: false,
        is_favorite: false,
        file_size: 1024,
        file_modified_unix_ms: 100,
        missing_on_disk: false,
    }
}

fn query(tab: LibraryTab, search: &str, page: u32, page_size: u32) -> FontQuery {
    FontQuery {
        tab,
        search: search.to_owned(),
        category: FontCategory::All,
        collection_id: None,
        page,
        page_size,
    }
}

#[test]
fn migration_is_idempotent_and_enables_foreign_keys() {
    let (_dir, path) = temp_db();

    db::migrate(&path).expect("first migration");
    db::migrate(&path).expect("repeat migration");

    let conn = db::open_connection(&path).expect("open configured connection");
    let foreign_keys: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .expect("read foreign key setting");
    let tables: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('fonts','collections','collection_items','scan_roots','schema_migrations')",
            [],
            |row| row.get(0),
        )
        .expect("count schema tables");

    assert_eq!(foreign_keys, 1);
    assert_eq!(tables, 5);
}

#[test]
fn query_search_is_case_insensitive_and_pages_results() {
    let (_dir, path) = temp_db();
    db::migrate(&path).expect("migrate");
    for index in 0..55 {
        let item = font(
            &format!("id-{index}"),
            if index == 0 { "Roboto" } else { "Inter" },
            &PathBuf::from(format!("C:/fonts/{index}.ttf")),
        );
        db::upsert_font(&path, &item).expect("insert font");
    }

    let first =
        db::list_fonts(&path, query(LibraryTab::Local, "INTER", 0, 50)).expect("first page");
    let second =
        db::list_fonts(&path, query(LibraryTab::Local, "inter", 1, 50)).expect("second page");

    assert_eq!(first.total, 54);
    assert_eq!(first.items.len(), 50);
    assert_eq!(second.items.len(), 4);
    assert_eq!(second.page, 1);
    assert!(first.items.iter().all(|item| item.family == "Inter"));
}

#[test]
fn query_rejects_page_sizes_outside_one_to_one_hundred() {
    let (_dir, path) = temp_db();
    db::migrate(&path).expect("migrate");

    assert!(db::list_fonts(&path, query(LibraryTab::Local, "", 0, 0)).is_err());
    assert!(db::list_fonts(&path, query(LibraryTab::Local, "", 0, 101)).is_err());
}

#[test]
fn upsert_preserves_user_state_and_reports_insert_then_update() {
    let (_dir, path) = temp_db();
    db::migrate(&path).expect("migrate");
    let path1 = PathBuf::from("C:/fonts/roboto.ttf");
    let mut item = font("font-hash", "Roboto", &path1);

    assert_eq!(
        db::upsert_font(&path, &item).expect("insert"),
        UpsertOutcome::Inserted
    );
    db::set_font_active(&path, "font-hash", true).expect("activate");
    db::set_font_favorite(&path, "font-hash", true).expect("favorite");
    item.subfamily = "Bold".to_owned();
    item.weight = 700;

    assert_eq!(
        db::upsert_font(&path, &item).expect("update"),
        UpsertOutcome::Updated
    );
    let loaded = db::list_fonts(&path, query(LibraryTab::Local, "Roboto", 0, 50))
        .expect("load")
        .items
        .remove(0);
    assert_eq!(loaded.subfamily, "Bold");
    assert_eq!(loaded.weight, 700);
    assert!(loaded.is_active);
    assert!(loaded.is_favorite);
    assert!(db::upsert_font(&path, &item).expect("unchanged") == UpsertOutcome::Unchanged);
}

#[test]
fn collection_names_are_case_insensitive_and_duplicate_membership_is_idempotent() {
    let (_dir, path) = temp_db();
    db::migrate(&path).expect("migrate");
    let collection = db::create_collection(&path, "Work").expect("create collection");
    let duplicate_name = db::create_collection(&path, "work");
    assert!(duplicate_name.is_err());
    db::upsert_font(
        &path,
        &font("font-id", "Inter", Path::new("C:/fonts/inter.ttf")),
    )
    .expect("insert font");

    db::add_font_to_collection(&path, collection.id, "font-id").expect("add item");
    db::add_font_to_collection(&path, collection.id, "font-id").expect("duplicate add is harmless");
    let contents = db::list_fonts(
        &path,
        FontQuery {
            tab: LibraryTab::Local,
            search: String::new(),
            category: FontCategory::All,
            collection_id: Some(collection.id),
            page: 0,
            page_size: 50,
        },
    )
    .expect("collection contents");
    assert_eq!(contents.items.len(), 1);

    db::delete_collection(&path, collection.id).expect("delete collection");
    let remaining = db::list_collections(&path).expect("list collections");
    assert!(remaining.is_empty());
    let conn = db::open_connection(&path).expect("connection");
    let links: i64 = conn
        .query_row("SELECT COUNT(*) FROM collection_items", [], |row| {
            row.get(0)
        })
        .expect("count links");
    assert_eq!(links, 0);
}

#[test]
fn deleting_font_cascades_collection_membership() {
    let (_dir, path) = temp_db();
    db::migrate(&path).expect("migrate");
    let collection: Collection = db::create_collection(&path, "Brand").expect("create");
    db::upsert_font(
        &path,
        &font("font-id", "Inter", Path::new("C:/fonts/inter.ttf")),
    )
    .expect("insert");
    db::add_font_to_collection(&path, collection.id, "font-id").expect("link");

    db::delete_font(&path, "font-id").expect("delete font");

    let conn = db::open_connection(&path).expect("connection");
    let links: i64 = conn
        .query_row("SELECT COUNT(*) FROM collection_items", [], |row| {
            row.get(0)
        })
        .expect("count links");
    assert_eq!(links, 0);
}

#[test]
fn missing_file_refresh_only_marks_local_fonts_under_the_given_root() {
    let (_dir, path) = temp_db();
    db::migrate(&path).expect("migrate");
    let root = PathBuf::from("C:/fonts");
    db::upsert_font(&path, &font("missing", "Old", &root.join("old.ttf"))).expect("local");
    let mut google = font("google", "Remote", Path::new("C:/vault/remote.ttf"));
    google.source = FontSource::Google;
    db::upsert_font(&path, &google).expect("google");

    db::mark_root_missing_files(&path, &root, &[]).expect("mark missing");
    let local = db::list_fonts(&path, query(LibraryTab::Local, "Old", 0, 50))
        .expect("local")
        .items
        .remove(0);
    assert!(local.missing_on_disk);
    let cloud = db::list_fonts(&path, query(LibraryTab::Google, "Remote", 0, 50))
        .expect("google")
        .items
        .remove(0);
    assert!(!cloud.missing_on_disk);
}

#[test]
fn activation_and_favorite_updates_require_existing_font() {
    let (_dir, path) = temp_db();
    db::migrate(&path).expect("migrate");

    assert!(db::set_font_active(&path, "absent", true).is_err());
    assert!(db::set_font_favorite(&path, "absent", true).is_err());
    assert_eq!(FontActivation::Active, FontActivation::Active);
}
