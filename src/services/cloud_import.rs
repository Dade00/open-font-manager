use std::{
    collections::HashSet,
    io::{Cursor, Read, Write},
    path::{Component, Path, PathBuf},
};

use futures::{stream, StreamExt};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use zip::ZipArchive;

use crate::{
    cloud_fonts::{CloudFontsClient, FontsourceVariantFile},
    errors::AppError,
    font_scanner::parse_font_bytes,
    models::{
        FontDownloadStrategy, FontSource, RemoteFontCatalogItem, RemoteFontFailure,
        RemoteFontInstallReport, RemoteFontProvider,
    },
    services::local_library::LocalLibraryService,
};

const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 8192;
const MAX_FONT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_FONT_BYTES: u64 = 512 * 1024 * 1024;
const MAX_CONCURRENT_DIRECT_DOWNLOADS: usize = 3;
const MAX_DIRECT_VARIANTS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudFontImportOutcome {
    pub report: RemoteFontInstallReport,
    pub font_ids: Vec<String>,
}

#[derive(Debug)]
pub struct CloudFontImporter {
    vault_root: PathBuf,
    library: LocalLibraryService,
}

impl CloudFontImporter {
    pub fn new(vault_root: impl Into<PathBuf>, database_path: impl Into<PathBuf>) -> Self {
        Self {
            vault_root: vault_root.into(),
            library: LocalLibraryService::new(database_path),
        }
    }

    pub async fn import_catalog_item(
        &self,
        client: &CloudFontsClient,
        item: &RemoteFontCatalogItem,
    ) -> Result<CloudFontImportOutcome, AppError> {
        validate_item(item)?;
        let mut report = RemoteFontInstallReport {
            provider: item.provider,
            family_id: item.id.clone(),
            family: item.family.clone(),
            saved_files: 0,
            indexed_fonts: 0,
            activated_fonts: 0,
            failures: Vec::new(),
        };

        match item.download_strategy {
            FontDownloadStrategy::DirectFiles
                if item.provider == RemoteFontProvider::FontsourceExtra =>
            {
                let variants = client.fetch_fontsource_details(&item.id).await?;
                self.download_direct_variants(client, item, variants, &mut report)
                    .await;
            }
            FontDownloadStrategy::ZipArchive
                if matches!(
                    item.provider,
                    RemoteFontProvider::Fontshare | RemoteFontProvider::NerdFonts
                ) =>
            {
                let url = item.download_url.as_deref().ok_or_else(|| {
                    AppError::InvalidInput("Remote font catalog item has no download URL.".into())
                })?;
                let bytes = client.download_archive(item.provider, url).await?;
                return self.import_archive(item, &bytes);
            }
            _ => {
                return Err(AppError::InvalidInput(
                    "Remote font provider and download strategy do not match.".into(),
                ));
            }
        }

        self.index_saved_fonts(item, report)
    }

    pub fn import_archive(
        &self,
        item: &RemoteFontCatalogItem,
        bytes: &[u8],
    ) -> Result<CloudFontImportOutcome, AppError> {
        validate_item(item)?;
        if item.download_strategy != FontDownloadStrategy::ZipArchive {
            return Err(AppError::InvalidInput(
                "In-memory ZIP import requires a ZIP archive provider.".into(),
            ));
        }
        let mut report = RemoteFontInstallReport {
            provider: item.provider,
            family_id: item.id.clone(),
            family: item.family.clone(),
            saved_files: 0,
            indexed_fonts: 0,
            activated_fonts: 0,
            failures: Vec::new(),
        };
        self.import_archive_bytes(item, bytes, &mut report)?;
        if report.saved_files == 0 && report.failures.is_empty() {
            report.failures.push(RemoteFontFailure {
                variant_id: item.id.clone(),
                stage: "archive-content".into(),
                reason: "L'archivio non contiene font TTF o OTF validi.".into(),
            });
        }
        self.index_saved_fonts(item, report)
    }

