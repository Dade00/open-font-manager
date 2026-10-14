# RustFontManager Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Deliver a native Windows 11 Rust font manager with local scan/catalog, SQLite persistence, temporary GDI activation, Iced previews, Google Fonts downloads/cache, collections, snippets, and verified release build.

**Architecture:** Iced 0.13 owns UI state and dispatches async `Task`s. Pure/domain modules parse fonts, access SQLite, generate snippets, and validate Google data; service modules coordinate filesystem, HTTP, Iced font loading and Windows GDI. All persistent files live under `%LOCALAPPDATA%`; cache files live under the Iced-independent cache path returned by `directories`.

**Tech Stack:** Rust 2021, iced 0.13.1/WGPU, rusqlite 0.32/bundled, reqwest 0.12/Rustls, skrifa/Fontations (not ttf-parser), walkdir, tokio, rfd, arboard, sha2, windows 0.58.

**Spec:** `docs/superpowers/specs/2026-10-14-rust-font-manager-design.md`

## Global Constraints

- Target `x86_64-pc-windows-msvc`, Windows 11; edition 2021.
- No WebView, Tauri, Electron, JavaScript, Node.js, `package.json`, tray icon in v1.
- Never add unmaintained `ttf-parser`; use maintained Fontations parser and verify selected skrifa version/API.
- Never execute filesystem, network, SQLite or potentially blocking Win32 broadcast work on the Iced UI thread.
- No production `unwrap()`, `expect()`, `todo!()`, `unimplemented!()`, deliberate panic or SQL string interpolation.
- Keep active DB state false until GDI activation succeeds; download files atomically and validate before persistence.
- Maximum page size 50 in UI; font parsers and remote data are untrusted.
- Preserve original `IDEA.md`; the detailed spec is authoritative.

## Review Focus

- Malformed or malicious font bytes and missing/invalid OpenType names must not crash scanning; scanner tests own this.
- Windows paths, filesystem failures, and GDI broadcast timeouts must not freeze Iced or produce false active state; services/Win32 tests own this.
- Interrupted/oversized/untrusted HTTP payloads must not leave a final partial TTF or active DB row; Google service tests own this.
- Corrupt DB/cache and stale/missing font files must degrade to actionable errors and preserve recoverable data; DB/bootstrap tests own this.
- User-provided family names/variant IDs must not escape vault paths or inject CSS/TypeScript; path validation and snippet tests own this.

---

## File Map

- `Cargo.toml`, `Cargo.lock`: versions, features, MSVC target dependencies.
- `src/main.rs`: logging and application startup only.
- `src/app.rs`: Iced `AppState`, `Message`, `update`, `view`, task mapping and page state.
- `src/theme.rs`: palette/style functions.
- `src/models.rs`: domain/DTO structs and enums.
- `src/errors.rs`: typed domain error and user-facing error mapping.
- `src/paths.rs`: application data/cache paths and directory creation.
- `src/db.rs`: migrations, parameterized queries, transaction helpers.
- `src/font_names.rs`: unique static family-name interning and Iced font descriptor.
- `src/font_scanner.rs`: supported-file detection and metadata parsing with skrifa.
- `src/win_font_loader.rs`: Windows GDI API wrapper and non-Windows unsupported stubs.
- `src/google_fonts.rs`: HTTP DTO/cache/download/atomic vault writer.
- `src/snippets.rs`: pure CSS/Next.js snippet generation and escaping.
- `src/services/local_library.rs`: scan/import/activation/startup orchestration.
- `src/services/google_library.rs`: catalog/detail/preview/install orchestration.
- `tests/db_tests.rs`, `scanner_tests.rs`, `google_fonts_tests.rs`, `snippets_tests.rs`: integration coverage and fixtures.
- `README.md`: build/run/use/limits.

## Task 1: Cargo scaffold and core types

**Files:** Create manifest, module files, `src/models.rs`, `src/errors.rs`, `src/paths.rs`, `src/main.rs`; create tests directory. Preserve `IDEA.md` and spec.

