use std::path::PathBuf;

use iced::font::{Family, Style, Weight};
use rust_font_manager::{
    font_names::{font_for_item, intern_font_family},
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
        is_favorite: false,
        file_size: 100,
        file_modified_unix_ms: 0,
        missing_on_disk: false,
    }
}

#[test]
fn family_interning_reuses_static_names_and_keeps_distinct_names_separate() {
    let first = intern_font_family("Font Intern Test");
    let repeated = intern_font_family("Font Intern Test");
    let other = intern_font_family("Font Intern Test Other");
    assert!(std::ptr::eq(first, repeated));
    assert_eq!(first, "Font Intern Test");
    assert_ne!(first, other);
}

#[test]
fn iced_font_uses_the_item_family_weight_and_italic_style() {
    let font = font_for_item(&item(FontSource::Local, "Example Family"));
    assert_eq!(font.family, Family::Name("Example Family"));
    assert_eq!(font.weight, Weight::Bold);
    assert_eq!(font.style, Style::Italic);
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