    pub fn preview_archive_font(
        &self,
        item: &RemoteFontCatalogItem,
        bytes: &[u8],
        variant_id: &str,
    ) -> Result<Vec<u8>, AppError> {
        validate_item(item)?;
        if item.provider != RemoteFontProvider::Fontshare
            || item.download_strategy != FontDownloadStrategy::ZipArchive
        {
            return Err(AppError::InvalidInput(
                "Archive preview is supported only for Fontshare.".into(),
            ));
        }
        if bytes.len() as u64 > MAX_ARCHIVE_BYTES {
            return Err(AppError::DownloadTooLarge {
                url: item.download_url.clone().unwrap_or_else(|| item.id.clone()),
                limit_bytes: MAX_ARCHIVE_BYTES,
            });
        }
        let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| {
            AppError::InvalidInput("L'archivio ZIP remoto non è valido o è danneggiato.".into())
        })?;
        if archive.len() > MAX_ARCHIVE_ENTRIES {
            return Err(AppError::InvalidInput(
                "L'archivio ZIP remoto contiene troppi elementi.".into(),
            ));
        }
        let mut total_font_bytes = 0_u64;
        let mut fallback = None;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|_| {
                AppError::InvalidInput("Impossibile leggere un elemento del ZIP remoto.".into())
            })?;
            if entry.is_dir() || is_symlink(entry.unix_mode()) || entry.size() > MAX_FONT_BYTES {
                continue;
            }
            let Some(relative_path) = entry.enclosed_name() else {
                continue;
            };
            if is_metadata_path(&relative_path) || !is_supported_font(&relative_path) {
                continue;
            }
            if total_font_bytes.saturating_add(entry.size()) > MAX_TOTAL_FONT_BYTES {
                return Err(AppError::InvalidInput(
                    "I font decompressi superano il limite complessivo di 512 MiB.".into(),
                ));
            }
            let mut font_bytes = Vec::with_capacity(entry.size() as usize);
            entry
                .by_ref()
                .take(MAX_FONT_BYTES + 1)
                .read_to_end(&mut font_bytes)
                .map_err(|error| {
                    AppError::InvalidInput(format!(
                        "Impossibile decomprimere il font di anteprima: {error}"
                    ))
                })?;
            total_font_bytes = total_font_bytes.saturating_add(font_bytes.len() as u64);
            if font_bytes.len() as u64 > MAX_FONT_BYTES || total_font_bytes > MAX_TOTAL_FONT_BYTES {
                return Err(AppError::InvalidInput(
                    "I font decompressi superano i limiti dell'anteprima.".into(),
                ));
            }
            let extension = relative_path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("ttf");
            if let Ok(metadata) = parse_font_bytes(
                Path::new("fontshare-preview")
                    .with_extension(extension)
                    .as_path(),
                &font_bytes,
            ) {
                let wanted_weight = variant_id
                    .split('-')
                    .next()
                    .and_then(|value| value.parse::<u16>().ok());
                let wanted_italic = variant_id.ends_with("italic");
                if wanted_weight == Some(metadata.weight) && wanted_italic == metadata.is_italic {
                    return Ok(font_bytes);
                }
                if fallback.is_none() {
                    fallback = Some(font_bytes);
                }
            }
        }
        fallback.ok_or_else(|| {
            AppError::InvalidInput("Fontshare non contiene un font valido per l'anteprima.".into())
        })
    }

    fn index_saved_fonts(
        &self,
        item: &RemoteFontCatalogItem,
        mut report: RemoteFontInstallReport,
    ) -> Result<CloudFontImportOutcome, AppError> {
        let family_directory = self.family_directory(item)?;
        if family_directory.is_dir() {
            match self.library.index_cloud_directory(
                &family_directory,
                source_for(item.provider)?,
                &item.id,
            ) {
                Ok((scan, font_ids)) => {
                    report.indexed_fonts = font_ids.len();
                    report.failures.extend(scan.failed.into_iter().map(|issue| {
                        RemoteFontFailure {
                            variant_id: issue
                                .path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or("unknown")
                                .to_owned(),
                            stage: "index".into(),
                            reason: issue.reason,
                        }
                    }));
                    return Ok(CloudFontImportOutcome { report, font_ids });
                }
                Err(error) => report.failures.push(RemoteFontFailure {
                    variant_id: item.id.clone(),
                    stage: "index".into(),
                    reason: error.user_message(),
                }),
            }
        }
        Ok(CloudFontImportOutcome {
            report,
            font_ids: Vec::new(),
        })
    }

    pub fn import_fontsource_files(
        &self,
        item: &RemoteFontCatalogItem,
        files: Vec<(String, Vec<u8>)>,
    ) -> Result<CloudFontImportOutcome, AppError> {
        validate_item(item)?;
        if item.provider != RemoteFontProvider::FontsourceExtra
            || item.download_strategy != FontDownloadStrategy::DirectFiles
        {
            return Err(AppError::InvalidInput(
                "Direct file import requires a Fontsource Extra item.".into(),
            ));
        }
        let mut report = RemoteFontInstallReport {
            provider: item.provider,
            family_id: item.id.clone(),
            family: item.family.clone(),
            saved_files: 0,
            indexed_fonts: 0,
            activated_fonts: 0,
            failures: Vec::new(),
        };
        let mut saved_digests = HashSet::new();
        let mut total_bytes = 0_u64;
        let mut files = files;
        if files.len() > MAX_DIRECT_VARIANTS {
            files.truncate(MAX_DIRECT_VARIANTS);
            report.failures.push(RemoteFontFailure {
                variant_id: item.id.clone(),
                stage: "download-limit".into(),
                reason: format!("Superato il limite di {MAX_DIRECT_VARIANTS} varianti."),
            });
        }
        for (variant_id, bytes) in files {
            if total_bytes.saturating_add(bytes.len() as u64) > MAX_TOTAL_FONT_BYTES {
                report.failures.push(RemoteFontFailure {
                    variant_id,
                    stage: "download-limit".into(),
                    reason: "I file font superano il limite complessivo di 512 MiB.".into(),
                });
                break;
            }
            total_bytes = total_bytes.saturating_add(bytes.len() as u64);
            if bytes.len() as u64 > MAX_FONT_BYTES {
                report.failures.push(RemoteFontFailure {
                    variant_id,
                    stage: "download-limit".into(),
                    reason: "Il file font supera il limite di 64 MiB.".into(),
                });
                continue;
            }
            if !saved_digests.insert(digest_hex(&bytes)) {
                continue;
            }
            match self.save_validated_font(item, "ttf", &bytes) {
                Ok(_) => report.saved_files += 1,
                Err(error) => report.failures.push(RemoteFontFailure {
                    variant_id,
                    stage: "validation-or-write".into(),
                    reason: error.user_message(),
                }),
            }
        }
        if report.saved_files == 0 && report.failures.is_empty() {
            report.failures.push(RemoteFontFailure {
                variant_id: item.id.clone(),
                stage: "download".into(),
                reason: "Il provider non ha restituito file font validi.".into(),
            });
        }
        self.index_saved_fonts(item, report)
    }

    async fn download_direct_variants(
        &self,
        client: &CloudFontsClient,
        item: &RemoteFontCatalogItem,
        variants: Vec<FontsourceVariantFile>,
        report: &mut RemoteFontInstallReport,
    ) {
        let mut variants = variants;
        if variants.len() > MAX_DIRECT_VARIANTS {
            variants.truncate(MAX_DIRECT_VARIANTS);
            report.failures.push(RemoteFontFailure {
                variant_id: item.id.clone(),
                stage: "download-limit".into(),
                reason: format!("Superato il limite di {MAX_DIRECT_VARIANTS} varianti."),
            });
        }
        let downloads = stream::iter(variants.into_iter().map(|variant| {
            let client = client.clone();
            async move {
                let result = client.download_fontsource_ttf(&variant.url).await;
                (variant, result)
            }
        }))
        .buffer_unordered(MAX_CONCURRENT_DIRECT_DOWNLOADS);
        futures::pin_mut!(downloads);
        let mut saved_digests = HashSet::new();
        let mut total_bytes = 0_u64;
        while let Some((variant, result)) = downloads.next().await {
            match result {
                Ok(bytes) => {
                    if bytes.len() as u64 > MAX_FONT_BYTES {
                        report.failures.push(RemoteFontFailure {
                            variant_id: variant.variant_id,
                            stage: "download-limit".into(),
                            reason: "Il file font supera il limite di 64 MiB.".into(),
                        });
                        continue;
                    }
                    if total_bytes.saturating_add(bytes.len() as u64) > MAX_TOTAL_FONT_BYTES {
                        report.failures.push(RemoteFontFailure {
                            variant_id: variant.variant_id,
                            stage: "download-limit".into(),
                            reason: "I file font superano il limite complessivo di 512 MiB.".into(),
                        });
                        break;
                    }
                    total_bytes = total_bytes.saturating_add(bytes.len() as u64);
                    let digest = digest_hex(&bytes);
                    if !saved_digests.insert(digest) {
                        continue;
                    }
                    match self.save_validated_font(item, "ttf", &bytes) {
                        Ok(_) => report.saved_files += 1,
                        Err(error) => report.failures.push(RemoteFontFailure {
                            variant_id: variant.variant_id,
                            stage: "validation-or-write".into(),
                            reason: error.user_message(),
                        }),
                    }
                }
                Err(error) => report.failures.push(RemoteFontFailure {
                    variant_id: variant.variant_id,
                    stage: "download".into(),
                    reason: error.user_message(),
                }),
            }
        }
    }

    fn import_archive_bytes(
        &self,
        item: &RemoteFontCatalogItem,
        bytes: &[u8],
        report: &mut RemoteFontInstallReport,
    ) -> Result<(), AppError> {
        if bytes.len() as u64 > MAX_ARCHIVE_BYTES {
            return Err(AppError::DownloadTooLarge {
                url: item.download_url.clone().unwrap_or_else(|| item.id.clone()),
                limit_bytes: MAX_ARCHIVE_BYTES,
            });
        }
        let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| {
            AppError::InvalidInput("L'archivio ZIP remoto non è valido o è danneggiato.".into())
        })?;
        if archive.len() > MAX_ARCHIVE_ENTRIES {
            return Err(AppError::InvalidInput(
                "L'archivio ZIP remoto contiene troppi elementi.".into(),
            ));
        }

        let mut total_font_bytes = 0_u64;
        let mut saved_digests = HashSet::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|_| {
                AppError::InvalidInput("Impossibile leggere un elemento del ZIP remoto.".into())
            })?;
            if entry.is_dir() || is_symlink(entry.unix_mode()) {
                continue;
            }
            let Some(relative_path) = entry.enclosed_name() else {
                report.failures.push(RemoteFontFailure {
                    variant_id: entry.name().to_owned(),
                    stage: "archive-path".into(),
                    reason: "Percorso ZIP assoluto o non sicuro, elemento ignorato.".into(),
                });
                continue;
            };
            if is_metadata_path(&relative_path) || !is_supported_font(&relative_path) {
                continue;
            }
            let entry_label = relative_path.to_string_lossy().into_owned();
            if item.provider == RemoteFontProvider::NerdFonts
                && is_excluded_nerd_font_path(&entry_label)
            {
                continue;
            }
            if entry.size() > MAX_FONT_BYTES {
                report.failures.push(RemoteFontFailure {
                    variant_id: entry_label,
                    stage: "archive-limit".into(),
                    reason: "Il file font supera il limite di 64 MiB.".into(),
                });
                continue;
            }
            let extension = relative_path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            let mut font_bytes = Vec::with_capacity(entry.size() as usize);
            if let Err(error) = entry
                .by_ref()
                .take(MAX_FONT_BYTES + 1)
                .read_to_end(&mut font_bytes)
            {
                report.failures.push(RemoteFontFailure {
                    variant_id: entry_label,
                    stage: "decompression".into(),
                    reason: error.to_string(),
                });
                continue;
            }
            if font_bytes.len() as u64 > MAX_FONT_BYTES {
                report.failures.push(RemoteFontFailure {
                    variant_id: entry_label,
                    stage: "archive-limit".into(),
                    reason: "Il file decompresso supera il limite di 64 MiB.".into(),
                });
                continue;
            }
            if total_font_bytes.saturating_add(font_bytes.len() as u64) > MAX_TOTAL_FONT_BYTES {
                report.failures.push(RemoteFontFailure {
                    variant_id: entry_label,
                    stage: "archive-limit".into(),
                    reason: "I font decompressi superano il limite complessivo di 512 MiB.".into(),
                });
                break;
            }
            total_font_bytes = total_font_bytes.saturating_add(font_bytes.len() as u64);
            let digest = digest_hex(&font_bytes);
            if !saved_digests.insert(digest) {
                continue;
            }
            match self.save_validated_font(item, &extension, &font_bytes) {
                Ok(_) => report.saved_files += 1,
                Err(error) => report.failures.push(RemoteFontFailure {
                    variant_id: entry_label,
                    stage: "validation-or-write".into(),
                    reason: error.user_message(),
                }),
            }
        }
        Ok(())
    }

    fn save_validated_font(
        &self,
        item: &RemoteFontCatalogItem,
        extension: &str,
        bytes: &[u8],
    ) -> Result<PathBuf, AppError> {
        if !matches!(extension, "ttf" | "otf") {
            return Err(AppError::InvalidInput(
                "Formato font non supportato.".into(),
            ));
        }
        let directory = self.family_directory(item)?;
        let candidate = Path::new("validated-font").with_extension(extension);
        parse_font_bytes(&candidate, bytes)?;
        let destination = directory.join(format!("{}.{}", digest_hex(bytes), extension));
        atomic_write(&destination, bytes)?;
        Ok(destination)
    }

    fn family_directory(&self, item: &RemoteFontCatalogItem) -> Result<PathBuf, AppError> {
        validate_safe_id(&item.id)?;
        let provider = match item.provider {
            RemoteFontProvider::Google => {
                return Err(AppError::InvalidInput(
                    "Google Fonts must use its existing import service.".into(),
                ));
            }
            RemoteFontProvider::FontsourceExtra => "fontsource",
            RemoteFontProvider::Fontshare => "fontshare",
            RemoteFontProvider::NerdFonts => "nerd-fonts",
        };
        Ok(self.vault_root.join(provider).join(&item.id))
    }
}

