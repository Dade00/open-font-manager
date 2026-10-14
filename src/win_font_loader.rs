use std::path::Path;

use crate::errors::AppError;

#[cfg(windows)]
mod platform {
    use std::{os::windows::ffi::OsStrExt, path::Path};

    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::{LPARAM, WPARAM},
            Graphics::Gdi::{AddFontResourceW, RemoveFontResourceW},
            UI::WindowsAndMessaging::{
                SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_FONTCHANGE,
            },
        },
    };

    use crate::errors::AppError;

    fn validate_font_path(path: &Path) -> Result<Vec<u16>, AppError> {
        let metadata = std::fs::metadata(path).map_err(|error| AppError::Storage {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
        if !metadata.is_file() {
            return Err(AppError::InvalidInput(format!(
                "Font path is not a regular file: {}",
                path.display()
            )));
        }
        Ok(path.as_os_str().encode_wide().chain(Some(0)).collect())
    }

    pub(super) fn activate_font(path: &Path) -> Result<u32, AppError> {
        let wide_path = validate_font_path(path)?;
        // SAFETY: wide_path is a valid, nul-terminated UTF-16 path and remains alive for the call.
        let count = unsafe { AddFontResourceW(PCWSTR(wide_path.as_ptr())) };
        if count == 0 {
            return Err(AppError::Win32FontLoad {
                path: path.to_path_buf(),
            });
        }
        if let Err(error) = broadcast_font_change() {
            tracing::warn!(path = %path.display(), %error, "Font activated but WM_FONTCHANGE broadcast failed");
        }
        Ok(count as u32)
    }

    pub(super) fn deactivate_font(path: &Path) -> Result<(), AppError> {
        let wide_path = validate_font_path(path)?;
        // SAFETY: wide_path is a valid, nul-terminated UTF-16 path and remains alive for the call.
        let removed = unsafe { RemoveFontResourceW(PCWSTR(wide_path.as_ptr())) };
        if removed.as_bool() {
            if let Err(error) = broadcast_font_change() {
                tracing::warn!(path = %path.display(), %error, "Font removed but WM_FONTCHANGE broadcast failed");
            }
            Ok(())
        } else {
            Err(AppError::Win32FontRemove {
                path: path.to_path_buf(),
            })
        }
    }

    pub(super) fn broadcast_font_change() -> Result<(), AppError> {
        let mut result = 0_usize;
        // SAFETY: system broadcast uses a null-free message and a valid result pointer.
        let sent = unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_FONTCHANGE,
                WPARAM(0),
                LPARAM(0),
                SMTO_ABORTIFHUNG,
                1000,
                Some(&mut result as *mut usize),
            )
        };
        if sent.0 == 0 {
            Err(AppError::Win32Broadcast(
                "SendMessageTimeoutW did not deliver WM_FONTCHANGE before timeout".into(),
            ))
        } else {
            Ok(())
        }
    }

    #[cfg(test)]
    pub(super) fn utf16_path(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }
}

#[cfg(not(windows))]
mod platform {
    use std::path::Path;

    use crate::errors::AppError;

    pub(super) fn activate_font(_path: &Path) -> Result<u32, AppError> {
        Err(AppError::UnsupportedPlatform)
    }

    pub(super) fn deactivate_font(_path: &Path) -> Result<(), AppError> {
        Err(AppError::UnsupportedPlatform)
    }

    pub(super) fn broadcast_font_change() -> Result<(), AppError> {
        Err(AppError::UnsupportedPlatform)
    }
}

pub fn activate_font(path: &Path) -> Result<u32, AppError> {
    platform::activate_font(path)
}

pub fn deactivate_font(path: &Path) -> Result<(), AppError> {
    platform::deactivate_font(path)
}

pub fn broadcast_font_change() -> Result<(), AppError> {
    platform::broadcast_font_change()
}

#[cfg(test)]
#[cfg(windows)]
mod tests {
    use std::path::Path;

    use super::platform::utf16_path;

    #[test]
    fn utf16_path_is_nul_terminated_without_changing_unicode() {
        let path = Path::new("C:\\fonts\\épreuve.ttf");
        let encoded = utf16_path(path);
        assert_eq!(encoded.last(), Some(&0));
        let decoded = String::from_utf16(&encoded[..encoded.len() - 1]).unwrap();
        assert_eq!(decoded, path.to_string_lossy());
    }
}
