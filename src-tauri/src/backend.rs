use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rust_font_manager::cloud_fonts::{map_google_catalog, CloudFontsService};
use rust_font_manager::{
    db,
    errors::AppError,
    font_scanner::parse_font_bytes,
    google_fonts::GoogleFontsClient,
    installed_fonts::{installed_font_roots, is_installed_font_path},
    models::{
        Collection, FontCategory, FontItem, FontPage, FontQuery, LibraryTab,
        RemoteFontCatalogSnapshot, RemoteFontFailure, RemoteFontInstallReport, RemoteFontProvider,
        ScanReport, ScanRoot,
    },
    paths::AppPaths,
    services::{
        cloud_import::{CloudFontImportOutcome, CloudFontImporter},
        font_install::{FontInstaller, FontRegistration, WindowsFontRegistration},
        google_library::{CatalogSnapshot, FamilyDownloadReport, GoogleLibraryService},
        local_library::LocalLibraryService,
    },
    snippets,
};
use serde::Serialize;

pub struct AppBackend {
    paths: AppPaths,
    local: LocalLibraryService,
    font_installer: FontInstaller<Arc<dyn FontRegistration>>,
    google: Option<GoogleLibraryService>,
    google_error: Option<String>,
    cloud: Option<CloudFontsService>,
    cloud_error: Option<String>,
    cloud_importer: CloudFontImporter,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRootSettings {
    pub root: ScanRoot,
    pub is_accessible: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteCatalogStatus {
    pub provider: RemoteFontProvider,
    pub available: bool,
    pub item_count: usize,
    pub fetched_at_unix_ms: Option<i64>,
    pub stale: Option<bool>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsSnapshot {
    pub data_directory: PathBuf,
    pub cache_directory: PathBuf,
    pub font_vault_directory: PathBuf,
    pub windows_fonts_directory: PathBuf,
    pub scan_roots: Vec<ScanRootSettings>,
    pub cloud_catalogs: Vec<RemoteCatalogStatus>,
}

impl AppBackend {
    pub fn new(paths: AppPaths) -> Result<Self, String> {
        let install_dir = user_font_directory(&paths.data_dir);
        Self::with_font_registration(paths, install_dir, Arc::new(WindowsFontRegistration))
    }

    fn with_font_registration(
        paths: AppPaths,
        install_dir: PathBuf,
        registration: Arc<dyn FontRegistration>,
    ) -> Result<Self, String> {
        db::migrate(&paths.database).map_err(user_message)?;
        let (google, google_error) = match GoogleFontsClient::new() {
            Ok(client) => (Some(GoogleLibraryService::new(client, paths.clone())), None),
            Err(error) => (None, Some(error.user_message())),
        };
        let (cloud, cloud_error) =
            match CloudFontsService::new(paths.cache_dir.join("cloud-catalogs")) {
                Ok(service) => (Some(service), None),
                Err(error) => (None, Some(error.user_message())),
            };
        let cloud_importer =
            CloudFontImporter::new(paths.data_dir.join("vault"), paths.database.clone());
        Ok(Self {
            local: LocalLibraryService::new(paths.database.clone()),
            font_installer: FontInstaller::new(install_dir, registration),
            paths,
            google,
            google_error,
            cloud,
            cloud_error,
            cloud_importer,
            maintenance_started: AtomicBool::new(false),
        })
    }

    pub fn from_roots(data_dir: &Path, cache_dir: &Path) -> Result<Self, String> {
        let paths = AppPaths::from_roots(data_dir, cache_dir).map_err(user_message)?;
        Self::new(paths)
    }

    pub fn from_roots_with_registration(
        data_dir: &Path,
        cache_dir: &Path,
        install_dir: &Path,
        registration: Arc<dyn FontRegistration>,
    ) -> Result<Self, String> {
        let paths = AppPaths::from_roots(data_dir, cache_dir).map_err(user_message)?;
        Self::with_font_registration(paths, install_dir.to_path_buf(), registration)
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
        let active_count = self
            .list_fonts(FontQuery {
                tab: LibraryTab::Active,
                search: String::new(),
                category: FontCategory::All,
                collection_id: None,
                page: 0,
                page_size: 1,
            })?
            .total;
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
        match db::list_active_fonts(&self.paths.database) {
            Ok(fonts) => {
                for font in fonts {
                    if font.missing_on_disk || !font.file_path.is_file() {
                        let _ = db::set_font_active(&self.paths.database, &font.id, false);
                        report.restore_failures.push(font.family);
                        continue;
                    }
                    match self
                        .font_installer
                        .install(&font.id, &font.full_name, &font.file_path)
                    {
                        Ok(_) => report.restored += 1,
                        Err(error) => {
                            let _ = db::set_font_active(&self.paths.database, &font.id, false);
                            report.errors.push(user_message(error));
                        }
                    }
                }
            }
            Err(error) => report.errors.push(user_message(error)),
        }
        match self.local.scan_installed_fonts() {
            Ok(scans) => report.scan_reports = scans,
            Err(error) => report.errors.push(error.user_message()),
        }
        report
    }

    pub fn list_fonts(&self, query: FontQuery) -> Result<FontPage, String> {
        if !(1..=100).contains(&query.page_size) {
            return Err("Font page size must be between 1 and 100.".into());
        }
        let roots = installed_font_roots();
        if query.tab == LibraryTab::Active {
            return db::list_installed_fonts(&self.paths.database, query, &roots)
                .map_err(user_message);
        }

        let mut page = db::list_fonts(&self.paths.database, query).map_err(user_message)?;
        mark_installed(&mut page.items, &roots);
        Ok(page)
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
        let item = db::font_by_id(&self.paths.database, font_id).map_err(user_message)?;
        let installed_by_windows = is_installed_font_path(&item.file_path, &installed_font_roots());
        if installed_by_windows && !item.is_active {
            return Err("Questo font è già installato in Windows e non è gestito da OFM.".into());
        }
        if active && (item.missing_on_disk || !item.file_path.is_file()) {
            return Err("Il file del font non è più disponibile.".into());
        }
        if active {
            self.font_installer
                .install(&item.id, &item.full_name, &item.file_path)
                .map_err(user_message)?;
            if let Err(error) = db::set_font_active(&self.paths.database, &item.id, true) {
                let rollback =
                    self.font_installer
                        .uninstall(&item.id, &item.full_name, &item.file_path);
                return Err(match rollback {
                    Ok(()) => user_message(error),
                    Err(rollback_error) => format!(
                        "{} Rollback installazione non riuscito: {}",
                        user_message(error),
                        user_message(rollback_error)
                    ),
                });
            }
            Ok(())
        } else {
            self.font_installer
                .uninstall(&item.id, &item.full_name, &item.file_path)
                .map_err(user_message)?;
            if let Err(error) = db::set_font_active(&self.paths.database, &item.id, false) {
                let rollback =
                    self.font_installer
                        .install(&item.id, &item.full_name, &item.file_path);
                return Err(match rollback {
                    Ok(_) => user_message(error),
                    Err(rollback_error) => format!(
                        "{} Ripristino installazione non riuscito: {}",
                        user_message(error),
                        user_message(rollback_error)
                    ),
                });
            }
            Ok(())
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

    pub fn import_zip_font(&self, archive_path: &Path) -> Result<ScanReport, String> {
        let imports_root = self.paths.data_dir.join("imported-fonts");
        let extracted_root = rust_font_manager::services::zip_import::extract_font_archive(
            archive_path,
            &imports_root,
        )
        .map_err(user_message)?;
        self.local
            .scan_directory(&extracted_root)
            .map_err(user_message)
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

impl AppBackend {
    pub fn load_cached_remote_catalog(
        &self,
        provider: RemoteFontProvider,
    ) -> Result<Option<RemoteFontCatalogSnapshot>, String> {
        if provider == RemoteFontProvider::Google {
            let snapshot = self
                .google
                .as_ref()
                .ok_or_else(|| {
                    self.google_error
                        .clone()
                        .unwrap_or_else(|| "Google Fonts non disponibile.".into())
                })?
                .load_cached_catalog()
                .map_err(user_message)?;
            return Ok(snapshot.map(google_snapshot_to_remote));
        }
        self.cloud
            .as_ref()
            .ok_or_else(|| {
                self.cloud_error
                    .clone()
                    .unwrap_or_else(|| "Cataloghi cloud non disponibili.".into())
            })?
            .load_cached_catalog(provider)
            .map_err(user_message)
    }

    pub async fn refresh_remote_catalog(
        &self,
        provider: RemoteFontProvider,
    ) -> Result<RemoteFontCatalogSnapshot, String> {
        if provider == RemoteFontProvider::Google {
            let snapshot = self
                .google
                .as_ref()
                .ok_or_else(|| {
                    self.google_error
                        .clone()
                        .unwrap_or_else(|| "Google Fonts non disponibile.".into())
                })?
                .refresh_catalog()
                .await
                .map_err(user_message)?;
            return Ok(google_snapshot_to_remote(snapshot));
        }
        self.cloud
            .as_ref()
            .ok_or_else(|| {
                self.cloud_error
                    .clone()
                    .unwrap_or_else(|| "Cataloghi cloud non disponibili.".into())
            })?
            .refresh_catalog(provider)
            .await
            .map_err(user_message)
    }

    pub fn settings_snapshot(&self) -> Result<AppSettingsSnapshot, String> {
        let scan_roots = db::list_scan_roots(&self.paths.database)
            .map_err(user_message)?
            .into_iter()
            .map(|root| ScanRootSettings {
                is_accessible: fs::read_dir(&root.path).is_ok(),
                root,
            })
            .collect();
        let cloud_catalogs = [
            RemoteFontProvider::Google,
            RemoteFontProvider::FontsourceExtra,
            RemoteFontProvider::Fontshare,
            RemoteFontProvider::NerdFonts,
        ]
        .into_iter()
        .map(|provider| {
            let snapshot = self.load_cached_remote_catalog(provider);
            let (available, item_count, fetched_at_unix_ms, stale, last_error) = match snapshot {
                Ok(Some(snapshot)) => (
                    true,
                    snapshot.items.len(),
                    Some(snapshot.fetched_at_unix_ms),
                    Some(snapshot.stale),
                    snapshot.last_error,
                ),
                Ok(None) => (
                    self.provider_available(provider),
                    0,
                    None,
                    None,
                    self.provider_error(provider),
                ),
                Err(error) => (
                    self.provider_available(provider),
                    0,
                    None,
                    None,
                    Some(error),
                ),
            };
            RemoteCatalogStatus {
                provider,
                available,
                item_count,
                fetched_at_unix_ms,
                stale,
                last_error,
            }
        })
        .collect();
        Ok(AppSettingsSnapshot {
            data_directory: self.paths.data_dir.clone(),
            cache_directory: self.paths.cache_dir.clone(),
            font_vault_directory: self.paths.data_dir.join("vault"),
            windows_fonts_directory: user_font_directory(&self.paths.data_dir),
            scan_roots,
            cloud_catalogs,
        })
    }

    pub fn add_scan_root(&self, path: &Path) -> Result<ScanReport, String> {
        let canonical = self.local.register_scan_root(path).map_err(user_message)?;
        self.local.scan_directory(&canonical).map_err(user_message)
    }

    pub fn scan_scan_root(&self, path: &Path) -> Result<ScanReport, String> {
        let roots = db::list_scan_roots(&self.paths.database).map_err(user_message)?;
        let root = roots
            .into_iter()
            .find(|root| root.path == path)
            .ok_or_else(|| "La cartella non è configurata come radice di scansione.".to_owned())?;
        if !root.is_enabled {
            return Err("Riattiva la cartella prima di avviare la scansione.".into());
        }
        self.local.scan_directory(path).map_err(user_message)
    }

    pub fn set_scan_root_enabled(&self, path: &Path, enabled: bool) -> Result<(), String> {
        db::set_scan_root_enabled(&self.paths.database, path, enabled).map_err(user_message)
    }

    pub fn remove_scan_root(&self, path: &Path) -> Result<(), String> {
        db::remove_scan_root(&self.paths.database, path).map_err(user_message)
    }

    pub async fn preview_remote_variant(
        &self,
        provider: RemoteFontProvider,
        family_id: &str,
        variant_id: &str,
    ) -> Result<Vec<u8>, String> {
        let bytes = match provider {
            RemoteFontProvider::Google => {
                self.google
                    .as_ref()
                    .ok_or_else(|| {
                        self.google_error
                            .clone()
                            .unwrap_or_else(|| "Google Fonts non disponibile.".into())
                    })?
                    .preview_variant(family_id, Some(variant_id))
                    .await
                    .map_err(user_message)?
                    .0
            }
            RemoteFontProvider::FontsourceExtra => {
                let client = self
                    .cloud
                    .as_ref()
                    .ok_or_else(|| {
                        self.cloud_error
                            .clone()
                            .unwrap_or_else(|| "Cataloghi cloud non disponibili.".into())
                    })?
                    .client();
                let variants = client
                    .fetch_fontsource_details(family_id)
                    .await
                    .map_err(user_message)?;
                let variant = variants
                    .iter()
                    .find(|variant| variant.variant_id == variant_id)
                    .ok_or_else(|| format!("La variante Fontsource '{variant_id}' non esiste."))?;
                client
                    .download_fontsource_ttf(&variant.url)
                    .await
                    .map_err(user_message)?
            }
            RemoteFontProvider::Fontshare => {
                let item = self.resolve_remote_item(provider, family_id).await?;
                let url = item.download_url.as_deref().ok_or_else(|| {
                    "Fontshare non ha fornito un archivio scaricabile.".to_owned()
                })?;
                let bytes = self
                    .cloud
                    .as_ref()
                    .ok_or_else(|| {
                        self.cloud_error
                            .clone()
                            .unwrap_or_else(|| "Cataloghi cloud non disponibili.".into())
                    })?
                    .client()
                    .download_archive(provider, url)
                    .await
                    .map_err(user_message)?;
                self.cloud_importer
                    .preview_archive_font(&item, &bytes, variant_id)
                    .map_err(user_message)?
            }
            RemoteFontProvider::NerdFonts => {
                return Err("Nerd Fonts non espone un file di anteprima leggero.".into());
            }
        };
        parse_font_bytes(Path::new("remote-preview"), &bytes).map_err(user_message)?;
        Ok(bytes)
    }

    pub async fn download_remote_family(
        &self,
        provider: RemoteFontProvider,
        family_id: &str,
        variant_id: Option<&str>,
        activate: bool,
    ) -> Result<CloudFontImportOutcome, String> {
        let mut outcome = if provider == RemoteFontProvider::Google {
            let service = self.google.as_ref().ok_or_else(|| {
                self.google_error
                    .clone()
                    .unwrap_or_else(|| "Google Fonts non disponibile.".into())
            })?;
            if let Some(variant_id) = variant_id {
                let item = service
                    .install_variant(family_id, variant_id)
                    .await
                    .map_err(user_message)?;
                CloudFontImportOutcome {
                    report: RemoteFontInstallReport {
                        provider,
                        family_id: family_id.to_owned(),
                        family: item.family,
                        saved_files: 1,
                        indexed_fonts: 1,
                        activated_fonts: 0,
                        failures: Vec::new(),
                    },
                    font_ids: vec![item.id],
                }
            } else {
                let FamilyDownloadReport { installed, failed } = service
                    .download_family(family_id)
                    .await
                    .map_err(user_message)?;
                let family = installed
                    .first()
                    .map(|item| item.family.clone())
                    .unwrap_or_else(|| family_id.to_owned());
                CloudFontImportOutcome {
                    report: RemoteFontInstallReport {
                        provider,
                        family_id: family_id.to_owned(),
                        family,
                        saved_files: installed.len(),
                        indexed_fonts: installed.len(),
                        activated_fonts: 0,
                        failures: failed
                            .into_iter()
                            .map(|failure| RemoteFontFailure {
                                variant_id: failure.variant_id,
                                stage: "download".into(),
                                reason: failure.reason,
                            })
                            .collect(),
                    },
                    font_ids: installed.into_iter().map(|item| item.id).collect(),
                }
            }
        } else {
            let item = self.resolve_remote_item(provider, family_id).await?;
            let cloud = self.cloud.as_ref().ok_or_else(|| {
                self.cloud_error
                    .clone()
                    .unwrap_or_else(|| "Cataloghi cloud non disponibili.".into())
            })?;
            self.cloud_importer
                .import_catalog_item(cloud.client(), &item)
                .await
                .map_err(user_message)?
        };

        if activate {
            for font_id in &outcome.font_ids {
                match self.set_font_active(font_id, true) {
                    Ok(()) => outcome.report.activated_fonts += 1,
                    Err(reason) => outcome.report.failures.push(RemoteFontFailure {
                        variant_id: font_id.clone(),
                        stage: "activation".into(),
                        reason,
                    }),
                }
            }
        }
        Ok(outcome)
    }

    async fn resolve_remote_item(
        &self,
        provider: RemoteFontProvider,
        family_id: &str,
    ) -> Result<rust_font_manager::models::RemoteFontCatalogItem, String> {
        if let Some(snapshot) = self.load_cached_remote_catalog(provider)? {
            if let Some(item) = snapshot.items.into_iter().find(|item| item.id == family_id) {
                return Ok(item);
            }
        }
        self.refresh_remote_catalog(provider)
            .await?
            .items
            .into_iter()
            .find(|item| item.id == family_id)
            .ok_or_else(|| {
                format!("Il font '{family_id}' non è presente nel catalogo {provider:?}.")
            })
    }

    fn provider_available(&self, provider: RemoteFontProvider) -> bool {
        match provider {
            RemoteFontProvider::Google => self.google.is_some(),
            _ => self.cloud.is_some(),
        }
    }

    fn provider_error(&self, provider: RemoteFontProvider) -> Option<String> {
        match provider {
            RemoteFontProvider::Google => self.google_error.clone(),
            _ => self.cloud_error.clone(),
        }
    }
}

fn google_snapshot_to_remote(snapshot: CatalogSnapshot) -> RemoteFontCatalogSnapshot {
    RemoteFontCatalogSnapshot {
        provider: RemoteFontProvider::Google,
        items: map_google_catalog(&snapshot.items),
        fetched_at_unix_ms: snapshot.fetched_at_unix_ms,
        stale: snapshot.stale,
        last_error: snapshot.last_error,
    }
}

fn user_message(error: AppError) -> String {
    error.user_message()
}

fn mark_installed(fonts: &mut [FontItem], roots: &[PathBuf]) {
    for font in fonts {
        font.is_installed = font.is_active || is_installed_font_path(&font.file_path, roots);
    }
}

fn user_font_directory(fallback: &Path) -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| fallback.to_path_buf())
        .join("Microsoft")
        .join("Windows")
        .join("Fonts")
}