fn validate_item(item: &RemoteFontCatalogItem) -> Result<(), AppError> {
    validate_safe_id(&item.id)?;
    if item.family.trim().is_empty() || item.family.len() > 256 {
        return Err(AppError::InvalidInput(
            "Invalid remote font family name.".into(),
        ));
    }
    match (item.provider, item.download_strategy) {
        (RemoteFontProvider::FontsourceExtra, FontDownloadStrategy::DirectFiles)
        | (
            RemoteFontProvider::Fontshare | RemoteFontProvider::NerdFonts,
            FontDownloadStrategy::ZipArchive,
        ) => Ok(()),
        _ => Err(AppError::InvalidInput(
            "Remote font provider and download strategy do not match.".into(),
        )),
    }
}

fn source_for(provider: RemoteFontProvider) -> Result<FontSource, AppError> {
    match provider {
        RemoteFontProvider::FontsourceExtra => Ok(FontSource::Fontsource),
        RemoteFontProvider::Fontshare => Ok(FontSource::Fontshare),
        RemoteFontProvider::NerdFonts => Ok(FontSource::NerdFonts),
        RemoteFontProvider::Google => Err(AppError::InvalidInput(
            "Google Fonts must use its existing import service.".into(),
        )),
    }
}

fn validate_safe_id(value: &str) -> Result<(), AppError> {
    if value.is_empty()
        || value.len() > 128
        || value == "."
        || value == ".."
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(AppError::InvalidInput(
            "Invalid remote font identifier.".into(),
        ));
    }
    Ok(())
}

