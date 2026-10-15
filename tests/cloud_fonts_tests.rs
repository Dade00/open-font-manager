use rust_font_manager::{
    cloud_fonts::{
        map_google_catalog, parse_fontshare_catalog, parse_fontsource_catalog,
        parse_fontsource_details, parse_nerd_fonts_release, validate_download_url,
        CloudCatalogCache, CloudFontsClient,
    },
    models::{
        FontCategory, FontDownloadStrategy, GoogleFontCatalogItem, RemoteFontCatalogSnapshot,
        RemoteFontProvider,
    },
};

#[test]
fn fontsource_catalog_keeps_only_extra_fonts_and_selects_default_variant() {
    let json = br#"[
        {"id":"adwaita-mono","family":"Adwaita Mono","weights":[400,700],"styles":["normal","italic"],"defSubset":"latin","category":"monospace","type":"other"},
        {"id":"roboto","family":"Roboto","weights":[400],"styles":["normal"],"defSubset":"latin","category":"sans-serif","type":"google"}
    ]"#;

    let items = parse_fontsource_catalog(json).expect("parse Fontsource catalog");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].provider, RemoteFontProvider::FontsourceExtra);
    assert_eq!(items[0].id, "adwaita-mono");
    assert_eq!(items[0].category, FontCategory::Monospace);
    assert_eq!(items[0].preview_variant_id.as_deref(), Some("400-normal"));
    assert_eq!(
        items[0].download_strategy,
        FontDownloadStrategy::DirectFiles
    );
    assert!(items[0].download_url.is_none());
    assert_eq!(items[0].variants.len(), 4);
}

#[test]
fn fontsource_catalog_accepts_empty_lists_and_rejects_missing_identity() {
    assert!(parse_fontsource_catalog(b"[]").unwrap().is_empty());
    assert!(parse_fontsource_catalog(br#"[{"family":"No ID","type":"other"}]"#).is_err());
}

#[test]
fn fontsource_details_use_default_subset_and_only_ttf_urls() {
    let json = br#"{
        "id":"adwaita-mono",
        "defSubset":"latin",
        "variants":{
            "400":{"normal":{"latin":{"url":{"ttf":"https://cdn.jsdelivr.net/fontsource/fonts/adwaita-mono@latest/latin-400-normal.ttf","woff2":"https://cdn.jsdelivr.net/fontsource/fonts/adwaita-mono@latest/latin-400-normal.woff2"}}}},
            "700":{"italic":{"cyrillic":{"url":{"ttf":"https://cdn.jsdelivr.net/fontsource/fonts/adwaita-mono@latest/cyrillic-700-italic.ttf"}}}}
        }
    }"#;

    let files = parse_fontsource_details(json, "adwaita-mono").expect("parse detail");

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].variant_id, "400-normal");
    assert_eq!(
        files[0].url,
        "https://cdn.jsdelivr.net/fontsource/fonts/adwaita-mono@latest/latin-400-normal.ttf"
    );
}

#[test]
fn fontshare_accepts_fonts_root_and_prefers_regular_nonitalic_style() {
    let json = br#"{"count":1,"fonts":[{"slug":"gambetta","name":"Gambetta","styles":[
        {"default":false,"id":"italic","file":"//cdn.fontshare.com/italic","weight":{"number":400,"label":"Regular"},"is_italic":true},
        {"default":true,"id":"regular","file":"//cdn.fontshare.com/regular","weight":{"number":400,"label":"Regular"},"is_italic":false}
    ]}]}"#;

    let items = parse_fontshare_catalog(json).expect("parse Fontshare catalog");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "gambetta");
    assert_eq!(items[0].family, "Gambetta");
    assert_eq!(items[0].provider, RemoteFontProvider::Fontshare);
    assert_eq!(items[0].download_strategy, FontDownloadStrategy::ZipArchive);
    assert_eq!(items[0].preview_variant_id.as_deref(), Some("regular"));
    assert_eq!(
        items[0].download_url.as_deref(),
        Some("https://api.fontshare.com/v2/fonts/download/gambetta")
    );
}

#[test]
fn fontshare_response_root_and_style_fallback_are_supported() {
    let json = br#"{"response":[{"slug":"sample","name":"Sample","styles":[
        {"id":"medium","file":"//cdn.fontshare.com/medium","weight":{"weight":500,"label":"Medium"},"is_italic":false},
        {"id":"light-italic","file":"//cdn.fontshare.com/light-italic","weight":{"weight":300,"label":"Light"},"is_italic":true}
    ]}]}"#;

    let items = parse_fontshare_catalog(json).expect("parse legacy Fontshare root");

    assert_eq!(items[0].preview_variant_id.as_deref(), Some("medium"));
    assert_eq!(items[0].variants[0].weight, 500);
}

