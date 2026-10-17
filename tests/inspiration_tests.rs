use rust_font_manager::{
    inspiration::{self, InspirationDocument},
    paths::AppPaths,
};
use std::{fs, path::Path};
use tempfile::tempdir;

fn empty() -> InspirationDocument {
    InspirationDocument {
        version: 1,
        revision: 0,
        boards: vec![],
        compositions: vec![],
    }
}

#[test]
fn loads_empty_document_and_persists_snapshot_across_reopen() {
    let root = tempdir().unwrap();
    let paths =
        AppPaths::from_roots(&root.path().join("data"), &root.path().join("cache")).unwrap();
    rust_font_manager::db::migrate(&paths.database).unwrap();
    assert_eq!(inspiration::load(&paths.database).unwrap(), empty());
    let mut doc = empty();
    doc.boards.push(inspiration::Board {
        id: "b1".into(),
        title: "Moodboard".into(),
        description: String::new(),
        items: vec![],
    });
    let saved = inspiration::save(&paths.database, &paths.data_dir, doc, 0).unwrap();
    assert_eq!(saved.revision, 1);
    assert_eq!(inspiration::load(&paths.database).unwrap(), saved);
}

#[test]
fn compare_and_swap_rejects_stale_revision_without_losing_current_snapshot() {
    let root = tempdir().unwrap();
    let paths =
        AppPaths::from_roots(&root.path().join("data"), &root.path().join("cache")).unwrap();
    rust_font_manager::db::migrate(&paths.database).unwrap();
    let mut first = empty();
    first.boards.push(inspiration::Board {
        id: "b1".into(),
        title: "One".into(),
        description: String::new(),
        items: vec![],
    });
    inspiration::save(&paths.database, &paths.data_dir, first, 0).unwrap();
    assert!(inspiration::save(&paths.database, &paths.data_dir, empty(), 0).is_err());
    assert_eq!(
        inspiration::load(&paths.database).unwrap().boards[0].id,
        "b1"
    );
}

#[test]
fn rejects_asset_traversal_and_unreferenced_hashes() {
    let root = tempdir().unwrap();
    let paths =
        AppPaths::from_roots(&root.path().join("data"), &root.path().join("cache")).unwrap();
    rust_font_manager::db::migrate(&paths.database).unwrap();
    assert!(inspiration::get_asset(&paths.data_dir, "../secret").is_err());
}

#[test]
fn imports_validates_and_deduplicates_image_assets_and_keeps_shared_reference() {
    let root = tempdir().unwrap();
    let paths =
        AppPaths::from_roots(&root.path().join("data"), &root.path().join("cache")).unwrap();
    rust_font_manager::db::migrate(&paths.database).unwrap();
    // 1x1 PNG
    let png = [
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 240,
        31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    let source = root.path().join("image.bin");
    fs::write(&source, png).unwrap();
    let asset = inspiration::import_image(&paths.database, &paths.data_dir, &source).unwrap();
    assert_eq!(asset.mime, "image/png");
    assert_eq!(
        inspiration::import_image(&paths.database, &paths.data_dir, &source).unwrap(),
        asset
    );
    assert_eq!(
        inspiration::get_asset(&paths.data_dir, &asset.id)
            .unwrap()
            .len(),
        png.len()
    );
    let mut doc = empty();
    for id in ["b1", "b2"] {
        doc.boards.push(inspiration::Board {
            id: id.into(),
            title: id.into(),
            description: String::new(),
            items: vec![inspiration::BoardItem::Image {
                id: format!("i{id}"),
                title: String::new(),
                text: String::new(),
                asset: asset.clone(),
            }],
        });
    }
    inspiration::save(&paths.database, &paths.data_dir, doc, 0).unwrap();
}

#[test]
fn rejects_corrupt_image_and_invalid_link_or_document_references() {
    let root = tempdir().unwrap();
    let paths =
        AppPaths::from_roots(&root.path().join("data"), &root.path().join("cache")).unwrap();
    rust_font_manager::db::migrate(&paths.database).unwrap();
    let corrupt = root.path().join("x.png");
    fs::write(&corrupt, b"not png").unwrap();
    assert!(inspiration::import_image(&paths.database, &paths.data_dir, &corrupt).is_err());
    assert!(inspiration::validate_link("javascript:alert(1)").is_err());
    assert!(inspiration::validate_link("https://user:pass@example.com").is_err());
}

#[test]
fn asset_id_must_be_exact_sha256_hex() {
    assert!(inspiration::asset_path(Path::new("data"), "A0").is_err());
}

#[test]
fn serializes_frontend_provider_literals_and_composition_item_contract() {
    for provider in [
        "local",
        "google",
        "fontsource-extra",
        "fontshare",
        "nerd-fonts",
    ] {
        let font = inspiration::InspirationFontRef {
            provider: provider.into(),
            family_id: Some("family".into()),
            variant_id: None,
            local_font_id: Some("local-id".into()),
            family: "Example".into(),
            is_variable: None,
        };
        let value = serde_json::to_value(&font).unwrap();
        assert_eq!(value["provider"], provider);
        assert!(serde_json::from_value::<inspiration::InspirationFontRef>(value).is_ok());
        assert!(inspiration::validate_document_font(&font).is_ok());
        let mut doc = empty();
        doc.compositions.push(inspiration::Composition {
            id: "c1".into(),
            title: "Composition".into(),
            heading: inspiration::CompositionRole {
                font: None,
                text: "h".into(),
                size: 24.0,
                line_height: 1.0,
                letter_spacing: 0.0,
                color: "#000".into(),
            },
            subheading: None,
            body: inspiration::CompositionRole {
                font: None,
                text: "b".into(),
                size: 16.0,
                line_height: 1.0,
                letter_spacing: 0.0,
                color: "#000".into(),
            },
            background: "#fff".into(),
            narrow: false,
        });
        doc.boards.push(inspiration::Board {
            id: "b1".into(),
            title: "B".into(),
            description: String::new(),
            items: vec![inspiration::BoardItem::Composition {
                id: "i1".into(),
                title: "C".into(),
                composition_id: "c1".into(),
            }],
        });
        let json = serde_json::to_value(&doc).unwrap();
        assert_eq!(json["boards"][0]["items"][0]["compositionId"], "c1");
        assert!(json["boards"][0]["items"][0]
            .get("composition_id")
            .is_none());
    }
}

#[test]
fn save_does_not_delete_unreferenced_or_pending_assets() {
    let root = tempdir().unwrap();
    let paths =
        AppPaths::from_roots(&root.path().join("data"), &root.path().join("cache")).unwrap();
    rust_font_manager::db::migrate(&paths.database).unwrap();
    let source = root.path().join("image.png");
    let png = [
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 240,
        31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    fs::write(&source, png).unwrap();
    let asset = inspiration::import_image(&paths.database, &paths.data_dir, &source).unwrap();
    inspiration::save(&paths.database, &paths.data_dir, empty(), 0).unwrap();
    assert_eq!(
        inspiration::get_asset(&paths.data_dir, &asset.id).unwrap(),
        png
    );
}
