use rust_font_manager::models::{FontCategory, FontSource};

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
