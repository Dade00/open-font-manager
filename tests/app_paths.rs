use std::path::PathBuf;

use rust_font_manager::paths::AppPaths;

#[test]
fn app_paths_use_expected_files_under_injected_roots() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let data = temp.path().join("data-local");
    let cache = temp.path().join("cache");

    let paths = AppPaths::from_roots(&data, &cache).expect("paths should be created");

    assert_eq!(paths.data_dir, data);
    assert_eq!(paths.cache_dir, cache);
    assert_eq!(paths.database, data.join("library.db"));
    assert_eq!(paths.google_vault, data.join("vault/google"));
    assert_eq!(paths.catalog_cache, cache.join("gfonts_catalog.json"));
    assert_eq!(paths.catalog_meta, cache.join("gfonts_catalog.meta.json"));
    assert_eq!(paths.preview_cache, cache.join("previews"));
    assert_eq!(paths.logs, data.join("logs"));
    for directory in [
        &paths.data_dir,
        &paths.cache_dir,
        &paths.google_vault,
        &paths.preview_cache,
        &paths.logs,
    ] {
        assert!(directory.is_dir(), "{} should exist", directory.display());
    }
}

#[test]
fn app_paths_discover_uses_windows_local_data_and_cache() {
    let paths = AppPaths::discover().expect("project directories should resolve on this machine");
    assert!(paths.database.ends_with(PathBuf::from("library.db")));
    assert!(paths
        .catalog_cache
        .ends_with(PathBuf::from("gfonts_catalog.json")));
}
