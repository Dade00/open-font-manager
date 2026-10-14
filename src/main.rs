use rust_font_manager::app::FontApp;

fn main() -> iced::Result {
    iced::application("RustFontManager", FontApp::update, FontApp::view)
        .theme(FontApp::theme)
        .window(iced::window::Settings {
            size: iced::Size::new(1280.0, 800.0),
            min_size: Some(iced::Size::new(1000.0, 680.0)),
            position: iced::window::Position::Centered,
            ..Default::default()
        })
        .run_with(FontApp::new)
}
