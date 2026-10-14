use std::path::PathBuf;

use rust_font_manager::{
    models::{FontCategory, FontItem, FontSource},
    snippets::{css_snippet, next_font_snippet},
};

fn item(source: FontSource, family: &str) -> FontItem {
    FontItem {
        id: "id".into(),
        family: family.into(),
        subfamily: "Bold Italic".into(),
        full_name: format!("{family} Bold Italic"),
        weight: 700,
        is_italic: true,
        file_path: PathBuf::from("C:/fonts/test-font.ttf"),
        source,
        source_family_id: (source == FontSource::Google).then(|| "open-sans".into()),
        source_variant_id: None,
        category: FontCategory::Unknown,
        is_active: false,
        is_installed: false,
        is_favorite: false,
        file_size: 100,
        file_modified_unix_ms: 0,
        missing_on_disk: false,
    }
}

#[test]
fn css_snippet_escapes_quotes_backslashes_and_control_characters() {
    let snippet = css_snippet(&item(FontSource::Local, "Font\"\\\nName"));
    assert!(snippet.contains("font-family: \"Font\\\"\\\\\\a Name\", sans-serif;"));
    assert!(snippet.contains("font-weight: 700;"));
    assert!(snippet.contains("font-style: italic;"));
}

#[test]
fn google_next_snippet_uses_valid_google_export_and_variant_style() {
    let snippet =
        next_font_snippet(&item(FontSource::Google, "Open Sans")).expect("Google snippet");
    assert!(snippet.contains("import { Open_Sans } from \"next/font/google\";"));
    assert!(snippet.contains("const openSans = Open_Sans({"));
    assert!(snippet.contains("weight: [\"700\"]"));
    assert!(snippet.contains("style: [\"italic\"]"));
}

#[test]
fn local_next_snippet_uses_a_relative_path_and_copy_notice() {
    let snippet =
        next_font_snippet(&item(FontSource::Local, "Example Family")).expect("local snippet");
    assert!(snippet.contains("next/font/local"));
    assert!(snippet.contains("src: \"./test-font.ttf\""));
    assert!(snippet.contains("display: \"swap\""));
}
