use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;

use crate::{
    db,
    errors::AppError,
    font_scanner::parse_font_bytes,
    google_fonts::{CatalogResponse, GoogleFontsClient},
    models::{
        FontCategory, FontItem, FontSource, GoogleFontCatalogItem, GoogleFontDetails,
        GoogleFontVariant, ParsedFontMetadata,
    },
    paths::AppPaths,
};

const CATALOG_SCHEMA_VERSION: u32 = 1;
const CATALOG_MAX_AGE_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedCatalog {
    pub schema_version: u32,
    pub fetched_at_unix_ms: i64,
    pub etag: Option<String>,
    pub items: Vec<GoogleFontCatalogItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSnapshot {
    pub items: Vec<GoogleFontCatalogItem>,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FamilyDownloadFailure {
    pub variant_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FamilyDownloadReport {
    pub installed: Vec<FontItem>,
    pub failed: Vec<FamilyDownloadFailure>,
}

#[derive(Debug, Clone)]
pub struct GoogleLibraryService {
    client: GoogleFontsClient,
    paths: AppPaths,
}

impl GoogleLibraryService {
    pub fn new(client: GoogleFontsClient, paths: AppPaths) -> Self {
        Self { client, paths }
    }

    pub fn load_cached_catalog(&self) -> Result<Option<CatalogSnapshot>, AppError> {
        let Some(catalog) = self.read_cache()? else {
            return Ok(None);
        };
        let stale = now_ms().saturating_sub(catalog.fetched_at_unix_ms) >= CATALOG_MAX_AGE_MS;
        Ok(Some(CatalogSnapshot {
            items: catalog.items,
            stale,
        }))
    }

    pub async fn refresh_catalog(&self) -> Result<CatalogSnapshot, AppError> {
        let existing = self.read_cache()?;
        let etag = existing
            .as_ref()
            .and_then(|catalog| catalog.etag.as_deref());
        let response = match self.client.fetch_catalog(etag).await {
            Ok(response) => response,
            Err(error) => {
                if let Some(catalog) = existing {
                    return Ok(CatalogSnapshot {
                        items: catalog.items,
                        stale: true,
                    });
                }
                return Err(error);
            }
        };
        let next = match response {
            CatalogResponse::NotModified => {
                let mut catalog = existing.ok_or_else(|| AppError::RemoteFormat {
                    endpoint: "https://gwfh.mranftl.com/api/fonts".into(),
                    reason: "server returned 304 without a cached catalog".into(),
                })?;
                catalog.fetched_at_unix_ms = now_ms();
                catalog
            }
            CatalogResponse::Updated { items, etag } => CachedCatalog {
                schema_version: CATALOG_SCHEMA_VERSION,
                fetched_at_unix_ms: now_ms(),
                etag,
                items,
            },
        };
        self.write_cache(&next)?;
        Ok(CatalogSnapshot {
            items: next.items,
            stale: false,
        })
    }

    pub async fn preview_variant(
        &self,
        family_id: &str,
        variant_id: Option<&str>,
    ) -> Result<(Vec<u8>, ParsedFontMetadata), AppError> {
        let details = self.client.fetch_details(family_id).await?;
        let variant = select_variant(&details, variant_id)?;
        let cache_path = self.preview_path(family_id, &variant.id);
        if let Ok(bytes) = fs::read(&cache_path) {
            if let Ok(metadata) =
                parse_font_bytes(Path::new("<Google Fonts preview cache>"), &bytes)
            {
                return Ok((bytes, metadata));
            }
        }
        let url = variant
            .ttf
            .as_deref()
            .ok_or_else(|| AppError::RemoteFormat {
                endpoint: format!("https://gwfh.mranftl.com/api/fonts/{family_id}"),
                reason: format!("variant '{}' has no TTF download URL", variant.id),
            })?;
        let bytes = self.client.download_ttf(url).await?;
        let metadata = parse_font_bytes(Path::new("<Google Fonts preview>"), &bytes)?;
        atomic_write(&cache_path, &bytes)?;
        Ok((bytes, metadata))
    }

    pub async fn install_variant(
        &self,
        family_id: &str,
        variant_id: &str,
    ) -> Result<FontItem, AppError> {
        let details = self.client.fetch_details(family_id).await?;
        let variant = select_variant(&details, Some(variant_id))?;
        self.install_details_variant(&details, variant).await
    }

    pub async fn download_family(&self, family_id: &str) -> Result<FamilyDownloadReport, AppError> {
        let details = self.client.fetch_details(family_id).await?;
        let mut report = FamilyDownloadReport {
            installed: Vec::new(),
            failed: Vec::new(),
        };
        for variant in &details.variants {
            if variant.ttf.is_none() {
                report.failed.push(FamilyDownloadFailure {
                    variant_id: variant.id.clone(),
                    reason: "variant has no TTF download URL".into(),
                });
                continue;
            }
            match self.install_details_variant(&details, variant).await {
                Ok(item) => report.installed.push(item),
                Err(error) => report.failed.push(FamilyDownloadFailure {
                    variant_id: variant.id.clone(),
                    reason: error.to_string(),
                }),
            }
        }
        Ok(report)
    }

    async fn install_details_variant(
        &self,
        details: &GoogleFontDetails,
        variant: &GoogleFontVariant,
    ) -> Result<FontItem, AppError> {
        let family_id = details.id.as_str();
        let url = variant
            .ttf
            .as_deref()
            .ok_or_else(|| AppError::RemoteFormat {
                endpoint: format!("https://gwfh.mranftl.com/api/fonts/{family_id}"),
                reason: format!("variant '{}' has no TTF download URL", variant.id),
            })?;
        let bytes = self.client.download_ttf(url).await?;
        let target = self.vault_path(family_id, &variant.id)?;
        let metadata = parse_font_bytes(&target, &bytes)?;
        atomic_write(&target, &bytes)?;

        let file_metadata = fs::metadata(&target).map_err(|error| AppError::Storage {
            path: target.clone(),
            reason: error.to_string(),
        })?;
        let file_modified_unix_ms = file_metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| i64::try_from(duration.as_millis()).ok())
            .unwrap_or_default();
        let item = FontItem {
            id: google_font_id(family_id, &variant.id),
            family: metadata.family,
            subfamily: metadata.subfamily,
            full_name: metadata.full_name,
            weight: variant
                .font_weight
                .parse::<u16>()
                .ok()
                .filter(|weight| (1..=1000).contains(weight))
                .unwrap_or(metadata.weight),
            is_italic: variant.font_style.eq_ignore_ascii_case("italic") || metadata.is_italic,
            file_path: target,
            source: FontSource::Google,
            source_family_id: Some(family_id.to_owned()),
            source_variant_id: Some(variant.id.clone()),
            category: normalize_category(&details.category),
            is_active: false,
            is_favorite: false,
            file_size: file_metadata.len(),
            file_modified_unix_ms,
            missing_on_disk: false,
        };
        db::migrate(&self.paths.database)?;
        db::upsert_font(&self.paths.database, &item)?;
        Ok(item)
    }

    fn read_cache(&self) -> Result<Option<CachedCatalog>, AppError> {
        let bytes = match fs::read(&self.paths.catalog_cache) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(AppError::Storage {
                    path: self.paths.catalog_cache.clone(),
                    reason: error.to_string(),
                })
            }
        };
        let catalog: CachedCatalog =
            serde_json::from_slice(&bytes).map_err(|error| AppError::RemoteFormat {
                endpoint: self.paths.catalog_cache.display().to_string(),
                reason: error.to_string(),
            })?;
        if catalog.schema_version != CATALOG_SCHEMA_VERSION {
            return Err(AppError::RemoteFormat {
                endpoint: self.paths.catalog_cache.display().to_string(),
                reason: format!(
                    "unsupported catalog cache schema {}",
                    catalog.schema_version
                ),
            });
        }
        Ok(Some(catalog))
    }

    fn write_cache(&self, catalog: &CachedCatalog) -> Result<(), AppError> {
        let bytes = serde_json::to_vec(catalog).map_err(|error| AppError::RemoteFormat {
            endpoint: self.paths.catalog_cache.display().to_string(),
            reason: error.to_string(),
        })?;
        atomic_write(&self.paths.catalog_cache, &bytes)?;
        let meta = serde_json::json!({
            "schema_version": catalog.schema_version,
            "fetched_at_unix_ms": catalog.fetched_at_unix_ms,
            "etag": catalog.etag,
        });
        let meta_bytes = serde_json::to_vec(&meta).map_err(|error| AppError::RemoteFormat {
            endpoint: self.paths.catalog_meta.display().to_string(),
            reason: error.to_string(),
        })?;
        atomic_write(&self.paths.catalog_meta, &meta_bytes)
    }

    fn preview_path(&self, family_id: &str, variant_id: &str) -> PathBuf {
        self.paths
            .preview_cache
            .join(format!("{}.ttf", cache_key(family_id, variant_id)))
    }

    fn vault_path(&self, family_id: &str, variant_id: &str) -> Result<PathBuf, AppError> {
        validate_cache_id(family_id)?;
        validate_cache_id(variant_id)?;
        Ok(self
            .paths
            .google_vault
            .join(family_id)
            .join(format!("{variant_id}.ttf")))
    }
}