**Interfaces:**
- `AppError` variants match spec §15.1 and implement `Display`/`Error`; no source payload is shown raw to users.
- `AppPaths::discover() -> Result<AppPaths, AppError>` resolves `ProjectDirs::from("com", "RustFontManager", "RustFontManager")`, data-local/cache paths and creates required dirs.
- `FontItem`, `FontSource`, `FontCategory`, `LibraryTab`, `FontActivation`, `ScanIssue`, `ScanReport`, `Collection`, `ScanRoot`, `FontQuery`, `FontPage`, `UpsertOutcome`, `PreviewState`, `CatalogState`, `OperationState`, `Toast`, `SnippetKind` reflect spec §6/§13.
- Generate a lockfile with selected dependencies; check current skrifa maintenance/version/API before committing to exact parser dependency. Update spec only if necessary; do not silently choose a different parser.

- [ ] Create Cargo manifest with specified pinned/minimum versions and features, excluding tray-icon and ttf-parser.
- [ ] Add tests for `AppPaths` through an injectable base-path constructor (do not write to the developer's real AppData during tests): assert exact relative filenames and required directories.
- [ ] Add enum/model serialization round-trip tests for persisted source/category values.
- [ ] Run `cargo check --target x86_64-pc-windows-msvc` and `cargo test`.
- [ ] Verify `rg --files -g 'package.json'` returns no paths and manifest contains neither `ttf-parser` nor `tray-icon`.

## Task 2: SQLite schema and repository

**Files:** `src/db.rs`, `tests/db_tests.rs`.

**Interfaces:** Provide every function listed in spec §7.3. Use `db_path: &Path` per operation or a repository wrapper that owns only the path; do not store `Connection` in Iced state or send it across tasks. `FontQuery` accepts page size 1..=100; UI uses 50.

- [ ] Test-first: create temp DB; assert schema/migrations create expected tables, enable foreign keys and are idempotent.
- [ ] Test-first: test parameterized search/filter/tab/page queries, case-insensitive family matching, 50-result pagination and invalid page-size rejection.
- [ ] Test-first: test font upsert uniqueness, SHA ID caller contract, and preservation of favorite/active flags when metadata refreshes.
- [ ] Test-first: test collection names case-insensitive unique; adding duplicate item is idempotent; deleting collection cascades items; deleting font cascades items.
- [ ] Implement transaction-safe migration and CRUD with prepared/parameterized rusqlite calls; mark missing files only for matching scan root and preserve Google records.
- [ ] Run `cargo test --test db_tests` then full `cargo test`.

## Task 3: Font scanner and stable identity

**Files:** `src/font_scanner.rs`, `src/models.rs`, `src/services/local_library.rs`, test fixtures and `tests/scanner_tests.rs`.

**Interfaces:**
```rust
pub fn scan_directory(root: &Path) -> Result<ScanReport, AppError>;
pub fn parse_font_file(path: &Path) -> Result<ParsedFontMetadata, AppError>;
pub fn is_supported_font_path(path: &Path) -> bool;
pub fn font_id(canonical_path: &Path, variant_id: Option<&str>) -> Result<String, AppError>;
```

`ParsedFontMetadata` contains internal family/subfamily/full name, numeric weight, italic, file size and modified time. `font_id` uses SHA-256 of canonical UTF-8 path + newline + optional variant id. Handle non-UTF8 canonical paths deterministically using OS bytes/UTF-16LE documented serialization; do not lossy-normalize two distinct paths to one ID.

- [ ] Verify skrifa crate is maintained and available; inspect its current API and compile a minimal parser probe for name IDs 1/2/4/16/17, weight, italic and malformed input.
- [ ] Add tests for case-insensitive `.ttf/.otf`, ignored `.woff2`, invalid bytes, missing names/fallbacks and valid fixture metadata.
- [ ] Implement safe parser using supported skrifa/Fontations API. If fixture license is unclear, generate a small fixture from installed system font only if redistribution is permitted; otherwise document how fixture is generated and use test-only generated data.
- [ ] Walk recursively with `follow_links(false)`, one file at a time; continue after per-file failures and report all counts/issues.
- [ ] Run `cargo test --test scanner_tests`; ensure malformed fixture does not panic.

## Task 4: Session Win32 font loader

**Files:** `src/win_font_loader.rs`, `src/services/local_library.rs`, `tests` platform-gated tests.

**Interfaces:** `activate_font(&Path) -> Result<u32, AppError>`, `deactivate_font(&Path) -> Result<(), AppError>`, `broadcast_font_change() -> Result<(), AppError>`. Non-Windows implementations return `UnsupportedPlatform`.

- [ ] Compile-test exact windows 0.58 API calls on `x86_64-pc-windows-msvc`; preserve UTF-16 nul termination and check zero/BOOL return values.
- [ ] Add pure unit tests for path validation/UTF-16 conversion where possible; isolate FFI from those tests.
- [ ] Implement `SendMessageTimeoutW(HWND_BROADCAST, WM_FONTCHANGE, ..., SMTO_ABORTIFHUNG, 1000, ...)` and ensure service invokes it in a blocking worker, not UI thread.
- [ ] Activation success remains success if broadcast fails; log warning. Remove failure remains an error. Track one remove for each successful activation made by this process.
- [ ] Test service state transition: DB `is_active` changes only after GDI success; failure preserves inactive.
- [ ] Run `cargo check --target x86_64-pc-windows-msvc` and on Windows manually activate/deactivate a test font; never alter system Fonts directory/registry.

## Task 5: Snippet generation and font family interning

**Files:** `src/snippets.rs`, `src/font_names.rs`, `tests/snippets_tests.rs`.

**Interfaces:**
```rust
pub fn intern_font_family(name: &str) -> &'static str;
pub fn font_for_item(item: &FontItem) -> iced::Font;
pub fn css_snippet(item: &FontItem) -> String;
pub fn next_font_snippet(item: &FontItem) -> Result<String, AppError>;
```

- [ ] Test family name interning is idempotent by pointer/content and distinct names do not alias.
- [ ] Test CSS escapes quote/backslash/control chars; weight and normal/italic match item.
- [ ] Test Google Next snippet includes import family, subsets and supported weights/styles; local snippet uses `next/font/local` and provides a copy-file warning.
- [ ] Implement `OnceLock<Mutex<HashSet<&'static str>>>`; `Box::leak` only once per unique family string.
- [ ] Map numeric weight to nearest Iced `Weight` enum; set italic style. Avoid panics if mutex is poisoned: recover inner value or return a documented fallback.
- [ ] Run `cargo test --test snippets_tests` and fmt.

## Task 6: Google Fonts client and atomic cache/vault

**Files:** `src/google_fonts.rs`, `src/services/google_library.rs`, `tests/google_fonts_tests.rs`.

**Interfaces:**
```rust
pub async fn fetch_catalog(client: &reqwest::Client, paths: &AppPaths) -> Result<CatalogResult, AppError>;
pub async fn fetch_detail(client: &reqwest::Client, family_id: &str) -> Result<GoogleFontDetail, AppError>;
pub async fn download_ttf(client: &reqwest::Client, url: &Url, limit: u64) -> Result<Vec<u8>, AppError>;
pub fn validate_ttf(bytes: &[u8]) -> Result<ParsedFontMetadata, AppError>;
pub async fn write_font_atomically(path: &Path, bytes: &[u8]) -> Result<(), AppError>;
```

- [ ] Deserialize fixtures with unknown fields and missing optional `ttf`; handle aliases/casing to match exact API JSON in spec §11.
- [ ] Test catalog cache hit, stale cache fallback on transport error, HTTP error, oversized response, unknown JSON fields and invalid JSON.
- [ ] Test detail selection: default variant, regular fallback, first TTF fallback, no available TTF.
- [ ] Validate `family-id` and `variant-id` against a restrictive safe-character allowlist before path construction; never concatenate unsanitized remote IDs into paths.
- [ ] Implement 20s timeout, HTTPS-only redirect policy, 10 MiB catalog cap, 25 MiB font cap, ETag/304, schema version 1, 24h catalog freshness and atomic writes.
- [ ] Validate font before final rename; partial/error download never creates final path.
- [ ] Add HTTP mock tests using an in-process local server; do not depend on public network in automated tests.
- [ ] Run `cargo test --test google_fonts_tests`.

## Task 7: Local library service and startup restore

**Files:** `src/services/local_library.rs`, `src/db.rs`, tests.

**Interfaces:** service operations for scan/import, toggle activation, toggle favorite and startup active-font restore. Service functions are async wrappers that move blocking scanner/SQLite/GDI work to `tokio::task::spawn_blocking`; errors are converted to `AppError`.

- [ ] Test scanning root updates DB, preserves active/favorite flags and marks absent records missing.
- [ ] Test activation sequence as fake/injected loader: persist active only after successful GDI, deactivate only after successful remove.
- [ ] Test startup restore ignores missing paths, returns per-font failures, and limits concurrency to four.
- [ ] Implement scan-root add/rescan and report import/update/unchanged/unsupported/failures.
- [ ] Implement active-font startup read → bytes validation → iced loading result → GDI restore; if file/GDI fails set DB inactive and emit warning. Bound in-flight restores to four.
- [ ] Run service tests; Windows manual check of a loaded font remains required.

## Task 8: Google library service orchestration

**Files:** `src/services/google_library.rs`, `src/google_fonts.rs`, tests.

- [ ] Preview flow: detail/cache → choose variant → capped download → parse/validate → atomic preview cache → return bytes; must not touch DB or GDI.
- [ ] Install flow tests with injected HTTP/parser/Iced/GDI/storage adapters: validated vault file → DB inactive upsert → Iced load success → GDI success → DB active. Assert each failure leaves valid files recoverable and DB inactive.
- [ ] Family download tests retain successful variants and report failures individually; family download does not auto-activate all variants.
- [ ] Implement service API and deduplicate simultaneous downloads for the same family/variant to avoid conflicting temp/final writes.
- [ ] Run Google service tests.

## Task 9: Iced application shell and local UI

**Files:** `src/main.rs`, `src/app.rs`, `src/theme.rs`.

- [ ] Add testable pure state/update helpers for filtering and pagination; assert maximum 50 rendered items.
- [ ] Build `iced::application("RustFontManager", update, view)` with explicit dark palette from spec §12.1 and WGPU renderer.
- [ ] Add sidebar navigation, active count, collection list/create input, folder picker via `rfd::AsyncFileDialog` and messages.
- [ ] Add top search (debounce <=150ms), preview text input, 14..=96 slider, categories and page navigation.
- [ ] Add cards for local/downloaded fonts: preview, origin/category, weight/italic, activation toggle, favorite, CSS/Next copy, collection menu, missing-file state.
- [ ] Use Iced `Task` mapping for async results. For blocking operations use `spawn_blocking`; for HTTP use tokio async. Do not call `font::load` and immediately assume success; map the returned task result to `IcedFontLoaded`.
- [ ] Add per-operation state and toast feedback; no modal for routine progress/errors.
- [ ] `cargo check` and tests; manually inspect rendering/keyboard/focus/DPI.

## Task 10: Google UI and bootstrap integration

**Files:** `src/app.rs`, `src/services/google_library.rs`, `src/main.rs`.

- [ ] Startup command runs DB/path migration, local load, active font restore and catalog cache/network concurrently without blocking first UI render.
- [ ] Implement Google tab, offline/stale status, retry, search/filter, variant selector, on-demand preview, install one variant, family download and summary.
- [ ] Ensure preview states `NotRequested → Downloading → LoadingIntoIced → Ready/Failed`; do not store font byte vectors in long-lived `AppState`.
- [ ] Test update reducer state transitions and errors; run full automated suite.

## Task 11: Clipboard and collection interaction

**Files:** `src/app.rs`, `src/snippets.rs`, DB/service tests.

- [ ] Clipboard work runs in a worker; success/error generates toast. Test failure message through injected clipboard adapter.
- [ ] Create/rename/delete collection and add/remove font actions use repository operations; reject blank trimmed names and show uniqueness errors.
- [ ] Collection selection filters by joined collection IDs; empty collection has empty state.
- [ ] Run all DB/snippet/UI state tests.

## Task 12: Documentation and release hardening

**Files:** `README.md`, all project files.

- [ ] README covers build prerequisites, `cargo run`, release build, paths, local scan, activation caveats, Google/offline behavior, unsupported formats and privacy/network endpoints.
- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --all-features`.
- [ ] Run `cargo build --release --target x86_64-pc-windows-msvc`.
- [ ] Search production source for `unwrap(`, `expect(`, `todo!`, `unimplemented!`; inspect each match, allowing no unsafe panic shortcuts.
- [ ] Verify no package.json and no tray-icon/ttf-parser dependency; inspect `cargo tree` and lockfile.
- [ ] Execute manual Windows QA checklist in spec §16.4, record Windows build, GPU, test font and application results in README/release notes.
- [ ] Report remaining unverified items; do not claim distribution-ready without manual QA evidence.

---

## Execution notes

- Work sequentially because later task interfaces depend on earlier contracts. A second implementer can review discrete completed tasks, but should not invent alternate types or skip tests.
- The current directory is not a Git repository. Do not run `git commit`; do not initialize Git without an explicit request. Keep all changes under the project directory except temporary probes in the Hermes scratch directory.
- Run verification after each task and again at final acceptance. A passing `cargo check` does not substitute for tests, release build or Windows manual behavior.
