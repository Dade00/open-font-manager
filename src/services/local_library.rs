use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
};

use crate::{
    db,
    errors::AppError,
    font_scanner::scan_directory_with_items,
    models::{ScanReport, UpsertOutcome},
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionRestoreReport {
    pub restored: usize,
    pub failed: Vec<String>,
}

#[derive(Debug)]
pub struct LocalLibraryService {
    db_path: PathBuf,
    active_refs: Mutex<HashMap<String, (PathBuf, u32)>>,
}

impl LocalLibraryService {
    pub fn new(db_path: impl Into<PathBuf>) -> Self {
        Self {
            db_path: db_path.into(),
            active_refs: Mutex::new(HashMap::new()),
        }
    }

    pub fn activate(&self, id: &str, path: &Path) -> Result<u32, AppError> {
        let added_count = crate::win_font_loader::activate_font(path)?;
        let mut refs = self
            .active_refs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let entry = refs
            .entry(id.to_owned())
            .or_insert_with(|| (path.to_path_buf(), 0));
        entry.1 = entry.1.saturating_add(1);
        if let Err(error) = db::set_font_active(&self.db_path, id, true) {
            entry.1 = entry.1.saturating_sub(1);
            if entry.1 == 0 {
                refs.remove(id);
            }
            if let Err(rollback_error) = crate::win_font_loader::deactivate_font(path) {
                tracing::error!(%rollback_error, font_id = id, "Failed to roll back GDI activation after database error");
            }
            return Err(error);
        }
        Ok(added_count)
    }

    pub fn deactivate(&self, id: &str) -> Result<(), AppError> {
        let mut refs = self
            .active_refs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let (path, count) = refs.get(id).cloned().ok_or_else(|| {
            AppError::NotFound(format!("Font '{id}' is not active in this session."))
        })?;
        crate::win_font_loader::deactivate_font(&path)?;
        if count <= 1 {
            if let Err(error) = db::set_font_active(&self.db_path, id, false) {
                if let Err(rollback_error) = crate::win_font_loader::activate_font(&path) {
                    tracing::error!(%rollback_error, font_id = id, "Failed to restore GDI activation after database error");
                }
                return Err(error);
            }
            refs.remove(id);
        } else if let Some(entry) = refs.get_mut(id) {
            entry.1 = count - 1;
        }
        Ok(())
    }

    pub fn restore_active_fonts(&self) -> Result<SessionRestoreReport, AppError> {
        let fonts = db::list_active_fonts(&self.db_path)?;
        let mut report = SessionRestoreReport::default();
        for font in fonts {
            if font.missing_on_disk || !font.file_path.is_file() {
                db::set_font_active(&self.db_path, &font.id, false)?;
                report.failed.push(font.family);
                continue;
            }
            match self.activate(&font.id, &font.file_path) {
                Ok(_) => report.restored += 1,
                Err(error) => {
                    db::set_font_active(&self.db_path, &font.id, false)?;
                    tracing::warn!(font_id = %font.id, %error, "Could not restore an active font");
                    report.failed.push(font.family);
                }
            }
        }
        Ok(report)
    }

    pub fn scan_directory(&self, root: &Path) -> Result<ScanReport, AppError> {
        let (mut report, fonts, present_paths) = scan_directory_with_items(root)?;
        db::migrate(&self.db_path)?;
        db::add_scan_root(&self.db_path, &report.root)?;
        for (_, font) in fonts {
            match db::upsert_font(&self.db_path, &font)? {
                UpsertOutcome::Inserted => report.imported += 1,
                UpsertOutcome::Updated => report.updated += 1,
                UpsertOutcome::Unchanged => report.skipped_unchanged += 1,
            }
        }
        db::mark_root_missing_files(&self.db_path, &report.root, &present_paths)?;
        db::finish_scan_root(&self.db_path, &report.root)?;
        Ok(report)
    }
}