fn select_variant<'a>(
    details: &'a GoogleFontDetails,
    requested_id: Option<&str>,
) -> Result<&'a GoogleFontVariant, AppError> {
    if let Some(id) = requested_id {
        return details
            .variants
            .iter()
            .find(|variant| variant.id == id)
            .ok_or_else(|| {
                AppError::NotFound(format!("Google Fonts variant '{id}' was not found."))
            });
    }
    details
        .variants
        .iter()
        .find(|variant| variant.id == details.default_variant && variant.ttf.is_some())
        .or_else(|| {
            details
                .variants
                .iter()
                .find(|variant| variant.id == "regular" && variant.ttf.is_some())
        })
        .or_else(|| {
            details
                .variants
                .iter()
                .find(|variant| variant.ttf.is_some())
        })
        .ok_or_else(|| AppError::RemoteFormat {
            endpoint: format!("https://gwfh.mranftl.com/api/fonts/{}", details.id),
            reason: "font has no variant with a TTF download URL".into(),
        })
}

fn normalize_category(category: &str) -> FontCategory {
    match category.trim().to_ascii_lowercase().as_str() {
        "sans-serif" => FontCategory::SansSerif,
        "serif" => FontCategory::Serif,
        "display" => FontCategory::Display,
        "monospace" => FontCategory::Monospace,
        _ => FontCategory::Unknown,
    }
}

fn google_font_id(family_id: &str, variant_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"google\n");
    hasher.update(family_id.as_bytes());
    hasher.update(b"\n");
    hasher.update(variant_id.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn cache_key(family_id: &str, variant_id: &str) -> String {
    google_font_id(family_id, variant_id)
}

fn validate_cache_id(value: &str) -> Result<(), AppError> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(AppError::InvalidInput(format!(
            "Invalid Google Fonts ID: {value}"
        )));
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(|| {
        AppError::InvalidInput(format!(
            "Target path has no parent directory: {}",
            path.display()
        ))
    })?;
    fs::create_dir_all(parent).map_err(|error| AppError::Storage {
        path: parent.to_path_buf(),
        reason: error.to_string(),
    })?;
    let mut temporary = NamedTempFile::new_in(parent).map_err(|error| AppError::Storage {
        path: parent.to_path_buf(),
        reason: error.to_string(),
    })?;
    temporary
        .write_all(bytes)
        .map_err(|error| AppError::Storage {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| AppError::Storage {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
    temporary.persist(path).map_err(|error| AppError::Storage {
        path: path.to_path_buf(),
        reason: error.error.to_string(),
    })?;
    Ok(())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or_default()
}
