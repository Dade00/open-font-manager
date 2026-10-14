use rust_font_manager::{
    google_fonts::GoogleFontsClient, models::FontSource, paths::AppPaths,
    services::google_library::GoogleLibraryService,
};
use tempfile::tempdir;

#[tokio::test]
#[ignore = "requires the public google-webfonts-helper service"]
async fn public_catalog_cache_preview_and_install_flow_work() {
    let root = tempdir().expect("temporary app root");
    let paths = AppPaths::from_roots(&root.path().join("data"), &root.path().join("cache"))
        .expect("app paths");
    let service = GoogleLibraryService::new(
        GoogleFontsClient::new().expect("HTTP client"),
        paths.clone(),
    );

    let catalog = service.refresh_catalog().await.expect("live catalog");
    let roboto = catalog
        .items
        .iter()
        .find(|font| font.id == "roboto")
        .expect("Roboto in catalog");
    assert_eq!(roboto.family, "Roboto");
    assert!(paths.catalog_cache.is_file());
    assert!(paths.catalog_meta.is_file());

    let cached = service
        .load_cached_catalog()
        .expect("read cache")
        .expect("catalog cache");
    assert!(!cached.stale);
    assert!(cached.items.iter().any(|font| font.id == "roboto"));

    let (preview_bytes, preview_metadata) = service
        .preview_variant("roboto", Some("regular"))
        .await
        .expect("Roboto preview download");
    assert!(!preview_bytes.is_empty());
    assert_eq!(preview_metadata.family, "Roboto");
    let item = service
        .install_variant("roboto", "regular")
        .await
        .expect("install Roboto locally");
    assert_eq!(item.source, FontSource::Google);
    assert_eq!(item.family, "Roboto");
    assert!(!item.is_active);
    assert!(item.file_path.is_file());
}
