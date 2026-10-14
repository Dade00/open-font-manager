use iced::{theme::Palette, Color, Theme};

pub fn rust_font_manager() -> Theme {
    Theme::custom(
        "RustFontManager".to_owned(),
        Palette {
            background: Color::from_rgb8(0x0F, 0x17, 0x2A),
            text: Color::from_rgb8(0xF8, 0xFA, 0xFC),
            primary: Color::from_rgb8(0x38, 0xBD, 0xF8),
            success: Color::from_rgb8(0x22, 0xC5, 0x5E),
            danger: Color::from_rgb8(0xF8, 0x71, 0x71),
        },
    )
}
