use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use sha2::{Digest, Sha256};

use crate::errors::AppError;

const USER_FONTS_KEY: &str = "Software\\Microsoft\\Windows NT\\CurrentVersion\\Fonts";

pub trait FontRegistration: Send + Sync + 'static {
    fn register(&self, name: &str, path: &Path) -> Result<(), AppError>;
    fn unregister(&self, name: &str) -> Result<(), AppError>;
}

impl<T: FontRegistration + ?Sized> FontRegistration for Arc<T> {
    fn register(&self, name: &str, path: &Path) -> Result<(), AppError> {
        (**self).register(name, path)
    }

    fn unregister(&self, name: &str) -> Result<(), AppError> {
        (**self).unregister(name)
    }
}

#[derive(Debug, Clone)]
pub struct FontInstaller<R> {
    install_dir: PathBuf,
    registration: R,
}

impl<R: FontRegistration> FontInstaller<R> {
    pub fn new(install_dir: impl Into<PathBuf>, registration: R) -> Self {
        Self {
            install_dir: install_dir.into(),
            registration,
        }
    }

    pub fn install(
        &self,
        font_id: &str,
        full_name: &str,
        source: &Path,
    ) -> Result<PathBuf, AppError> {
        let destination = self.destination(font_id, source)?;
        let value_name = registration_name(font_id, full_name);
        let bytes = std::fs::read(source).map_err(|error| storage_error(source, error))?;
        std::fs::create_dir_all(&self.install_dir)
            .map_err(|error| storage_error(&self.install_dir, error))?;

        let created = if destination.exists() {
            let installed =
                std::fs::read(&destination).map_err(|error| storage_error(&destination, error))?;
            if installed != bytes {
                return Err(AppError::InvalidInput(
                    "Il file installato è diverso dalla copia attuale; rimuovilo prima di reinstallare.".into(),
                ));
            }
            false
        } else {
            let staging = destination.with_extension(format!(
                "{}.tmp",
                destination
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .unwrap_or("font")
            ));
            std::fs::write(&staging, bytes).map_err(|error| storage_error(&staging, error))?;
            if let Err(error) = std::fs::rename(&staging, &destination) {
                let _ = std::fs::remove_file(&staging);
                return Err(storage_error(&destination, error));
            }
            true
        };

        if let Err(error) = self.registration.register(&value_name, &destination) {
            if created {
                let _ = std::fs::remove_file(&destination);
            }
            return Err(error);
        }
        Ok(destination)
    }

    pub fn uninstall(&self, font_id: &str, full_name: &str, source: &Path) -> Result<(), AppError> {
        let destination = self.destination(font_id, source)?;
        let value_name = registration_name(font_id, full_name);
        self.registration.unregister(&value_name)?;
        if destination.exists() {
            if let Err(error) = std::fs::remove_file(&destination) {
                if let Err(rollback_error) = self.registration.register(&value_name, &destination) {
                    tracing::error!(%rollback_error, path = %destination.display(), "Could not restore font registration after file removal failed");
                }
                return Err(storage_error(&destination, error));
            }
        }
        Ok(())
    }

    fn destination(&self, font_id: &str, source: &Path) -> Result<PathBuf, AppError> {
        let extension = source
            .extension()
            .and_then(|extension| extension.to_str())
            .filter(|extension| {
                ["ttf", "otf", "ttc"]
                    .iter()
                    .any(|supported| extension.eq_ignore_ascii_case(supported))
            })
            .ok_or_else(|| AppError::InvalidInput("Formato font non supportato.".into()))?;
        let mut hash = Sha256::new();
        hash.update(font_id.as_bytes());
        let id_hash = format!("{:x}", hash.finalize());
        Ok(self.install_dir.join(format!(
            "OFM_{}.{}",
            &id_hash[..16],
            extension.to_ascii_lowercase()
        )))
    }
}

fn registration_name(font_id: &str, full_name: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(font_id.as_bytes());
    let id_hash = format!("{:x}", hash.finalize());
    format!("{full_name} (Open Font Manager {})", &id_hash[..8])
}

fn storage_error(path: &Path, error: std::io::Error) -> AppError {
    AppError::Storage {
        path: path.to_path_buf(),
        reason: error.to_string(),
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WindowsFontRegistration;

#[cfg(windows)]
impl FontRegistration for WindowsFontRegistration {
    fn register(&self, name: &str, path: &Path) -> Result<(), AppError> {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};

        let current_user = RegKey::predef(HKEY_CURRENT_USER);
        let (fonts, _) = current_user
            .create_subkey(USER_FONTS_KEY)
            .map_err(|error| storage_error(Path::new(USER_FONTS_KEY), error))?;
        fonts
            .set_value(name, &path.to_string_lossy().to_string())
            .map_err(|error| storage_error(Path::new(USER_FONTS_KEY), error))?;
        if let Err(error) = crate::win_font_loader::broadcast_font_change() {
            tracing::warn!(%error, "Installed font but WM_FONTCHANGE broadcast failed");
        }
        Ok(())
    }

    fn unregister(&self, name: &str) -> Result<(), AppError> {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};

        let current_user = RegKey::predef(HKEY_CURRENT_USER);
        let fonts = current_user
            .open_subkey_with_flags(USER_FONTS_KEY, winreg::enums::KEY_SET_VALUE)
            .or_else(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    Ok(current_user.create_subkey(USER_FONTS_KEY)?.0)
                } else {
                    Err(error)
                }
            })
            .map_err(|error| storage_error(Path::new(USER_FONTS_KEY), error))?;
        match fonts.delete_value(name) {
            Ok(()) => {
                if let Err(error) = crate::win_font_loader::broadcast_font_change() {
                    tracing::warn!(%error, "Removed font registration but WM_FONTCHANGE broadcast failed");
                }
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(storage_error(Path::new(USER_FONTS_KEY), error)),
        }
    }
}

#[cfg(not(windows))]
impl FontRegistration for WindowsFontRegistration {
    fn register(&self, _name: &str, _path: &Path) -> Result<(), AppError> {
        Err(AppError::UnsupportedPlatform)
    }

    fn unregister(&self, _name: &str) -> Result<(), AppError> {
        Err(AppError::UnsupportedPlatform)
    }
}
