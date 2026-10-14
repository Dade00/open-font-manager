use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub database: PathBuf,
    pub google_vault: PathBuf,
    pub catalog_cache: PathBuf,
    pub catalog_meta: PathBuf,
    pub preview_cache: PathBuf,
    pub logs: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self, AppError> {
        let project =
            ProjectDirs::from("com", "RustFontManager", "RustFontManager").ok_or_else(|| {
                AppError::Storage {
                    path: PathBuf::from("%LOCALAPPDATA%"),
                    reason: "Windows application data directory is unavailable".to_owned(),
                }
            })?;
        Self::from_roots(project.data_local_dir(), project.cache_dir())
    }

    pub fn from_roots(data_dir: &Path, cache_dir: &Path) -> Result<Self, AppError> {
        let paths = Self {
            data_dir: data_dir.to_path_buf(),
            cache_dir: cache_dir.to_path_buf(),
            database: data_dir.join("library.db"),
            google_vault: data_dir.join("vault").join("google"),
            catalog_cache: cache_dir.join("gfonts_catalog.json"),
            catalog_meta: cache_dir.join("gfonts_catalog.meta.json"),
            preview_cache: cache_dir.join("previews"),
            logs: data_dir.join("logs"),
        };

        for directory in [
            &paths.data_dir,
            &paths.cache_dir,
            &paths.google_vault,
            &paths.preview_cache,
            &paths.logs,
        ] {
            std::fs::create_dir_all(directory).map_err(|error| AppError::Storage {
                path: directory.clone(),
                reason: error.to_string(),
            })?;
        }
        Ok(paths)
    }
}
