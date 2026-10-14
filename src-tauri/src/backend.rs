use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rust_font_manager::{
    db,
    errors::AppError,
    google_fonts::GoogleFontsClient,
    models::{Collection, FontCategory, FontItem, FontPage, FontQuery, LibraryTab, ScanReport},
    paths::AppPaths,
    services::{
        google_library::{CatalogSnapshot, FamilyDownloadReport, GoogleLibraryService},
        local_library::LocalLibraryService,
    },
    snippets,
};
use serde::Serialize;

#[derive(Debug)]
pub struct AppBackend {
    paths: AppPaths,
    local: LocalLibraryService,
    google: Option<GoogleLibraryService>,
    google_error: Option<String>,
    maintenance_started: AtomicBool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupSnapshot {
    pub fonts: FontPage,
    pub collections: Vec<Collection>,
    pub active_count: usize,
    pub google_catalog: Option<CatalogSnapshot>,
    pub google_error: Option<String>,
    pub data_directory: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupMaintenanceReport {
    pub restored: usize,
    pub restore_failures: Vec<String>,
    pub scan_reports: Vec<ScanReport>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePreview {
    pub font_data_base64: String,
    pub family: String,
    pub subfamily: String,
    pub weight: u16,
    pub is_italic: bool,
}

impl AppBackend {
    pub fn new(paths: AppPaths) -> Result<Self, String> {
        db::migrate(&paths.database).map_err(user_message)?;
        let (google, google_error) = match GoogleFontsClient::new() {
            Ok(client) => (Some(GoogleLibraryService::new(client, paths.clone())), None),
            Err(error) => (None, Some(error.user_message())),
        };
        Ok(Self {
            local: LocalLibraryService::new(paths.database.clone()),
            paths,
            google,
            google_error,
            maintenance_started: AtomicBool::new(false),
        })
    }

    pub fn from_roots(data_dir: &Path, cache_dir: &Path) -> Result<Self, String> {
        let paths = AppPaths::from_roots(data_dir, cache_dir).map_err(user_message)?;
        Self::new(paths)
    }

    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    pub fn startup_snapshot(&self) -> Result<StartupSnapshot, String> {
        let fonts = self.list_fonts(FontQuery {
            tab: LibraryTab::Local,
            search: String::new(),
            category: FontCategory::All,
            collection_id: None,
            page: 0,
            page_size: 50,
        })?;
        let collections = self.list_collections()?;
        let active_count = db::count_active_fonts(&self.paths.database).map_err(user_message)?;
        let mut google_error = self.google_error.clone();
        let google_catalog = match &self.google {
            Some(service) => match service.load_cached_catalog() {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    google_error = Some(error.user_message());
                    None
                }
            },
            None => None,
        };
        Ok(StartupSnapshot {
            fonts,
            collections,
            active_count,
            google_catalog,
            google_error,
            data_directory: self.paths.data_dir.clone(),
        })
    }

    pub fn run_startup_maintenance(&self) -> StartupMaintenanceReport {
        if self.maintenance_started.swap(true, Ordering::AcqRel) {
            return StartupMaintenanceReport::default();
        }

        let mut report = StartupMaintenanceReport::default();
        match self.local.restore_active_fonts() {
            Ok(restored) => {
                report.restored = restored.restored;
                report.restore_failures = restored.failed;
            }
            Err(error) => report.errors.push(error.user_message()),
        }
        match self.local.scan_installed_fonts() {
            Ok(scans) => report.scan_reports = scans,
            Err(error) => report.errors.push(error.user_message()),
        }
        report
    }

    pub fn list_fonts(&self, query: FontQuery) -> Result<FontPage, String> {
        db::list_fonts(&self.paths.database, query).map_err(user_message)
    }

    pub fn list_collections(&self) -> Result<Vec<Collection>, String> {
        db::list_collections(&self.paths.database).map_err(user_message)
    }

    pub fn font_preview_bytes(&self, font_id: &str) -> Result<Vec<u8>, String> {
        let item = db::font_by_id(&self.paths.database, font_id).map_err(user_message)?;
        if item.missing_on_disk || !item.file_path.is_file() {
            return Err("Il file del font non è più disponibile.".into());
        }
        fs::read(&item.file_path).map_err(|error| {
            AppError::Storage {
                path: item.file_path,
                reason: error.to_string(),
            }
            .user_message()
        })
    }

    pub fn font_snippet(&self, font_id: &str, kind: &str) -> Result<String, String> {
        let item = db::font_by_id(&self.paths.database, font_id).map_err(user_message)?;
        match kind {
            "css" => Ok(snippets::css_snippet(&item)),
            "nextjs" => snippets::next_font_snippet(&item).map_err(user_message),
            _ => Err(format!("Tipo di snippet non supportato: {kind}")),
        }
    }

    pub fn set_font_favorite(&self, font_id: &str, favorite: bool) -> Result<(), String> {
        db::set_font_favorite(&self.paths.database, font_id, favorite).map_err(user_message)
    }

    pub fn set_font_active(&self, font_id: &str, active: bool) -> Result<(), String> {
        if active {
            let item = db::font_by_id(&self.paths.database, font_id).map_err(user_message)?;
            if item.missing_on_disk || !item.file_path.is_file() {
                return Err("Il file del font non è più disponibile.".into());
            }
            self.local
                .activate(&item.id, &item.file_path)
                .map(|_| ())
                .map_err(user_message)
        } else {
            self.local.deactivate(font_id).map_err(user_message)
        }
    }

    pub fn create_collection(&self, name: &str) -> Result<Collection, String> {
        db::create_collection(&self.paths.database, name).map_err(user_message)
    }

    pub fn add_font_to_collection(&self, collection_id: i64, font_id: &str) -> Result<(), String> {
        db::add_font_to_collection(&self.paths.database, collection_id, font_id)
            .map_err(user_message)
    }

    pub fn scan_directory(&self, root: &Path) -> Result<ScanReport, String> {
        self.local.scan_directory(root).map_err(user_message)
    }

    pub fn load_cached_google_catalog(&self) -> Result<Option<CatalogSnapshot>, String> {
        self.google
            .as_ref()
            .ok_or_else(|| {
                self.google_error
                    .clone()
                    .unwrap_or_else(|| "Google Fonts non disponibile.".into())
            })?
            .load_cached_catalog()
            .map_err(user_message)
    }

    pub async fn refresh_google_catalog(&self) -> Result<CatalogSnapshot, String> {
        self.google
            .as_ref()
            .ok_or_else(|| {
                self.google_error
                    .clone()
                    .unwrap_or_else(|| "Google Fonts non disponibile.".into())
            })?
            .refresh_catalog()
            .await
            .map_err(user_message)
    }

    pub async fn preview_google_variant(
        &self,
        family_id: &str,
        variant_id: &str,
    ) -> Result<GooglePreview, String> {
        let (bytes, metadata) = self
            .google
            .as_ref()
            .ok_or_else(|| {
                self.google_error
                    .clone()
                    .unwrap_or_else(|| "Google Fonts non disponibile.".into())
            })?
            .preview_variant(family_id, Some(variant_id))
            .await
            .map_err(user_message)?;
        Ok(GooglePreview {
            font_data_base64: BASE64.encode(bytes),
            family: metadata.family,
            subfamily: metadata.subfamily,
            weight: metadata.weight,
            is_italic: metadata.is_italic,
        })
    }

    pub async fn install_google_variant(
        &self,
        family_id: &str,
        variant_id: &str,
    ) -> Result<FontItem, String> {
        self.google
            .as_ref()
            .ok_or_else(|| {
                self.google_error
                    .clone()
                    .unwrap_or_else(|| "Google Fonts non disponibile.".into())
            })?
            .install_variant(family_id, variant_id)
            .await
            .map_err(user_message)
    }

    pub async fn download_google_family(
        &self,
        family_id: &str,
    ) -> Result<FamilyDownloadReport, String> {
        self.google
            .as_ref()
            .ok_or_else(|| {
                self.google_error
                    .clone()
                    .unwrap_or_else(|| "Google Fonts non disponibile.".into())
            })?
            .download_family(family_id)
            .await
            .map_err(user_message)
    }
}

fn user_message(error: AppError) -> String {
    error.user_message()
}
