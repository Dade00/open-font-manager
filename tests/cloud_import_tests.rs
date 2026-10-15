use std::{
    fs,
    io::{Cursor, Write},
    path::PathBuf,
};

use rust_font_manager::{
    cloud_fonts::CloudFontsClient,
    db,
    models::{
        FontCategory, FontDownloadStrategy, FontQuery, FontSource, LibraryTab,
        RemoteFontCatalogItem, RemoteFontProvider,
    },
    services::cloud_import::CloudFontImporter,
};
use tempfile::tempdir;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

fn windows_font_bytes() -> Option<Vec<u8>> {
    let fonts = std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:/Windows"))
        .join("Fonts");
    ["segoeui.ttf", "arial.ttf", "calibri.ttf", "times.ttf"]
        .iter()
        .map(|name| fonts.join(name))
        .find(|path| path.is_file())
        .and_then(|path| fs::read(path).ok())
}

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, bytes) in entries {
        writer.start_file(*name, options).expect("start ZIP entry");
        writer.write_all(bytes).expect("write ZIP entry");
    }
    writer.finish().expect("finish ZIP").into_inner()
}

fn nerd_item(id: &str) -> RemoteFontCatalogItem {
    RemoteFontCatalogItem {
        id: id.into(),
        provider: RemoteFontProvider::NerdFonts,
        family: id.into(),
        category: FontCategory::Monospace,
        variants: Vec::new(),
        preview_variant_id: None,
        download_strategy: FontDownloadStrategy::ZipArchive,
        download_url: Some(format!(
            "https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/{id}.zip"
        )),
    }
}

fn fontsource_item() -> RemoteFontCatalogItem {
    RemoteFontCatalogItem {
        id: "adwaita-mono".into(),
        provider: RemoteFontProvider::FontsourceExtra,
        family: "Adwaita Mono".into(),
        category: FontCategory::Monospace,
        variants: Vec::new(),
        preview_variant_id: Some("400-normal".into()),
        download_strategy: FontDownloadStrategy::DirectFiles,
        download_url: None,
    }
}

#[test]
fn imports_validated_zip_fonts_into_family_vault_and_indexes_cloud_source() {
    let Some(font_bytes) = windows_font_bytes() else {
        return;
    };
    let temp = tempdir().expect("temporary directory");
    let vault = temp.path().join("font-vault");
    let database = temp.path().join("library.sqlite3");
    let archive_bytes = archive(&[
        ("Nested/Valid.TTF", &font_bytes),
        ("__MACOSX/._metadata.ttf", &font_bytes),
        ("Windows Compatible/legacy.ttf", &font_bytes),
        ("NerdFontPropo-Regular.ttf", &font_bytes),
        ("webfont.woff2", b"not a desktop font"),
        ("broken.otf", b"invalid font bytes"),
        ("../../../escape.ttf", &font_bytes),
    ]);
    let importer = CloudFontImporter::new(&vault, &database);

    let outcome = importer
        .import_archive(&nerd_item("AdwaitaMono"), &archive_bytes)
        .expect("import archive");
    let page = db::list_fonts(
        &database,
        FontQuery {
            tab: LibraryTab::Local,
            search: String::new(),
            category: FontCategory::All,
            collection_id: None,
            page: 0,
            page_size: 20,
        },
    )
    .expect("query indexed font");

    assert_eq!(outcome.report.saved_files, 1, "{:?}", outcome.report);
    assert_eq!(outcome.report.indexed_fonts, 1, "{:?}", outcome.report);
    assert_eq!(outcome.font_ids.len(), 1);
    assert!(outcome
        .report
        .failures
        .iter()
        .any(|failure| failure.stage == "archive-path"));
    assert!(outcome
        .report
        .failures
        .iter()
        .any(|failure| failure.stage == "validation-or-write"));
    assert_eq!(page.total, 1);
    let font = db::font_by_id(&database, &outcome.font_ids[0]).expect("cloud font record");
    assert_eq!(font.source, FontSource::NerdFonts);
    assert_eq!(font.source_family_id.as_deref(), Some("AdwaitaMono"));
    assert!(font
        .file_path
        .starts_with(fs::canonicalize(vault.join("nerd-fonts").join("AdwaitaMono")).unwrap()));
    assert!(!font.file_path.to_string_lossy().contains("Nested"));
    assert!(font.file_path.is_file());
    assert!(!temp.path().join("escape.ttf").exists());
    assert!(db::list_scan_roots(&database)
        .expect("configured scan roots")
        .is_empty());
}

#[test]
fn imports_fontsource_files_individually_and_keeps_partial_successes() {
    let Some(font_bytes) = windows_font_bytes() else {
        return;
    };
    let temp = tempdir().expect("temporary directory");
    let vault = temp.path().join("font-vault");
    let database = temp.path().join("library.sqlite3");
    let importer = CloudFontImporter::new(&vault, &database);

    let outcome = importer
        .import_fontsource_files(
            &fontsource_item(),
            vec![
                ("400-normal".into(), font_bytes),
                ("700-italic".into(), b"invalid font data".to_vec()),
            ],
        )
        .expect("import direct files");

    assert_eq!(outcome.report.saved_files, 1);
    assert_eq!(outcome.report.indexed_fonts, 1);
    assert_eq!(outcome.report.failures.len(), 1);
    assert_eq!(outcome.report.failures[0].variant_id, "700-italic");
    let font = db::font_by_id(&database, &outcome.font_ids[0]).expect("indexed Fontsource font");
    assert_eq!(font.source, FontSource::Fontsource);
    assert_eq!(font.source_family_id.as_deref(), Some("adwaita-mono"));
    assert!(font.file_path.is_file());
    assert!(!font.file_path.to_string_lossy().contains("400-normal"));
}

