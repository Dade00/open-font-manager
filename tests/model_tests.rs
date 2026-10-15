use rust_font_manager::models::{
    FontCategory, FontDownloadStrategy, FontSource, RemoteFontCatalogItem, RemoteFontFailure,
    RemoteFontInstallReport, RemoteFontProvider, RemoteFontVariant,
};

#[test]
fn persisted_source_and_category_round_trip_as_stable_strings() {
    let source = serde_json::to_string(&FontSource::Google).expect("serialize source");
    let category = serde_json::to_string(&FontCategory::SansSerif).expect("serialize category");

    assert_eq!(source, "\"google\"");
    assert_eq!(category, "\"sans-serif\"");
    assert_eq!(
        serde_json::from_str::<FontSource>(&source).unwrap(),
        FontSource::Google
    );
    assert_eq!(
        serde_json::from_str::<FontCategory>(&category).unwrap(),
        FontCategory::SansSerif
    );
}

#[test]
fn all_cloud_provider_sources_round_trip_as_stable_strings() {
    let cases = [
        (FontSource::Fontsource, "\"fontsource\""),
        (FontSource::Fontshare, "\"fontshare\""),
        (FontSource::NerdFonts, "\"nerd-fonts\""),
    ];

    for (source, expected) in cases {
        let encoded = serde_json::to_string(&source).expect("serialize source");
        assert_eq!(encoded, expected);
        assert_eq!(
            serde_json::from_str::<FontSource>(expected).unwrap(),
            source
        );
    }
}

#[test]
fn remote_provider_contract_uses_stable_ids_and_camel_case_fields() {
    let providers = [
        (RemoteFontProvider::Google, "google"),
        (RemoteFontProvider::FontsourceExtra, "fontsource-extra"),
        (RemoteFontProvider::Fontshare, "fontshare"),
        (RemoteFontProvider::NerdFonts, "nerd-fonts"),
    ];
    for (provider, expected) in providers {
        let encoded = serde_json::to_string(&provider).expect("serialize provider");
        assert_eq!(encoded, format!("\"{expected}\""));
        assert_eq!(
            serde_json::from_str::<RemoteFontProvider>(&encoded).unwrap(),
            provider
        );
    }

    let item = RemoteFontCatalogItem {
        id: "inter".into(),
        provider: RemoteFontProvider::FontsourceExtra,
        family: "Inter".into(),
        category: FontCategory::SansSerif,
        variants: vec![RemoteFontVariant {
            id: "400-normal".into(),
            label: "Regular".into(),
            weight: 400,
            is_italic: false,
        }],
        preview_variant_id: Some("400-normal".into()),
        download_strategy: FontDownloadStrategy::DirectFiles,
        download_url: Some("https://api.fontsource.org/v1/fonts/inter".into()),
    };
    let value = serde_json::to_value(item).expect("serialize catalog item");
    assert_eq!(value["provider"], "fontsource-extra");
    assert_eq!(value["previewVariantId"], "400-normal");
    assert_eq!(value["downloadStrategy"], "direct-files");
    assert_eq!(
        value["downloadUrl"],
        "https://api.fontsource.org/v1/fonts/inter"
    );
    assert_eq!(value["variants"][0]["isItalic"], false);
    assert_eq!(
        serde_json::to_string(&FontDownloadStrategy::ZipArchive).unwrap(),
        "\"zip-archive\""
    );

    let report = RemoteFontInstallReport {
        provider: RemoteFontProvider::Fontshare,
        family_id: "gambetta".into(),
        family: "Gambetta".into(),
        saved_files: 3,
        indexed_fonts: 3,
        activated_fonts: 3,
        failures: vec![RemoteFontFailure {
            variant_id: "700-italic".into(),
            stage: "activation".into(),
            reason: "not available".into(),
        }],
    };
    let report = serde_json::to_value(report).expect("serialize install report");
    assert_eq!(report["savedFiles"], 3);
    assert_eq!(report["indexedFonts"], 3);
    assert_eq!(report["failures"][0]["variantId"], "700-italic");
}
