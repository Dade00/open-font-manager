use std::sync::{Mutex, OnceLock};

use iced::font::{Font, Style, Weight};

use crate::models::FontItem;

static FONT_NAMES: OnceLock<Mutex<std::collections::HashSet<&'static str>>> = OnceLock::new();

pub fn intern_font_family(name: &str) -> &'static str {
    let names = FONT_NAMES.get_or_init(|| Mutex::new(std::collections::HashSet::new()));
    let mut names = names
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(existing) = names.get(name) {
        return existing;
    }
    let interned: &'static str = Box::leak(name.to_owned().into_boxed_str());
    names.insert(interned);
    interned
}

pub fn font_for_item(item: &FontItem) -> Font {
    Font {
        family: iced::font::Family::Name(intern_font_family(&item.family)),
        weight: iced_weight(item.weight),
        style: if item.is_italic {
            Style::Italic
        } else {
            Style::Normal
        },
        ..Font::DEFAULT
    }
}

fn iced_weight(weight: u16) -> Weight {
    match weight {
        1..=149 => Weight::Thin,
        150..=249 => Weight::ExtraLight,
        250..=349 => Weight::Light,
        350..=449 => Weight::Normal,
        450..=549 => Weight::Medium,
        550..=649 => Weight::Semibold,
        650..=749 => Weight::Bold,
        750..=849 => Weight::ExtraBold,
        _ => Weight::Black,
    }
}
