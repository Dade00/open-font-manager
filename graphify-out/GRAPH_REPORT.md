# Graph Report - fontmanager  (2026-10-18)

## Corpus Check
- 97 files · ~87,894 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 5 file(s) not represented in the graph (top: (none) 2, .css 2, .ico 1)

## Summary
- 1266 nodes · 2358 edges · 91 communities (74 shown, 17 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 51 edges (avg confidence: 0.88)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- cloud_fonts.rs
- db_tests.rs
- InspirationView.tsx
- types.ts
- google_library.rs
- font_scanner.rs
- GlobalSearchView.tsx
- inspiration.rs
- commands.rs
- App.tsx
- cloud_import.rs
- db.rs
- next_font_snippet()
- Piano di implementazione: provider cloud e Impostazioni
- AppBackend
- RustFontManager Implementation Plan
- AppError
- compilerOptions
- font_install.rs
- CloudCatalogView.tsx
- .list_fonts()
- tauri.conf.json
- compilerOptions
- package.json
- FontQuery
- RemoteFontProvider
- cloud_import_tests.rs
- devDependencies
- RustFontManager — Specifica tecnica di progetto
- Open Font Manager
- FontRegistration
- Ricerca Globale e navigazione unificata
- win_font_loader.rs
- check_catalog_layout.py
- Ispirazione: abbinamenti e moodboard
- SettingsView.tsx
- PreviewQueue
- README.md
- Coda evolutiva Open Font Manager
- Ricerca aggiuntiva: cataloghi font e API configurabili
- models.rs
- backend.rs
- inspiration_commands.rs
- 12. UI Iced
- 18. Sequenza di sviluppo obbligatoria
- FontCategory
- RemoteFontProvider
- load_cached_remote_catalog()
- Ricerca Globale e Ispirazione Implementation Plan
- 10. Wrapper Win32 per font temporanei
- IDEA.md
- 11. Client Google Fonts
- 9. Font Iced e interning dei nomi
- dependencies
- FontSource
- default.json
- Contribuire a Open Font Manager
- 16. Test obbligatori
- 1. Obiettivo e confini
- 6. Modelli e contratti di dominio
- 7. Database SQLite
- 8. Scanner OpenType
- bug_report.md
- feature_request.md
- scripts
- LibraryTab
- 13. Stato applicazione e messaggi
- 5. Directory e file locali
- PULL_REQUEST_TEMPLATE.md
- vitest
- UpsertOutcome
- list_fonts()
- repository
- FontActivation
- FontDownloadStrategy
- SnippetKind
- tsconfig.json
- open-font-manager-app
- PONYTAIL-DEBT.md
- @testing-library/jest-dom

## God Nodes (most connected - your core abstractions)
1. `AppError` - 154 edges
2. `AppBackend` - 74 edges
3. `fontManagerApi` - 36 edges
4. `App()` - 36 edges
5. `InspirationView()` - 29 edges
6. `open_connection()` - 25 edges
7. `blocking()` - 22 edges
8. `RustFontManager — Specifica tecnica di progetto` - 20 edges
9. `temp_db()` - 20 edges
10. `RemoteFontProvider` - 18 edges

## Surprising Connections (you probably didn't know these)
- `11.4 Download + attivazione di una variante` --references--> `FontItem`  [INFERRED]
  docs/superpowers/specs/2026-10-14-rust-font-manager-design.md → ui/src/types.ts
- `Pull request` --references--> `main()`  [INFERRED]
  CONTRIBUTING.md → scripts/check_catalog_layout.py
- `Task 1 - Contratto remoto e persistenza provider` --references--> `RemoteFontCatalogItem`  [INFERRED]
  docs/superpowers/plans/2026-10-14-cloud-font-providers.md → ui/src/types.ts
- `Task 1 - Contratto remoto e persistenza provider` --references--> `RemoteFontProvider`  [INFERRED]
  docs/superpowers/plans/2026-10-14-cloud-font-providers.md → ui/src/types.ts
- `Task 1: Cargo scaffold and core types` --references--> `ScanRoot`  [INFERRED]
  docs/superpowers/plans/2026-10-14-rust-font-manager-implementation.md → ui/src/types.ts

## Import Cycles
- None detected.

## Communities (91 total, 17 thin omitted)

### Community 0 - "cloud_fonts.rs"
Cohesion: 0.06
Nodes (51): ARCHIVE_RESPONSE_LIMIT, atomic_write(), CACHE_RESPONSE_LIMIT, CACHE_SCHEMA_VERSION, CachedRemoteCatalog, CATALOG_MAX_AGE_MS, category_from_string(), CloudCatalogCache (+43 more)

### Community 1 - "db_tests.rs"
Cohesion: 0.05
Nodes (35): activation_and_favorite_updates_require_existing_font(), active_font_restore_query_pages_past_one_hundred_and_counts_existing_files_only(), cloud_provider_source_persists_through_database_migration(), collection_names_are_case_insensitive_and_duplicate_membership_is_idempotent(), database_stores_multiple_faces_from_the_same_collection_file(), deleting_font_cascades_collection_membership(), font(), font_api_lookup_returns_serializable_camel_case_font_and_rejects_missing_id() (+27 more)

### Community 2 - "InspirationView.tsx"
Cohesion: 0.06
Nodes (40): @tauri-apps/api, inspirationApi, InspirationConfirm(), Board, BoardItem, Composition, CompositionRole, InspirationApi (+32 more)

### Community 3 - "types.ts"
Cohesion: 0.05
Nodes (28): 3.4 Backend e contratti Tauri, @testing-library/react, @testing-library/user-event, fontManagerApi, dragDropMock, fixture, googleDisplayCatalog, initial (+20 more)

### Community 4 - "google_library.rs"
Cohesion: 0.06
Nodes (27): CATALOG_LIMIT, CATALOG_URL, CatalogResponse, NotModified, Updated, DETAIL_LIMIT, endpoint_url(), FONT_LIMIT (+19 more)

### Community 5 - "font_scanner.rs"
Cohesion: 0.06
Nodes (24): font_id(), hash_non_utf8_path(), is_open_font_manager_install_copy(), is_supported_font_path(), parse_font_bytes(), parse_font_bytes_at_index(), parse_font_file(), preferred_name() (+16 more)

### Community 6 - "GlobalSearchView.tsx"
Cohesion: 0.11
Nodes (29): filterGlobalFonts(), GlobalFontFilters, GlobalFontGroup, GlobalFontResult, GlobalProviderSnapshot, groupGlobalFonts(), identityKey(), isInstalled() (+21 more)

### Community 7 - "inspiration.rs"
Cohesion: 0.12
Nodes (32): asset_path(), Board, BoardItem, Composition, Font, Image, Link, Note (+24 more)

### Community 8 - "commands.rs"
Cohesion: 0.12
Nodes (22): add_font_to_collection(), add_scan_root(), blocking(), create_collection(), download_google_family(), get_app_settings(), get_font_preview_bytes(), get_font_snippet() (+14 more)

### Community 9 - "App.tsx"
Cohesion: 0.10
Nodes (31): react-dom, App(), handleActivation(), handleAddToCollection(), handleCopy(), handleCreateCollection(), handleFavorite(), handleGooglePreview() (+23 more)

### Community 10 - "cloud_import.rs"
Cohesion: 0.14
Nodes (18): atomic_write(), CloudFontImporter, CloudFontImportOutcome, digest_hex(), is_excluded_nerd_font_path(), is_metadata_path(), is_supported_font(), is_symlink() (+10 more)

### Community 11 - "db.rs"
Cohesion: 0.11
Nodes (23): add_font_to_collection(), add_scan_root(), count_active_fonts(), create_collection(), delete_collection(), delete_font(), finish_scan_root(), finish_scan_root_report() (+15 more)

### Community 12 - "next_font_snippet()"
Cohesion: 0.12
Nodes (12): css_snippet(), google_import_identifier(), google_variable_name(), json_string(), lower_camel_case(), next_font_snippet(), pascal_case_identifier(), cloud_downloaded_font_uses_a_local_next_snippet() (+4 more)

### Community 13 - "Piano di implementazione: provider cloud e Impostazioni"
Cohesion: 0.08
Nodes (23): Decisioni di ambito, Piano di implementazione: provider cloud e Impostazioni, Task 1 - Contratto remoto e persistenza provider, Task 2 - Adapter catalogo e client cloud, Task 3 - Download, import atomico e attivazione, Task 4 - Comandi Tauri e Impostazioni backend, Task 5 - UI cataloghi, preview lazy e Impostazioni, Task 6 - Verifica completa e chiusura (+15 more)

### Community 14 - "AppBackend"
Cohesion: 0.12
Nodes (4): AppPaths, AppBackend, StartupMaintenanceReport, user_font_directory()

### Community 15 - "RustFontManager Implementation Plan"
Cohesion: 0.10
Nodes (20): Execution notes, File Map, Global Constraints, Review Focus, RustFontManager Implementation Plan, Task 10: Google UI and bootstrap integration, Task 11: Clipboard and collection interaction, Task 12: Documentation and release hardening (+12 more)

### Community 16 - "AppError"
Cohesion: 0.11
Nodes (19): set_font_active(), set_font_favorite(), update_flag(), AppError, Clipboard, Database, DownloadTooLarge, FontParse (+11 more)

### Community 17 - "compilerOptions"
Cohesion: 0.11
Nodes (18): compilerOptions, allowImportingTsExtensions, jsx, lib, module, moduleDetection, moduleResolution, noEmit (+10 more)

### Community 18 - "font_install.rs"
Cohesion: 0.18
Nodes (7): Arc<T>, font_id_digest(), FontInstaller, FontInstaller<R>, storage_error(), USER_FONTS_KEY, WindowsFontRegistration

### Community 19 - "CloudCatalogView.tsx"
Cohesion: 0.22
Nodes (14): react, CATEGORIES, categoryLabel(), FontCategory, LABELS, normalizeCategory(), categories, CategoryFilter() (+6 more)

### Community 20 - ".list_fonts()"
Cohesion: 0.15
Nodes (8): font_roots_from(), installed_font_roots(), is_installed_font_path(), normalized_path(), mark_installed(), StartupSnapshot, user_message(), installed_roots_include_system_and_per_user_font_directories()

### Community 21 - "tauri.conf.json"
Cohesion: 0.12
Nodes (16): app, security, windows, build, beforeBuildCommand, beforeDevCommand, devUrl, frontendDist (+8 more)

### Community 22 - "compilerOptions"
Cohesion: 0.12
Nodes (16): compilerOptions, allowImportingTsExtensions, lib, module, moduleDetection, moduleResolution, noEmit, noFallthroughCasesInSwitch (+8 more)

### Community 23 - "package.json"
Cohesion: 0.12
Nodes (15): funding, license, name, private, type, version, daisyui, jsdom (+7 more)

### Community 24 - "FontQuery"
Cohesion: 0.17
Nodes (11): category_str(), font_query_filters(), list_active_fonts(), list_fonts(), list_installed_fonts(), now_ms(), parse_cat(), row_font() (+3 more)

### Community 26 - "cloud_import_tests.rs"
Cohesion: 0.23
Nodes (9): archive(), archive_without_desktop_fonts_reports_a_reason_and_writes_nothing(), fontshare_preview_validates_bytes_without_writing_a_vault(), fontsource_item(), imports_fontsource_files_individually_and_keeps_partial_successes(), imports_validated_zip_fonts_into_family_vault_and_indexes_cloud_source(), nerd_item(), symlinks_and_hidden_font_entries_are_not_materialized() (+1 more)

### Community 27 - "devDependencies"
Cohesion: 0.13
Nodes (15): devDependencies, daisyui, jsdom, tailwindcss, @tailwindcss/vite, @tauri-apps/cli, @testing-library/jest-dom, @testing-library/react (+7 more)

### Community 28 - "RustFontManager — Specifica tecnica di progetto"
Cohesion: 0.15
Nodes (12): 14.1 CSS, 14.2 Next.js, 14. Snippet clipboard, 15.1 Errori dominio, 15.2 Logging, 15. Errori e logging, 17. Criteri di accettazione, 19. Limiti dichiarati all’utente (+4 more)

### Community 29 - "Open Font Manager"
Cohesion: 0.15
Nodes (13): Avvio rapido, Compilazione ed esecuzione, Contribuire e segnalare problemi, Funzioni, Licenza, Limitazioni note, Open Font Manager, Privacy e dati (+5 more)

### Community 30 - "FontRegistration"
Cohesion: 0.17
Nodes (3): FontRegistration, FakeFontRegistration, FakeRegistration

### Community 31 - "Ricerca Globale e navigazione unificata"
Cohesion: 0.17
Nodes (12): Cache, progressione ed errori, Contratto dei provider, Criteri di accettazione, Esperienza di ricerca, Fuori perimetro, Identità e deduplicazione, Installazione e preview, Navigazione primaria (+4 more)

### Community 32 - "win_font_loader.rs"
Cohesion: 0.24
Nodes (6): activate_font(), broadcast_font_change(), deactivate_font(), utf16_path(), utf16_path_is_nul_terminated_without_changing_unicode(), validate_font_path()

### Community 33 - "check_catalog_layout.py"
Cohesion: 0.22
Nodes (5): main(), Segnalare una vulnerabilità, Sicurezza, Superficie rilevante, Versioni supportate

### Community 34 - "Ispirazione: abbinamenti e moodboard"
Cohesion: 0.18
Nodes (11): Abbinamenti, Collegamento fra viste e cataloghi, Criteri di accettazione, Fuori perimetro iniziale, Ispirazione: abbinamenti e moodboard, Moodboard, Obiettivo, Persistenza e architettura (+3 more)

### Community 35 - "SettingsView.tsx"
Cohesion: 0.33
Nodes (10): @tauri-apps/plugin-dialog, errorMessage(), formatDate(), PROVIDER_LABELS, scanSummary(), SettingsView(), addRoot(), removeRoot() (+2 more)

### Community 36 - "PreviewQueue"
Cohesion: 0.25
Nodes (4): PreviewJob, PreviewQueue, PreviewQueueOptions, RowPreviewObserver()

### Community 37 - "README.md"
Cohesion: 0.20
Nodes (5): Codice di condotta, Dipendenze e contenuti di terze parti, Dipendenze software, Font e cataloghi, Moodboard e altri contenuti

### Community 38 - "Coda evolutiva Open Font Manager"
Cohesion: 0.20
Nodes (10): Coda evolutiva Open Font Manager, Coda ordinata, Direzione approvata, Estensioni cataloghi e API configurabili, Ispirazione: Abbinamenti e Moodboard, Passaggio all'esecuzione, Prima della coda: tranche corrente da chiudere, Revisione UX del 2026-10-15 (+2 more)

### Community 39 - "Ricerca aggiuntiva: cataloghi font e API configurabili"
Cohesion: 0.20
Nodes (10): API configurabili dall'utente finale, Attenzione: provider già presente Fontshare, Confronto, Controllo della FAQ italiana fornita dall'utente, DaFont: risposta operativa, Fonti primarie, Limiti della verifica, Ordine consigliato per estendere le fonti (+2 more)

### Community 40 - "models.rs"
Cohesion: 0.27
Nodes (9): Collection, FontItem, FontPage, GoogleFontCatalogItem, RemoteFontFailure, RemoteFontInstallReport, ScanIssue, ScanReport (+1 more)

### Community 41 - "backend.rs"
Cohesion: 0.28
Nodes (4): AppSettingsSnapshot, GooglePreview, RemoteCatalogStatus, ScanRootSettings

### Community 42 - "inspiration_commands.rs"
Cohesion: 0.36
Nodes (4): get_inspiration_asset(), import_inspiration_image(), load_inspiration(), save_inspiration()

### Community 43 - "12. UI Iced"
Cohesion: 0.25
Nodes (8): 12.1 Tema, 12.2 Layout desktop, 12.3 Sidebar, 12.4 Top bar, 12.5 Card font locale/scaricato, 12.6 Card Google cloud, 12.7 Stato e messaggi, 12. UI Iced

### Community 44 - "18. Sequenza di sviluppo obbligatoria"
Cohesion: 0.25
Nodes (8): 18. Sequenza di sviluppo obbligatoria, Fase 1 — Fondazioni, Fase 2 — Scanner, Fase 3 — Attivazione Windows, Fase 4 — Libreria locale UI, Fase 5 — Google Fonts, Fase 6 — Ripristino e resilienza, Fase 7 — Hardening e release

### Community 45 - "FontCategory"
Cohesion: 0.25
Nodes (8): FontCategory, All, Display, Handwriting, Monospace, SansSerif, Serif, Unknown

### Community 46 - "RemoteFontProvider"
Cohesion: 0.29
Nodes (8): RemoteFontCatalogItem, RemoteFontCatalogSnapshot, RemoteFontProvider, Fontshare, FontsourceExtra, Google, NerdFonts, RemoteFontVariant

### Community 47 - "load_cached_remote_catalog()"
Cohesion: 0.29
Nodes (4): download_remote_family(), get_remote_preview_bytes(), load_cached_remote_catalog(), refresh_remote_catalog()

### Community 48 - "Ricerca Globale e Ispirazione Implementation Plan"
Cohesion: 0.29
Nodes (6): Contratto Ispirazione (condiviso), Global Constraints, Review Focus, Revisione richiesta dall'utente dopo il primo smoke, Ricerca Globale e Ispirazione Implementation Plan, Tranche

### Community 49 - "10. Wrapper Win32 per font temporanei"
Cohesion: 0.29
Nodes (7): 10.1 API pubblica, 10.2 Conversione path, 10.3 Attivazione, 10.4 Disattivazione, 10.5 Broadcast, 10.6 Invariante dei riferimenti, 10. Wrapper Win32 per font temporanei

### Community 50 - "IDEA.md"
Cohesion: 0.29
Nodes (6): Async & Networking (Google Fonts API + Download TTF), Database & Persistenza Locale, GUI Nativa GPU-accelerated (Architettura Elm), Integrazione Sistema Operativo & Utility, Parsing Binario dei Font, Windows Win32 API (GDI Font Loading + Broadcast)

### Community 51 - "11. Client Google Fonts"
Cohesion: 0.33
Nodes (6): 11.1 Endpoint, 11.2 Cache catalogo, 11.3 Preview Google, 11.4 Download + attivazione di una variante, 11.5 Download intera famiglia, 11. Client Google Fonts

### Community 52 - "9. Font Iced e interning dei nomi"
Cohesion: 0.33
Nodes (6): 9.1 Vincolo, 9.2 API, 9.3 Implementazione richiesta, 9.4 Variante, 9.5 Caricamento asincrono, 9. Font Iced e interning dei nomi

### Community 53 - "dependencies"
Cohesion: 0.33
Nodes (6): dependencies, react, react-dom, @tauri-apps/api, @tauri-apps/plugin-clipboard-manager, @tauri-apps/plugin-dialog

### Community 54 - "FontSource"
Cohesion: 0.33
Nodes (6): FontSource, Fontshare, Fontsource, Google, Local, NerdFonts

### Community 55 - "default.json"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 56 - "Contribuire a Open Font Manager"
Cohesion: 0.40
Nodes (5): Ambiente, Contribuire a Open Font Manager, Grafo del codice, Pull request, Verifiche prima di una pull request

### Community 57 - "16. Test obbligatori"
Cohesion: 0.40
Nodes (5): 16.1 Unit test, 16.2 Test integrazione, 16.3 Controlli statici obbligatori, 16.4 Test manuale Windows obbligatorio, 16. Test obbligatori

### Community 58 - "1. Obiettivo e confini"
Cohesion: 0.40
Nodes (5): 1.1 Prodotto, 1.2 Funzioni incluse nella prima versione, 1.3 Non inclusi nella prima versione, 1.4 Vincoli non negoziabili, 1. Obiettivo e confini

### Community 59 - "6. Modelli e contratti di dominio"
Cohesion: 0.40
Nodes (5): 6.1 Enumerazioni, 6.2 Font locale persistito, 6.3 Risultato scansione, 6.4 Google DTO, 6. Modelli e contratti di dominio

### Community 60 - "7. Database SQLite"
Cohesion: 0.40
Nodes (5): 7.1 Apertura, 7.2 Schema v1, 7.3 Query obbligatorie, 7.4 Semantica di scansione, 7. Database SQLite

### Community 61 - "8. Scanner OpenType"
Cohesion: 0.40
Nodes (5): 8.1 Interfaccia, 8.2 Regole di filesystem, 8.3 Estrazione metadata, 8.4 Protezione input ostile, 8. Scanner OpenType

### Community 62 - "bug_report.md"
Cohesion: 0.40
Nodes (4): Ambiente, Log e screenshot, Passaggi per riprodurre, Problema e risultato atteso

### Community 63 - "feature_request.md"
Cohesion: 0.40
Nodes (4): Alternative considerate, Comportamento proposto, Criteri per considerarla completata, Problema da risolvere

### Community 64 - "scripts"
Cohesion: 0.40
Nodes (5): scripts, build, dev, tauri, test

### Community 65 - "LibraryTab"
Cohesion: 0.40
Nodes (5): LibraryTab, Active, Favorites, Google, Local

### Community 67 - "13. Stato applicazione e messaggi"
Cohesion: 0.50
Nodes (4): 13.1 Stato minimo, 13.2 Regole `Message`, 13.3 Avvio, 13. Stato applicazione e messaggi

### Community 68 - "5. Directory e file locali"
Cohesion: 0.50
Nodes (4): 5.1 Radice, 5.2 Percorsi obbligatori, 5.3 Scrittura atomica, 5. Directory e file locali

### Community 69 - "PULL_REQUEST_TEMPLATE.md"
Cohesion: 0.50
Nodes (3): Modifiche, Problema risolto / issue collegata, Verifiche eseguite

### Community 70 - "vitest"
Cohesion: 0.50
Nodes (3): @tailwindcss/vite, @vitejs/plugin-react, vitest

### Community 71 - "UpsertOutcome"
Cohesion: 0.50
Nodes (4): UpsertOutcome, Inserted, Unchanged, Updated

### Community 74 - "repository"
Cohesion: 0.67
Nodes (3): repository, type, url

### Community 75 - "FontActivation"
Cohesion: 0.67
Nodes (3): FontActivation, Active, Inactive

### Community 76 - "FontDownloadStrategy"
Cohesion: 0.67
Nodes (3): FontDownloadStrategy, DirectFiles, ZipArchive

### Community 77 - "SnippetKind"
Cohesion: 0.67
Nodes (3): SnippetKind, Css, NextJs

## Knowledge Gaps
- **60 isolated node(s):** `All`, `Display`, `Handwriting`, `Monospace`, `SansSerif` (+55 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 561 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **17 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AppError` connect `AppError` to `cloud_fonts.rs`, `win_font_loader.rs`, `google_library.rs`, `font_scanner.rs`, `cloud_import.rs`, `db.rs`, `next_font_snippet()`, `AppBackend`, `font_install.rs`, `.list_fonts()`, `FontQuery`, `FontRegistration`?**
  _High betweenness centrality (0.079) - this node is a cross-community bridge._
- **Are the 3 inferred relationships involving `App()` (e.g. with `.cancel()` and `.dispose()`) actually correct?**
  _`App()` has 3 INFERRED edges - model-reasoned connections that need verification._
- **What connects `All`, `Display`, `Handwriting` to the rest of the system?**
  _60 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `cloud_fonts.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.0558641975308642 - nodes in this community are weakly interconnected._
- **Why does `FontItem` connect `types.ts` to `InspirationView.tsx`, `GlobalSearchView.tsx`, `App.tsx`, `RustFontManager Implementation Plan`, `CloudCatalogView.tsx`, `11. Client Google Fonts`?**
  _High betweenness centrality (0.059) - this node is a cross-community bridge._
- **Should `db_tests.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.05136986301369863 - nodes in this community are weakly interconnected._
- **Why does `11. Client Google Fonts` connect `11. Client Google Fonts` to `RustFontManager — Specifica tecnica di progetto`?**
  _High betweenness centrality (0.057) - this node is a cross-community bridge._
## Coverage note for open-source preparation

AST refreshed for the current code. New documentation entry points and explicit links were added, but the semantic pass was partial: existing document concepts were preserved, not certified as fully refreshed. Community names use deterministic hub labels, not a new LLM naming pass. Token usage for host-agent extraction is unavailable; zero reported by the local AST pipeline is not a measurement of total session usage.
