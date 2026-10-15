use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use read_fonts::{
    tables::name::{Name, NameId},
    FileRef, FontData, FontRead,
};
use sha2::{Digest, Sha256};
use skrifa::{FontRef, MetadataProvider, Tag};
use walkdir::WalkDir;

use crate::{
    errors::AppError,
    models::{FontCategory, FontItem, FontSource, ParsedFontMetadata, ScanIssue, ScanReport},
};

/// Reports parseable font files below `root`; per-file errors do not abort traversal.
pub fn scan_directory(root: &Path) -> Result<ScanReport, AppError> {
    let (report, _, _) = scan_directory_with_items(root)?;
    Ok(report)
}

pub fn parse_font_file(path: &Path) -> Result<ParsedFontMetadata, AppError> {
    let bytes = fs::read(path).map_err(|error| AppError::Storage {
        path: path.to_path_buf(),
        reason: error.to_string(),
    })?;
    parse_font_bytes(path, &bytes)
}

pub fn parse_font_bytes(path: &Path, bytes: &[u8]) -> Result<ParsedFontMetadata, AppError> {
    parse_font_bytes_at_index(path, bytes, 0)
}

fn parse_font_bytes_at_index(
    path: &Path,
    bytes: &[u8],
    index: u32,
) -> Result<ParsedFontMetadata, AppError> {
    let font = FontRef::from_index(bytes, index).map_err(|error| AppError::FontParse {
        path: path.to_path_buf(),
        reason: format!("invalid font data at face {index}: {error:?}"),
    })?;
    let name_table = font
        .table_data(Tag::new(b"name"))
        .and_then(|data| Name::read(FontData::new(data.as_bytes())).ok())
        .ok_or_else(|| AppError::InvalidFontMetadata {
            path: path.to_path_buf(),
            reason: "missing or malformed name table".into(),
        })?;

    let family = preferred_name(&name_table, NameId::TYPOGRAPHIC_FAMILY_NAME)
        .or_else(|| preferred_name(&name_table, NameId::FAMILY_NAME))
        .filter(|name| !name.is_empty())
        .ok_or_else(|| AppError::InvalidFontMetadata {
            path: path.to_path_buf(),
            reason: "font has no usable family name (name ID 16 or 1)".into(),
        })?;
    let subfamily = preferred_name(&name_table, NameId::TYPOGRAPHIC_SUBFAMILY_NAME)
        .or_else(|| preferred_name(&name_table, NameId::SUBFAMILY_NAME))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Regular".into());
    let full_name = preferred_name(&name_table, NameId::FULL_NAME)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| format!("{family} {subfamily}"));

    let attributes = font.attributes();
    let weight_value = attributes.weight.value();
    let weight = if weight_value.is_finite() {
        weight_value.round().clamp(1.0, 1000.0) as u16
    } else {
        400
    };
    let is_italic = attributes.style != skrifa::attribute::Style::Normal;
    let is_variable = !font.axes().is_empty();
    let metadata = fs::metadata(path).ok();
    let file_modified_unix_ms = metadata
        .as_ref()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or_default();

    Ok(ParsedFontMetadata {
        family,
        subfamily,
        full_name,
        weight,
        is_italic,
        is_variable,
        file_size: metadata.map_or(bytes.len() as u64, |metadata| metadata.len()),
        file_modified_unix_ms,
    })
}

pub fn is_supported_font_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("ttf")
                || extension.eq_ignore_ascii_case("otf")
                || extension.eq_ignore_ascii_case("ttc")
        })
}

fn is_open_font_manager_install_copy(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let Some(file_name) = file_name.strip_prefix("OFM_") else {
        return false;
    };
    let Some((id_hash, extension)) = file_name.rsplit_once('.') else {
        return false;
    };
    id_hash.len() == 16
        && id_hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        && ["ttf", "otf", "ttc"]
            .iter()
            .any(|supported| extension.eq_ignore_ascii_case(supported))
}

/// Creates a stable SHA-256 identity from a canonical path and an optional variant ID.
pub fn font_id(canonical_path: &Path, variant_id: Option<&str>) -> Result<String, AppError> {
    let canonical = fs::canonicalize(canonical_path).map_err(|error| AppError::Storage {
        path: canonical_path.to_path_buf(),
        reason: error.to_string(),
    })?;
    let mut hasher = Sha256::new();
    if let Some(path) = canonical.to_str() {
        hasher.update(path.as_bytes());
    } else {
        hash_non_utf8_path(&mut hasher, &canonical)?;
    }
    hasher.update(b"\n");
    if let Some(variant_id) = variant_id {
        hasher.update(variant_id.as_bytes());
    }
    Ok(format!("{:x}", hasher.finalize()))
}

