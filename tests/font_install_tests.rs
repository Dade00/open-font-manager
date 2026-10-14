use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use rust_font_manager::{
    errors::AppError,
    services::font_install::{FontInstaller, FontRegistration},
};
use tempfile::tempdir;

#[derive(Clone, Default)]
struct FakeRegistration(Arc<Mutex<HashMap<String, PathBuf>>>);

impl FontRegistration for FakeRegistration {
    fn register(&self, name: &str, path: &Path) -> Result<(), AppError> {
        self.0
            .lock()
            .unwrap()
            .insert(name.to_owned(), path.to_path_buf());
        Ok(())
    }

    fn unregister(&self, name: &str) -> Result<(), AppError> {
        self.0.lock().unwrap().remove(name);
        Ok(())
    }
}

#[test]
fn installs_font_into_managed_directory_and_registers_it_for_current_user() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("source.ttf");
    let install_dir = temp.path().join("windows-fonts");
    std::fs::write(&source, b"font fixture").unwrap();
    let registration = FakeRegistration::default();
    let installer = FontInstaller::new(&install_dir, registration.clone());

    let installed_path = installer
        .install("font-id", "Fixture Sans Regular", &source)
        .unwrap();

    assert!(installed_path.starts_with(&install_dir));
    assert_eq!(std::fs::read(&installed_path).unwrap(), b"font fixture");
    assert_eq!(registration.0.lock().unwrap().len(), 1);
}

#[test]
fn uninstall_removes_only_the_managed_file_and_its_registration() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("source.ttf");
    let install_dir = temp.path().join("windows-fonts");
    std::fs::write(&source, b"font fixture").unwrap();
    let registration = FakeRegistration::default();
    let installer = FontInstaller::new(&install_dir, registration.clone());
    let installed_path = installer
        .install("font-id", "Fixture Sans Regular", &source)
        .unwrap();

    installer
        .uninstall("font-id", "Fixture Sans Regular", &source)
        .unwrap();

    assert!(!installed_path.exists());
    assert!(registration.0.lock().unwrap().is_empty());
}
