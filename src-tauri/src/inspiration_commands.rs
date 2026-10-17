use crate::backend::AppBackend;
use rust_font_manager::inspiration::{self, InspirationAsset, InspirationDocument};
use std::{path::PathBuf, process::Command, sync::Arc};
use tauri::State;

#[tauri::command]
pub fn load_inspiration(
    backend: State<'_, Arc<AppBackend>>,
) -> Result<InspirationDocument, String> {
    let paths = backend.paths().clone();
    inspiration::load(&paths.database)
}
#[tauri::command]
pub async fn save_inspiration(
    backend: State<'_, Arc<AppBackend>>,
    document: InspirationDocument,
    expected_revision: u64,
) -> Result<InspirationDocument, String> {
    let paths = backend.paths().clone();
    tauri::async_runtime::spawn_blocking(move || {
        inspiration::save(
            &paths.database,
            &paths.data_dir,
            document,
            expected_revision,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn import_inspiration_image(
    backend: State<'_, Arc<AppBackend>>,
    path: PathBuf,
) -> Result<InspirationAsset, String> {
    let paths = backend.paths().clone();
    tauri::async_runtime::spawn_blocking(move || {
        inspiration::import_image(&paths.database, &paths.data_dir, &path)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn get_inspiration_asset(
    backend: State<'_, Arc<AppBackend>>,
    asset_id: String,
) -> Result<Vec<u8>, String> {
    let data_dir = backend.paths().data_dir.clone();
    tauri::async_runtime::spawn_blocking(move || inspiration::get_asset(&data_dir, &asset_id))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn open_inspiration_link(url: String) -> Result<(), String> {
    inspiration::validate_link(&url)?;
    #[cfg(target_os = "windows")]
    {
        Command::new("rundll32.exe")
            .arg("url.dll,FileProtocolHandler")
            .arg(&url)
            .spawn()
            .map(|_| ())
            .map_err(|_| "Impossibile aprire il collegamento nel browser.".to_owned())
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&url)
            .spawn()
            .map(|_| ())
            .map_err(|_| "Impossibile aprire il collegamento nel browser.".to_owned())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map(|_| ())
            .map_err(|_| "Impossibile aprire il collegamento nel browser.".to_owned())
    }
}