#[test]
fn invalid_archive_does_not_create_a_family_directory_or_write_files() {
    let temp = tempdir().expect("temporary directory");
    let vault = temp.path().join("font-vault");
    let importer = CloudFontImporter::new(&vault, temp.path().join("library.sqlite3"));

    assert!(importer
        .import_archive(&nerd_item("FiraCode"), b"not a ZIP archive")
        .is_err());
    assert!(!vault.join("nerd-fonts").join("FiraCode").exists());
}

#[test]
fn archive_without_desktop_fonts_reports_a_reason_and_writes_nothing() {
    let temp = tempdir().expect("temporary directory");
    let vault = temp.path().join("font-vault");
    let importer = CloudFontImporter::new(&vault, temp.path().join("library.sqlite3"));
    let archive_bytes = archive(&[("webfont.woff2", b"not a desktop font")]);

    let outcome = importer
        .import_archive(&nerd_item("FiraCode"), &archive_bytes)
        .expect("report unsupported archive");

    assert_eq!(outcome.report.saved_files, 0);
    assert_eq!(outcome.report.indexed_fonts, 0);
    assert_eq!(outcome.report.failures.len(), 1);
    assert_eq!(outcome.report.failures[0].stage, "archive-content");
    assert!(!vault.join("nerd-fonts").join("FiraCode").exists());
}

#[test]
fn cloud_import_rejects_path_like_family_identifiers_before_touching_the_vault() {
    let temp = tempdir().expect("temporary directory");
    let vault = temp.path().join("font-vault");
    let importer = CloudFontImporter::new(&vault, temp.path().join("library.sqlite3"));

    assert!(importer
        .import_archive(&nerd_item("../outside"), b"ignored")
        .is_err());
    assert!(!vault.exists());
}

#[test]
fn rejects_archives_with_too_many_entries_before_writing_fonts() {
    let temp = tempdir().expect("temporary directory");
    let vault = temp.path().join("font-vault");
    let importer = CloudFontImporter::new(&vault, temp.path().join("library.sqlite3"));
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..8193 {
        writer
            .start_file(format!("{index}.txt"), SimpleFileOptions::default())
            .expect("ZIP entry");
    }
    let bytes = writer.finish().expect("finish ZIP").into_inner();
    assert!(importer
        .import_archive(&nerd_item("FiraCode"), &bytes)
        .is_err());
    assert!(!vault.exists());
}

#[test]
fn symlinks_and_hidden_font_entries_are_not_materialized() {
    let temp = tempdir().expect("temporary directory");
    let vault = temp.path().join("font-vault");
    let importer = CloudFontImporter::new(&vault, temp.path().join("library.sqlite3"));
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .add_symlink(
            "escape.ttf",
            "../../escape.ttf",
            SimpleFileOptions::default(),
        )
        .expect("symlink entry");
    writer
        .start_file(".hidden/font.ttf", SimpleFileOptions::default())
        .expect("hidden entry");
    writer.write_all(b"not a font").unwrap();
    let bytes = writer.finish().expect("finish ZIP").into_inner();
    let result = importer
        .import_archive(&nerd_item("FiraCode"), &bytes)
        .unwrap();
    assert_eq!(result.report.saved_files, 0);
    assert_eq!(result.report.failures[0].stage, "archive-content");
    assert!(!vault.exists());
}

#[test]
fn fontshare_preview_validates_bytes_without_writing_a_vault() {
    let Some(bytes) = windows_font_bytes() else {
        return;
    };
    let temp = tempdir().expect("temporary directory");
    let vault = temp.path().join("font-vault");
    let importer = CloudFontImporter::new(&vault, temp.path().join("library.sqlite3"));
    let mut item = nerd_item("fixture-serif");
    item.provider = RemoteFontProvider::Fontshare;
    let zipped = archive(&[("nested/regular.TTF", &bytes)]);
    let preview = importer
        .preview_archive_font(&item, &zipped, "400-normal")
        .unwrap();
    assert_eq!(preview, bytes);
    assert!(!vault.exists());
    assert!(!temp.path().join("library.sqlite3").exists());
}

#[tokio::test]
#[ignore = "downloads real Fontsource and Fontshare fonts into a temporary vault"]
async fn live_direct_and_zip_imports_index_real_provider_fonts() {
    let client = CloudFontsClient::new().expect("HTTPS client");
    let temp = tempdir().expect("temporary vault");
    let database = temp.path().join("library.sqlite3");
    let importer = CloudFontImporter::new(temp.path().join("vault"), &database);
    for (provider, family_id) in [
        (RemoteFontProvider::FontsourceExtra, "adwaita-mono"),
        (RemoteFontProvider::Fontshare, "kihim"),
    ] {
        let catalog = client.fetch_catalog(provider).await.expect("live catalog");
        let item = catalog
            .iter()
            .find(|item| item.id == family_id)
            .expect("live family");
        let result = importer
            .import_catalog_item(&client, item)
            .await
            .expect("live import");
        assert!(result.report.saved_files > 0, "{:?}", result.report);
        assert!(result.report.indexed_fonts > 0, "{:?}", result.report);
        assert!(result.report.failures.is_empty(), "{:?}", result.report);
        for id in &result.font_ids {
            let font = db::font_by_id(&database, id).expect("indexed live font");
            assert!(font.file_path.is_file());
            assert_eq!(font.source_family_id.as_deref(), Some(family_id));
            assert!(!font.is_active);
        }
        println!(
            "{provider:?}: {} saved, {} indexed",
            result.report.saved_files, result.report.indexed_fonts
        );
    }
}