fn is_supported_font(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("ttf") || extension.eq_ignore_ascii_case("otf")
        })
}

fn is_metadata_path(path: &Path) -> bool {
    path.components().any(|component| {
        let Component::Normal(name) = component else {
            return true;
        };
        let name = name.to_string_lossy();
        name.starts_with('.')
            || name.eq_ignore_ascii_case("__MACOSX")
            || name.eq_ignore_ascii_case("Thumbs.db")
            || name.eq_ignore_ascii_case("desktop.ini")
            || name.eq_ignore_ascii_case("System Volume Information")
            || name.eq_ignore_ascii_case("$RECYCLE.BIN")
    })
}

fn is_excluded_nerd_font_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("windows compatible") || lower.contains("nerdfontpropo")
}

fn is_symlink(unix_mode: Option<u32>) -> bool {
    unix_mode.is_some_and(|mode| mode & 0o170000 == 0o120000)
}

fn digest_hex(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

fn atomic_write(destination: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = destination.parent().ok_or_else(|| AppError::Storage {
        path: destination.to_path_buf(),
        reason: "vault font path has no parent directory".into(),
    })?;
    std::fs::create_dir_all(parent).map_err(|error| storage_error(parent, error))?;
    let mut temporary =
        NamedTempFile::new_in(parent).map_err(|error| storage_error(parent, error))?;
    temporary
        .write_all(bytes)
        .map_err(|error| storage_error(destination, error))?;
    temporary
        .persist(destination)
        .map_err(|error| storage_error(destination, error.error))?;
    Ok(())
}

fn storage_error(path: &Path, error: std::io::Error) -> AppError {
    AppError::Storage {
        path: path.to_path_buf(),
        reason: error.to_string(),
    }
}