#[test]
fn fontshare_catalog_handles_empty_and_rejects_invalid_slugs() {
    assert!(parse_fontshare_catalog(br#"{"fonts":[],"count":0}"#)
        .unwrap()
        .is_empty());
    assert!(parse_fontshare_catalog(br#"{"fonts":[{"name":"Bad","styles":[]}] }"#).is_err());
}

#[test]
fn nerd_release_exposes_only_safe_zip_assets_and_keeps_large_asset_metadata() {
    let json = br#"{
        "tag_name":"v3.5.1",
        "assets":[
            {"name":"FiraCode.zip","size":314572800,"browser_download_url":"https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/FiraCode.zip"},
            {"name":"FontPatcher.zip","size":1000,"browser_download_url":"https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/FontPatcher.zip"},
            {"name":"FiraCode.tar.xz","size":1000,"browser_download_url":"https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/FiraCode.tar.xz"},
            {"name":"Unsafe.zip","size":1000,"browser_download_url":"http://evil.example/Unsafe.zip"}
        ]
    }"#;

    let items = parse_nerd_fonts_release(json).expect("parse Nerd Fonts release");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "FiraCode");
    assert_eq!(items[0].family, "FiraCode");
    assert_eq!(items[0].provider, RemoteFontProvider::NerdFonts);
    assert_eq!(items[0].download_strategy, FontDownloadStrategy::ZipArchive);
    assert_eq!(
        items[0].download_url.as_deref(),
        Some("https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/FiraCode.zip")
    );
}

#[test]
fn remote_urls_require_https_and_provider_allowlisted_hosts() {
    let valid = validate_download_url(
        RemoteFontProvider::FontsourceExtra,
        "https://cdn.jsdelivr.net/fontsource/fonts/example.ttf",
    )
    .expect("Fontsource CDN URL");
    assert_eq!(valid.scheme(), "https");
    assert!(validate_download_url(
        RemoteFontProvider::FontsourceExtra,
        "http://cdn.jsdelivr.net/fontsource/fonts/example.ttf"
    )
    .is_err());
    assert!(validate_download_url(
        RemoteFontProvider::FontsourceExtra,
        "https://cdn.jsdelivr.net.evil.example/font.ttf"
    )
    .is_err());
    assert!(validate_download_url(
        RemoteFontProvider::NerdFonts,
        "https://user@github.com/ryanoasis/nerd-fonts/a.zip"
    )
    .is_err());
    let scheme_relative = validate_download_url(
        RemoteFontProvider::Fontshare,
        "//api.fontshare.com/v2/fonts/download/gambetta",
    )
    .expect("normalize scheme-relative URL");
    assert_eq!(scheme_relative.scheme(), "https");
}

#[test]
fn google_catalog_maps_to_the_shared_remote_contract() {
    let google = GoogleFontCatalogItem {
        id: "open-sans".into(),
        family: "Open Sans".into(),
        subsets: vec!["latin".into()],
        category: "sans-serif".into(),
        default_variant: "regular".into(),
        default_subset: "latin".into(),
        variants: vec![
            "regular".into(),
            "700".into(),
            "italic".into(),
            "700italic".into(),
        ],
        popularity: 1,
    };

    let items = map_google_catalog(&[google]);

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].provider, RemoteFontProvider::Google);
    assert_eq!(items[0].preview_variant_id.as_deref(), Some("regular"));
    assert_eq!(items[0].variants[0].weight, 400);
    assert_eq!(items[0].variants[3].weight, 700);
    assert!(items[0].variants[3].is_italic);
}

#[test]
fn cloud_catalog_cache_persists_a_stale_snapshot_without_network_access() {
    let dir = tempfile::tempdir().expect("cache directory");
    let cache = CloudCatalogCache::new(dir.path());
    let snapshot = RemoteFontCatalogSnapshot {
        provider: RemoteFontProvider::Fontshare,
        items: vec![],
        fetched_at_unix_ms: 1,
        stale: false,
        last_error: None,
    };

    cache.store(&snapshot).expect("store cache");
    let loaded = cache
        .load(RemoteFontProvider::Fontshare)
        .expect("load cache")
        .expect("cached catalog");

    assert!(loaded.stale);
    assert_eq!(loaded.fetched_at_unix_ms, 1);
    assert!(loaded.items.is_empty());
}

#[tokio::test]
#[ignore = "requires public Fontsource, Fontshare, and GitHub APIs"]
async fn live_provider_catalogs_parse_against_public_apis() {
    let client = CloudFontsClient::new().expect("construct client");
    for provider in [
        RemoteFontProvider::FontsourceExtra,
        RemoteFontProvider::Fontshare,
        RemoteFontProvider::NerdFonts,
    ] {
        let items = client
            .fetch_catalog(provider)
            .await
            .expect("fetch provider catalog");
        assert!(!items.is_empty(), "{provider:?} returned an empty catalog");
    }
}
