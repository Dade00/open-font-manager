mod backend;
mod commands;

use std::sync::Arc;
use tauri::Manager;

pub use backend::AppBackend;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let paths = rust_font_manager::paths::AppPaths::discover()?;
            app.manage(Arc::new(
                AppBackend::new(paths).map_err(std::io::Error::other)?,
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::initialize_app,
            commands::run_startup_maintenance,
            commands::list_fonts,
            commands::list_collections,
            commands::create_collection,
            commands::add_font_to_collection,
            commands::set_font_favorite,
            commands::set_font_active,
            commands::get_font_preview_bytes,
            commands::get_font_snippet,
            commands::scan_directory,
            commands::load_cached_google_catalog,
            commands::refresh_google_catalog,
            commands::preview_google_variant,
            commands::install_google_variant,
            commands::download_google_family
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Open Font Manager");
}
