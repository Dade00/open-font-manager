use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

use sha2::{Digest, Sha256};
use tempfile::TempDir;
use zip::ZipArchive;

use crate::{errors::AppError, font_scanner::parse_font_bytes};

const MAX_ARCHIVE_BYTES: u64 = 250 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 4096;
const MAX_FONT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_FONT_BYTES: u64 = 512 * 1024 * 1024;

pub fn extract_font_archive(archive_path: &Path, imports_root: &Path) -> Result<PathBuf, AppError> {
    let extension_is_zip = archive_path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"));
    if !extension_is_zip {
        return Err(AppError::InvalidInput(
            "Trascina un archivio .zip contenente font TTF o OTF.".into(),
        ));
    }

    let metadata =
        fs::metadata(archive_path).map_err(|error| storage_error(archive_path, error))?;
    if !metadata.is_file() {
        return Err(AppError::InvalidInput(
            "Il percorso trascinato non è un file ZIP.".into(),
        ));
    }
    if metadata.len() > MAX_ARCHIVE_BYTES {
        return Err(AppError::InvalidInput(
            "L'archivio ZIP supera il limite di 250 MiB.".into(),
        ));
    }

    let archive_id = hash_file(archive_path)?;
    fs::create_dir_all(imports_root).map_err(|error| storage_error(imports_root, error))?;
    let destination = imports_root.join(archive_id);
    if destination.is_dir() {
        return Ok(destination);
    }

    let file = File::open(archive_path).map_err(|error| storage_error(archive_path, error))?;
    let mut archive = ZipArchive::new(file).map_err(|_| {
        AppError::InvalidInput("L'archivio ZIP non è valido o è danneggiato.".into())
    })?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(AppError::InvalidInput(
            "L'archivio ZIP contiene troppi elementi (limite: 4096).".into(),
        ));
    }

    let staging =
        TempDir::new_in(imports_root).map_err(|error| storage_error(imports_root, error))?;
    let mut extracted_paths = HashSet::new();
    let mut total_bytes = 0_u64;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|_| {
            AppError::InvalidInput("Impossibile leggere un elemento dell'archivio ZIP.".into())
        })?;
        if entry.is_dir() || is_symlink(entry.unix_mode()) {
            continue;
        }

        let Some(relative_path) = entry.enclosed_name() else {
            continue;
        };
        if !is_supported_font(&relative_path) || is_metadata_path(&relative_path) {
            continue;
        }
        if entry.size() > MAX_FONT_BYTES {
            continue;
        }

        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry
            .by_ref()
            .take(MAX_FONT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| {
                AppError::InvalidInput("Un font nell'archivio ZIP è danneggiato.".into())
            })?;
        if bytes.len() as u64 > MAX_FONT_BYTES {
            continue;
        }
        if total_bytes.saturating_add(bytes.len() as u64) > MAX_TOTAL_FONT_BYTES {
            return Err(AppError::InvalidInput(
                "I font nell'archivio superano il limite complessivo di 512 MiB.".into(),
            ));
        }

        let output_path = staging.path().join(&relative_path);
        if extracted_paths.contains(&relative_path) {
            continue;
        }
        if parse_font_bytes(&output_path, &bytes).is_err() {
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|error| storage_error(parent, error))?;
        }
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output_path)
            .map_err(|error| storage_error(&output_path, error))?;
        output
            .write_all(&bytes)
            .map_err(|error| storage_error(&output_path, error))?;
        extracted_paths.insert(relative_path);
        total_bytes = total_bytes.saturating_add(bytes.len() as u64);
    }

    if extracted_paths.is_empty() {
        return Err(AppError::InvalidInput(
            "L'archivio non contiene font TTF o OTF validi.".into(),
        ));
    }

    let staging_path = staging.keep();
    match fs::rename(&staging_path, &destination) {
        Ok(()) => Ok(destination),
        Err(_) if destination.is_dir() => {
            fs::remove_dir_all(&staging_path)
                .map_err(|error| storage_error(&staging_path, error))?;
            Ok(destination)
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&staging_path);
            Err(storage_error(&destination, error))
        }
    }
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
        name.eq_ignore_ascii_case("__MACOSX")
            || name.eq_ignore_ascii_case(".DS_Store")
            || name.eq_ignore_ascii_case("Thumbs.db")
            || name.eq_ignore_ascii_case("desktop.ini")
            || name.eq_ignore_ascii_case("System Volume Information")
            || name.eq_ignore_ascii_case("$RECYCLE.BIN")
            || name.starts_with("._")
    })
}

fn is_symlink(unix_mode: Option<u32>) -> bool {
    unix_mode.is_some_and(|mode| mode & 0o170000 == 0o120000)
}

fn hash_file(path: &Path) -> Result<String, AppError> {
    let mut file = File::open(path).map_err(|error| storage_error(path, error))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 32 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| storage_error(path, error))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn storage_error(path: &Path, error: std::io::Error) -> AppError {
    AppError::Storage {
        path: path.to_path_buf(),
        reason: error.to_string(),
    }
}
