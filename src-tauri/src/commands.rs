use std::{path::PathBuf, sync::Arc};

use rust_font_manager::models::{FontCategory, FontQuery, LibraryTab};
use tauri::{ipc::Response, State};

use crate::backend::{
    AppBackend, AppSettingsSnapshot, GooglePreview, StartupMaintenanceReport, StartupSnapshot,
};
use rust_font_manager::{
    models::{FontPage, RemoteFontCatalogSnapshot, RemoteFontProvider, ScanReport},
    services::{cloud_import::CloudFontImportOutcome, google_library::CatalogSnapshot},
};

async fn blocking<T, F>(operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn initialize_app(state: State<'_, Arc<AppBackend>>) -> Result<StartupSnapshot, String> {
    let backend = Arc::clone(state.inner());
    blocking(move || backend.startup_snapshot()).await
}

#[tauri::command]
pub async fn run_startup_maintenance(
    state: State<'_, Arc<AppBackend>>,
) -> Result<StartupMaintenanceReport, String> {
    let backend = Arc::clone(state.inner());
    blocking(move || Ok(backend.run_startup_maintenance())).await
}

#[tauri::command]
pub async fn list_fonts(
    state: State<'_, Arc<AppBackend>>,
    tab: LibraryTab,
    search: String,
    category: FontCategory,
    collection_id: Option<i64>,
    page: u32,
    page_size: u32,
) -> Result<FontPage, String> {
    let backend = Arc::clone(state.inner());
    blocking(move || {
        backend.list_fonts(FontQuery {
            tab,
            search,
            category,
            collection_id,
            page,
            page_size,
        })
    })
    .await
}

#[tauri::command]
pub async fn list_collections(
    state: State<'_, Arc<AppBackend>>,
) -> Result<Vec<rust_font_manager::models::Collection>, String> {
    let backend = Arc::clone(state.inner());
    blocking(move || backend.list_collections()).await
}

#[tauri::command]
pub async fn create_collection(
    state: State<'_, Arc<AppBackend>>,
    name: String,
) -> Result<rust_font_manager::models::Collection, String> {
    let backend = Arc::clone(state.inner());
    blocking(move || backend.create_collection(&name)).await
}

#[tauri::command]
pub async fn add_font_to_collection(
    state: State<'_, Arc<AppBackend>>,
    collection_id: i64,
    font_id: String,
) -> Result<(), String> {
    let backend = Arc::clone(state.inner());
    blocking(move || backend.add_font_to_collection(collection_id, &font_id)).await
}

#[tauri::command]
pub async fn set_font_favorite(
    state: State<'_, Arc<AppBackend>>,
    font_id: String,
    favorite: bool,
) -> Result<(), String> {
    let backend = Arc::clone(state.inner());
    blocking(move || backend.set_font_favorite(&font_id, favorite)).await
}

#[tauri::command]
pub async fn set_font_active(
    state: State<'_, Arc<AppBackend>>,
    font_id: String,
    active: bool,
) -> Result<(), String> {
    let backend = Arc::clone(state.inner());
    blocking(move || backend.set_font_active(&font_id, active)).await
}

#[tauri::command]
pub async fn get_font_preview_bytes(
    state: State<'_, Arc<AppBackend>>,
    font_id: String,
) -> Result<Response, String> {
    let backend = Arc::clone(state.inner());
    let bytes = blocking(move || backend.font_preview_bytes(&font_id)).await?;
    Ok(Response::new(bytes))
}

#[tauri::command]
pub async fn get_font_snippet(
    state: State<'_, Arc<AppBackend>>,
    font_id: String,
    kind: String,
) -> Result<String, String> {
    let backend = Arc::clone(state.inner());
    blocking(move || backend.font_snippet(&font_id, &kind)).await
}

#[tauri::command]
pub async fn scan_directory(
    state: State<'_, Arc<AppBackend>>,
    path: String,
) -> Result<rust_font_manager::models::ScanReport, String> {
    let backend = Arc::clone(state.inner());
    blocking(move || backend.scan_directory(&PathBuf::from(path))).await
}

#[tauri::command]
pub async fn import_zip_font(
    state: State<'_, Arc<AppBackend>>,
    path: String,
) -> Result<rust_font_manager::models::ScanReport, String> {
    let backend = Arc::clone(state.inner());
    blocking(move || backend.import_zip_font(&PathBuf::from(path))).await
}

#[tauri::command]
pub async fn load_cached_google_catalog(
    state: State<'_, Arc<AppBackend>>,
) -> Result<Option<CatalogSnapshot>, String> {
    let backend = Arc::clone(state.inner());
    blocking(move || backend.load_cached_google_catalog()).await
}

#[tauri::command]
pub async fn refresh_google_catalog(
    state: State<'_, Arc<AppBackend>>,
) -> Result<CatalogSnapshot, String> {
    Arc::clone(state.inner()).refresh_google_catalog().await
}

#[tauri::command]
pub async fn preview_google_variant(
    state: State<'_, Arc<AppBackend>>,
    family_id: String,
    variant_id: String,
) -> Result<GooglePreview, String> {
    Arc::clone(state.inner())
        .preview_google_variant(&family_id, &variant_id)
        .await
}

#[tauri::command]
pub async fn install_google_variant(
    state: State<'_, Arc<AppBackend>>,
    family_id: String,
    variant_id: String,
) -> Result<rust_font_manager::models::FontItem, String> {
    Arc::clone(state.inner())
        .install_google_variant(&family_id, &variant_id)
        .await
}

#[tauri::command]
pub async fn download_google_family(
    state: State<'_, Arc<AppBackend>>,
    family_id: String,
) -> Result<rust_font_manager::services::google_library::FamilyDownloadReport, String> {
    Arc::clone(state.inner())
        .download_google_family(&family_id)
        .await
}

#[tauri::command]
pub async fn load_cached_remote_catalog(
    state: State<'_, Arc<AppBackend>>,
    provider: RemoteFontProvider,
) -> Result<Option<RemoteFontCatalogSnapshot>, String> {
    let backend = Arc::clone(state.inner());
    blocking(move || backend.load_cached_remote_catalog(provider)).await
}

#[tauri::command]
pub async fn refresh_remote_catalog(
    state: State<'_, Arc<AppBackend>>,
    provider: RemoteFontProvider,
) -> Result<RemoteFontCatalogSnapshot, String> {
    Arc::clone(state.inner())
        .refresh_remote_catalog(provider)
        .await
}

#[tauri::command]
pub async fn get_remote_preview_bytes(
    state: State<'_, Arc<AppBackend>>,
    provider: RemoteFontProvider,
    family_id: String,
    variant_id: String,
) -> Result<Response, String> {
    let bytes = Arc::clone(state.inner())
        .preview_remote_variant(provider, &family_id, &variant_id)
        .await?;
    Ok(Response::new(bytes))
}

#[tauri::command]
pub async fn download_remote_family(
    state: State<'_, Arc<AppBackend>>,
    provider: RemoteFontProvider,
    family_id: String,
    variant_id: Option<String>,
    activate: bool,
) -> Result<CloudFontImportOutcome, String> {
    Arc::clone(state.inner())
        .download_remote_family(provider, &family_id, variant_id.as_deref(), activate)
        .await
}

#[tauri::command]
pub async fn get_app_settings(
    state: State<'_, Arc<AppBackend>>,
) -> Result<AppSettingsSnapshot, String> {
    let backend = Arc::clone(state.inner());
    blocking(move || backend.settings_snapshot()).await
}

#[tauri::command]
pub async fn add_scan_root(
    state: State<'_, Arc<AppBackend>>,
    path: String,
) -> Result<ScanReport, String> {
    let backend = Arc::clone(state.inner());
    blocking(move || backend.add_scan_root(&PathBuf::from(path))).await
}

#[tauri::command]
pub async fn scan_scan_root(
    state: State<'_, Arc<AppBackend>>,
    path: String,
) -> Result<ScanReport, String> {
    let backend = Arc::clone(state.inner());
    blocking(move || backend.scan_scan_root(&PathBuf::from(path))).await
}

#[tauri::command]
pub async fn set_scan_root_enabled(
    state: State<'_, Arc<AppBackend>>,
    path: String,
    enabled: bool,
) -> Result<(), String> {
    let backend = Arc::clone(state.inner());
    blocking(move || backend.set_scan_root_enabled(&PathBuf::from(path), enabled)).await
}

#[tauri::command]
pub async fn remove_scan_root(
    state: State<'_, Arc<AppBackend>>,
    path: String,
) -> Result<(), String> {
    let backend = Arc::clone(state.inner());
    blocking(move || backend.remove_scan_root(&PathBuf::from(path))).await
}
