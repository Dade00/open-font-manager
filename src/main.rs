use iced::widget::{column, text};
use iced::{Element, Task};

#[derive(Debug, Default)]
struct StartupState;

#[derive(Debug, Clone)]
enum StartupMessage {}

fn update(_state: &mut StartupState, message: StartupMessage) -> Task<StartupMessage> {
    match message {}
}

fn view(_state: &StartupState) -> Element<'_, StartupMessage> {
    column![text("RustFontManager")].into()
}

fn main() -> iced::Result {
    iced::application("RustFontManager", update, view).run()
}
