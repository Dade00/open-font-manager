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
        is_variable: false,
        file_path: path.to_path_buf(),
        source: FontSource::Local,
        source_family_id: None,
        source_variant_id: None,
        category: FontCategory::Unknown,
        is_active: false,
        is_installed: false,
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
fn installed_stream_keeps_filters_pagination_and_live_external_state() {
    let (dir, path) = temp_db();
    db::migrate(&path).unwrap();
    let root = dir.path().join("WindowsFonts");
    std::fs::create_dir(&root).unwrap();
    for index in 0..3 {
        let source = root.join(format!("{index}.ttf"));
        std::fs::write(&source, b"fixture").unwrap();
        let mut item = font(
            &format!("external-{index}"),
            &format!("External {index}"),
            &source,
        );
        item.category = FontCategory::Serif;
        db::upsert_font(&path, &item).unwrap();
    }
    let roots = vec![root.clone()];
    let mut filter = query(LibraryTab::Active, "External", 1, 1);
    filter.category = FontCategory::Serif;
    let page = db::list_installed_fonts(&path, filter.clone(), &roots).unwrap();
    assert_eq!(page.total, 3);
    assert_eq!(page.items[0].id, "external-1");
    assert!(page.items[0].is_installed);
    filter.search = "Does not exist".into();
    assert_eq!(
        db::list_installed_fonts(&path, filter.clone(), &roots)
            .unwrap()
            .total,
        0
    );
    filter.search.clear();
    filter.category = FontCategory::Unknown;
    assert_eq!(
        db::list_installed_fonts(&path, filter.clone(), &roots)
            .unwrap()
            .total,
        0
    );
    filter.category = FontCategory::All;
    let collection = db::create_collection(&path, "Filtered").unwrap();
    db::add_font_to_collection(&path, collection.id, "external-2").unwrap();
    filter.collection_id = Some(collection.id);
    filter.page = 0;
    assert_eq!(
        db::list_installed_fonts(&path, filter.clone(), &roots)
            .unwrap()
            .items[0]
            .id,
        "external-2"
    );
    std::fs::remove_file(root.join("2.ttf")).unwrap();
    assert_eq!(
        db::list_installed_fonts(&path, filter, &roots)
            .unwrap()
            .total,
        0
    );
}

#[test]
fn variable_metadata_and_handwriting_category_round_trip() {
    let (dir, path) = temp_db();
    db::migrate(&path).unwrap();
    let mut item = font(
        "variable",
        "Variable Family",
        &dir.path().join("variable.ttf"),
    );
    item.is_variable = true;
    item.category = FontCategory::Handwriting;
    db::upsert_font(&path, &item).unwrap();
    db::migrate(&path).unwrap();
    let stored = db::font_by_id(&path, &item.id).unwrap();
    assert!(stored.is_variable);
    assert_eq!(stored.category, FontCategory::Handwriting);
    db::open_connection(&path)
        .unwrap()
        .execute("DELETE FROM schema_migrations WHERE version=5", [])
        .unwrap();
    db::migrate(&path).unwrap();
    assert!(db::font_by_id(&path, &item.id).unwrap().is_variable);
    let mut filter = query(LibraryTab::Local, "", 0, 100);
    filter.category = FontCategory::Unknown;
    assert_eq!(db::list_fonts(&path, filter).unwrap().total, 0);
}

#[test]
fn local_library_includes_installed_google_fonts_without_changing_google_filter() {
    let (_dir, path) = temp_db();
    db::migrate(&path).expect("migrate");
    let local = font(
        "local-font",
        "Local Family",
        Path::new("C:/fonts/local.ttf"),
    );
    let mut google = font(
        "google-font",
        "Google Family",
        Path::new("C:/vault/google.ttf"),
    );
    google.source = FontSource::Google;
    google.source_family_id = Some("google-family".into());
    google.source_variant_id = Some("regular".into());
    db::upsert_font(&path, &local).expect("insert local font");
    db::upsert_font(&path, &google).expect("insert installed Google font");

    let local_page =
        db::list_fonts(&path, query(LibraryTab::Local, "", 0, 50)).expect("list local library");
    assert_eq!(local_page.total, 2);
    assert!(local_page.items.iter().any(|item| item.id == "google-font"));

    let google_page =
        db::list_fonts(&path, query(LibraryTab::Google, "", 0, 50)).expect("list Google source");
    assert_eq!(google_page.total, 1);
    assert_eq!(google_page.items[0].id, "google-font");
}