type ScanOutcome = (ScanReport, Vec<(PathBuf, FontItem)>, Vec<PathBuf>);

pub(crate) fn scan_directory_with_items(root: &Path) -> Result<ScanOutcome, AppError> {
    let root = fs::canonicalize(root).map_err(|error| AppError::Storage {
        path: root.to_path_buf(),
        reason: error.to_string(),
    })?;
    if !root.is_dir() {
        return Err(AppError::InvalidInput(format!(
            "Scan root is not a directory: {}",
            root.display()
        )));
    }

    let mut report = ScanReport {
        root: root.clone(),
        discovered_candidates: 0,
        imported: 0,
        updated: 0,
        skipped_unchanged: 0,
        skipped_unsupported: 0,
        failed: Vec::new(),
    };
    let mut items = Vec::new();
    let mut present_paths = Vec::new();

    for entry in WalkDir::new(&root).follow_links(false).into_iter() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                report.failed.push(ScanIssue {
                    path: error.path().unwrap_or(&root).to_path_buf(),
                    reason: error.to_string(),
                });
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if is_open_font_manager_install_copy(path) {
            continue;
        }
        if !is_supported_font_path(path) {
            report.skipped_unsupported += 1;
            continue;
        }
        report.discovered_candidates += 1;
        present_paths.push(path.to_path_buf());
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                report.failed.push(ScanIssue {
                    path: path.to_path_buf(),
                    reason: error.to_string(),
                });
                continue;
            }
        };
        let face_count = match FileRef::new(&bytes) {
            Ok(FileRef::Font(_)) => 1,
            Ok(FileRef::Collection(collection)) => collection.len(),
            Err(error) => {
                report.failed.push(ScanIssue {
                    path: path.to_path_buf(),
                    reason: format!("invalid font file: {error:?}"),
                });
                continue;
            }
        };
        let is_collection = face_count > 1
            || path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("ttc"));
        for index in 0..face_count {
            let metadata = match parse_font_bytes_at_index(path, &bytes, index) {
                Ok(metadata) => metadata,
                Err(error) => {
                    report.failed.push(ScanIssue {
                        path: path.to_path_buf(),
                        reason: error.to_string(),
                    });
                    continue;
                }
            };
            let variant_id = is_collection.then(|| format!("ttc-face-{index}"));
            match font_id(path, variant_id.as_deref()) {
                Ok(id) => items.push((
                    path.to_path_buf(),
                    FontItem {
                        id,
                        family: metadata.family,
                        subfamily: metadata.subfamily,
                        full_name: metadata.full_name,
                        weight: metadata.weight,
                        is_italic: metadata.is_italic,
                        is_variable: metadata.is_variable,
                        file_path: path.to_path_buf(),
                        source: FontSource::Local,
                        source_family_id: None,
                        source_variant_id: variant_id,
                        category: FontCategory::Unknown,
                        is_active: false,
                        is_installed: false,
                        is_favorite: false,
                        file_size: metadata.file_size,
                        file_modified_unix_ms: metadata.file_modified_unix_ms,
                        missing_on_disk: false,
                    },
                )),
                Err(error) => report.failed.push(ScanIssue {
                    path: path.to_path_buf(),
                    reason: error.to_string(),
                }),
            }
        }
    }
    Ok((report, items, present_paths))
}

fn preferred_name(table: &Name<'_>, name_id: NameId) -> Option<String> {
    table
        .name_record()
        .iter()
        .filter(|record| record.name_id() == name_id)
        .filter_map(|record| {
            let value = record.string(table.string_data()).ok()?.to_string();
            let value = value.trim();
            if value.is_empty() {
                return None;
            }
            let english_windows = record.platform_id() == 3 && record.language_id() == 0x0409;
            let unicode = matches!(record.platform_id(), 0 | 3);
            Some((!english_windows, !unicode, value.to_owned()))
        })
        .min_by_key(|(not_english_windows, not_unicode, _)| (*not_english_windows, *not_unicode))
        .map(|(_, _, value)| value)
}

fn hash_non_utf8_path(hasher: &mut Sha256, path: &Path) -> Result<(), AppError> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        hasher.update(b"windows-utf16le\0");
        for unit in path.as_os_str().encode_wide() {
            hasher.update(unit.to_le_bytes());
        }
        Ok(())
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        hasher.update(b"unix-os-bytes\0");
        hasher.update(path.as_os_str().as_bytes());
        Ok(())
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = hasher;
        Err(AppError::InvalidInput(format!(
            "Cannot hash non-Unicode path on this platform: {}",
            path.display()
        )))
    }
}