#[test]
fn cloud_provider_source_persists_through_database_migration() {
    let (_dir, path) = temp_db();
    db::migrate(&path).expect("migrate");
    let cases = [
        ("fontsource", FontSource::Fontsource),
        ("fontshare", FontSource::Fontshare),
        ("nerd-fonts", FontSource::NerdFonts),
    ];
    for (provider, source) in cases {
        let id = format!("{provider}-id");
        let mut item = font(
            &id,
            "Example Family",
            &PathBuf::from(format!("C:/vault/{provider}.ttf")),
        );
        item.source = source;
        item.source_family_id = Some("example-family".into());
        item.source_variant_id = Some("400-normal".into());

        db::upsert_font(&path, &item).expect("insert cloud font");
        let stored = db::font_by_id(&path, &id).expect("read cloud font");
        assert_eq!(stored.source, source);
        assert_eq!(stored.source_family_id.as_deref(), Some("example-family"));
    }
}

#[test]
fn removing_or_disabling_scan_root_preserves_indexed_fonts() {
    let (_dir, path) = temp_db();
    db::migrate(&path).expect("migrate");
    let root = Path::new("C:/fonts/extra");
    let item = font("root-font", "Root Family", &root.join("root.ttf"));
    db::add_scan_root(&path, root).expect("add root");
    db::upsert_font(&path, &item).expect("index font");

    db::set_scan_root_enabled(&path, root, false).expect("disable root");
    assert!(!db::list_scan_roots(&path).expect("list disabled root")[0].is_enabled);
    db::set_scan_root_enabled(&path, root, true).expect("enable root");
    assert!(db::list_scan_roots(&path).expect("list enabled root")[0].is_enabled);
    db::set_scan_root_enabled(&path, root, false).expect("disable root again");
    db::remove_scan_root(&path, root).expect("remove root");

    assert!(db::list_scan_roots(&path).expect("list roots").is_empty());
    assert_eq!(
        db::font_by_id(&path, "root-font")
            .expect("font remains")
            .family,
        "Root Family"
    );
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
fn migration_v1_preserves_fonts_and_collection_memberships() {
    let (_dir, path) = temp_db();
    let legacy = rusqlite::Connection::open(&path).expect("create legacy database");
    legacy
        .execute_batch(
            "CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,applied_at_unix_ms INTEGER NOT NULL);
             INSERT INTO schema_migrations VALUES(1,1);
             CREATE TABLE scan_roots(path TEXT PRIMARY KEY,added_at_unix_ms INTEGER NOT NULL,last_scanned_at_unix_ms INTEGER,is_enabled INTEGER NOT NULL DEFAULT 1 CHECK(is_enabled IN (0,1)));
             CREATE TABLE fonts(id TEXT PRIMARY KEY,family TEXT NOT NULL,subfamily TEXT NOT NULL,full_name TEXT NOT NULL,weight INTEGER NOT NULL CHECK(weight BETWEEN 1 AND 1000),is_italic INTEGER NOT NULL CHECK(is_italic IN (0,1)),file_path TEXT NOT NULL UNIQUE,source TEXT NOT NULL CHECK(source IN ('local','google')),source_family_id TEXT,source_variant_id TEXT,category TEXT NOT NULL,is_active INTEGER NOT NULL DEFAULT 0 CHECK(is_active IN (0,1)),is_favorite INTEGER NOT NULL DEFAULT 0 CHECK(is_favorite IN (0,1)),file_size INTEGER NOT NULL,file_modified_unix_ms INTEGER NOT NULL,missing_on_disk INTEGER NOT NULL DEFAULT 0 CHECK(missing_on_disk IN (0,1)),created_at_unix_ms INTEGER NOT NULL,updated_at_unix_ms INTEGER NOT NULL);
             CREATE TABLE collections(id INTEGER PRIMARY KEY,name TEXT NOT NULL UNIQUE COLLATE NOCASE,created_at_unix_ms INTEGER NOT NULL);
             CREATE TABLE collection_items(collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,font_id TEXT NOT NULL REFERENCES fonts(id) ON DELETE CASCADE,PRIMARY KEY(collection_id,font_id));
             INSERT INTO fonts VALUES('font-old','Inter','Regular','Inter Regular',400,0,'C:/fonts/inter.ttf','local',NULL,NULL,'unknown',1,1,1024,100,0,10,10);
             INSERT INTO collections VALUES(1,'Saved',10);
             INSERT INTO collection_items VALUES(1,'font-old');",
        )
        .expect("write v1 schema");
    drop(legacy);

    db::migrate(&path).expect("upgrade schema");

    let fonts = db::list_fonts(&path, query(LibraryTab::Local, "Inter", 0, 50))
        .expect("read migrated font");
    let collections = db::list_collections(&path).expect("read migrated collection");
    assert_eq!(fonts.total, 1);
    assert!(fonts.items[0].is_active);
    assert!(fonts.items[0].is_favorite);
    assert_eq!(collections[0].font_count, 1);
    let mut second_face = font("font-new", "Inter", Path::new("C:/fonts/inter.ttf"));
    second_face.source_variant_id = Some("ttc-face-1".into());
    assert_eq!(
        db::upsert_font(&path, &second_face).expect("insert second face"),
        UpsertOutcome::Inserted
    );
}

#[test]
fn migration_v2_preserves_local_google_fonts_flags_and_foreign_keys() {
    let (_dir, path) = temp_db();
    let legacy = rusqlite::Connection::open(&path).expect("create v2 database");
    legacy
        .execute_batch(
            "CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,applied_at_unix_ms INTEGER NOT NULL);
             INSERT INTO schema_migrations VALUES(2,2);
             CREATE TABLE scan_roots(path TEXT PRIMARY KEY,added_at_unix_ms INTEGER NOT NULL,last_scanned_at_unix_ms INTEGER,is_enabled INTEGER NOT NULL DEFAULT 1 CHECK(is_enabled IN (0,1)));
             CREATE TABLE fonts(id TEXT PRIMARY KEY,family TEXT NOT NULL,subfamily TEXT NOT NULL,full_name TEXT NOT NULL,weight INTEGER NOT NULL CHECK(weight BETWEEN 1 AND 1000),is_italic INTEGER NOT NULL CHECK(is_italic IN (0,1)),file_path TEXT NOT NULL,source TEXT NOT NULL CHECK(source IN ('local','google')),source_family_id TEXT,source_variant_id TEXT,category TEXT NOT NULL,is_active INTEGER NOT NULL DEFAULT 0 CHECK(is_active IN (0,1)),is_favorite INTEGER NOT NULL DEFAULT 0 CHECK(is_favorite IN (0,1)),file_size INTEGER NOT NULL,file_modified_unix_ms INTEGER NOT NULL,missing_on_disk INTEGER NOT NULL DEFAULT 0 CHECK(missing_on_disk IN (0,1)),created_at_unix_ms INTEGER NOT NULL,updated_at_unix_ms INTEGER NOT NULL);
             CREATE TABLE collections(id INTEGER PRIMARY KEY,name TEXT NOT NULL UNIQUE COLLATE NOCASE,created_at_unix_ms INTEGER NOT NULL);
             CREATE TABLE collection_items(collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,font_id TEXT NOT NULL REFERENCES fonts(id) ON DELETE CASCADE,PRIMARY KEY(collection_id,font_id));
             INSERT INTO fonts VALUES('local-old','Inter','Regular','Inter Regular',400,0,'C:/fonts/inter.ttf','local',NULL,NULL,'unknown',1,0,1024,100,0,10,10);
             INSERT INTO fonts VALUES('google-old','Roboto','Regular','Roboto Regular',400,0,'C:/vault/roboto.ttf','google','roboto','regular','sans-serif',0,1,2048,200,0,20,20);
             INSERT INTO collections VALUES(1,'Saved',10);
             INSERT INTO collection_items VALUES(1,'google-old');",
        )
        .expect("write v2 schema and records");
    drop(legacy);

    db::migrate(&path).expect("upgrade v2 schema");

    let local = db::font_by_id(&path, "local-old").expect("read local font");
    let google = db::font_by_id(&path, "google-old").expect("read Google font");
    assert_eq!(local.source, FontSource::Local);
    assert!(local.is_active);
    assert_eq!(google.source, FontSource::Google);
    assert!(google.is_favorite);
    assert_eq!(google.source_family_id.as_deref(), Some("roboto"));
    assert_eq!(db::list_collections(&path).unwrap()[0].font_count, 1);

    let connection = db::open_connection(&path).expect("open migrated database");
    let violations: i64 = connection
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .expect("check foreign keys");
    assert_eq!(violations, 0);
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
fn font_api_lookup_returns_serializable_camel_case_font_and_rejects_missing_id() {
    let (_dir, path) = temp_db();
    db::migrate(&path).expect("migrate");
    let item = font("font-api", "Inter", Path::new("C:/fonts/inter.ttf"));
    db::upsert_font(&path, &item).expect("insert font");

    let loaded = db::font_by_id(&path, "font-api").expect("load font");
    assert_eq!(loaded, item);
    let serialized = serde_json::to_value(&loaded).expect("serialize Tauri response");
    assert_eq!(serialized["filePath"], "C:/fonts/inter.ttf");
    assert_eq!(serialized["source"], "local");
    let page = db::list_fonts(&path, query(LibraryTab::Local, "Inter", 0, 50))
        .expect("load serializable font page");
    let serialized_page = serde_json::to_value(page).expect("serialize font page");
    assert_eq!(serialized_page["pageSize"], 50);
    let collection = db::create_collection(&path, "Brand").expect("create collection");
    let serialized_collection = serde_json::to_value(collection).expect("serialize collection");
    assert_eq!(serialized_collection["fontCount"], 0);
    assert!(db::font_by_id(&path, "missing").is_err());
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
fn database_stores_multiple_faces_from_the_same_collection_file() {
    let (_dir, path) = temp_db();
    db::migrate(&path).expect("migrate");
    let collection_path = Path::new("C:/fonts/cambria.ttc");
    let mut regular = font("cambria-regular", "Cambria", collection_path);
    regular.source_variant_id = Some("ttc-face-0".into());
    let mut italic = font("cambria-italic", "Cambria", collection_path);
    italic.subfamily = "Italic".into();
    italic.full_name = "Cambria Italic".into();
    italic.is_italic = true;
    italic.source_variant_id = Some("ttc-face-1".into());

    assert_eq!(
        db::upsert_font(&path, &regular).expect("insert regular face"),
        UpsertOutcome::Inserted
    );
    assert_eq!(
        db::upsert_font(&path, &italic).expect("insert italic face"),
        UpsertOutcome::Inserted
    );
    let page = db::list_fonts(&path, query(LibraryTab::Local, "Cambria", 0, 50))
        .expect("list collection faces");

    assert_eq!(page.total, 2);
    assert_eq!(page.items[0].file_path, page.items[1].file_path);
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

#[test]
fn active_font_restore_query_pages_past_one_hundred_and_counts_existing_files_only() {
    let (dir, path) = temp_db();
    db::migrate(&path).expect("migrate");
    let existing_path = dir.path().join("present.ttf");
    std::fs::write(&existing_path, b"font placeholder").expect("write font placeholder");

    for index in 0..105 {
        let font_path = if index == 0 {
            existing_path.clone()
        } else {
            PathBuf::from(format!("C:/fonts/{index}.ttf"))
        };
        let id = format!("active-{index}");
        db::upsert_font(&path, &font(&id, "Inter", &font_path)).expect("insert active font");
        db::set_font_active(&path, &id, true).expect("mark active");
    }

    let active = db::list_active_fonts(&path).expect("load all active font records");
    assert_eq!(active.len(), 105);
    assert_eq!(
        db::count_active_fonts(&path).expect("count on-disk active fonts"),
        1
    );
}
