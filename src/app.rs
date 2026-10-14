use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use iced::{
    alignment,
    widget::{column, container, pick_list, row, scrollable, slider, text, text_input},
    Element, Length, Task, Theme,
};

use crate::{
    db,
    errors::AppError,
    font_names::font_for_item,
    models::{
        CatalogState, Collection, FontCategory, FontItem, FontPage, FontQuery, FontSource,
        LibraryTab, ScanReport, SnippetKind,
    },
    paths::AppPaths,
    services::{google_library::GoogleLibraryService, local_library::LocalLibraryService},
    snippets,
};

const PAGE_SIZE: u32 = 50;
const DEFAULT_PREVIEW: &str = "The quick brown fox jumps over the lazy dog";

#[derive(Debug)]
pub struct FontApp {
    paths: Option<AppPaths>,
    local: Option<Arc<LocalLibraryService>>,
    google: Option<Arc<GoogleLibraryService>>,
    tab: LibraryTab,
    search: String,
    search_generation: u64,
    load_request_id: u64,
    preview_text: String,
    preview_size: u16,
    category: FontCategory,
    page: u32,
    fonts: Vec<FontItem>,
    total: usize,
    active_count: usize,
    collections: Vec<Collection>,
    selected_collection: Option<i64>,
    collection_name: String,
    google_catalog: Vec<crate::models::GoogleFontCatalogItem>,
    selected_google_family: Option<String>,
    pending_family_download: Option<(String, String)>,
    catalog_state: CatalogState,
    loading: bool,
    scanning: bool,
    toast: Option<(String, bool)>,
    preview_ready: HashMap<String, String>,
    google_preview_items: HashMap<String, FontItem>,
    google_busy: HashSet<String>,
    activation_busy: HashSet<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    StartupLoaded(Result<(AppPaths, Vec<Collection>, FontPage), String>),
    SessionRestored(Result<crate::services::local_library::SessionRestoreReport, String>),
    FontsLoaded {
        request_id: u64,
        result: Result<FontPage, String>,
    },
    ActiveCountLoaded(Result<usize, String>),
    TabSelected(LibraryTab),
    SearchChanged(String),
    SearchDebounceElapsed(u64),
    PreviewTextChanged(String),
    PreviewSizeChanged(u16),
    CategoryChanged(String),
    PageRequested(u32),
    GoogleCatalogPageRequested(u32),
    ChooseFolder,
    FolderChosen(Option<PathBuf>),
    ScanFinished(Result<ScanReport, String>),
    ToggleActivation(String),
    ActivationFinished {
        id: String,
        result: Result<(), String>,
    },
    ToggleFavorite(String),
    FavoriteFinished {
        id: String,
        result: Result<(), String>,
    },
    CreateCollection,
    CollectionNameChanged(String),
    CollectionCreated(Result<Collection, String>),
    CollectionSelected(Option<i64>),
    CollectionsLoaded(Result<Vec<Collection>, String>),
    AddFontToCollection {
        font_id: String,
        collection_name: String,
    },
    CollectionItemAdded(Result<(), String>),
    GoogleCatalogLoaded(Result<(Vec<crate::models::GoogleFontCatalogItem>, bool), String>),
    RefreshGoogle,
    ShowGoogleVariants(String),
    DownloadGoogleVariant {
        family_id: String,
        variant_id: String,
    },
    PreviewGoogleVariant {
        family_id: String,
        variant_id: String,
    },
    GooglePreviewBytesReady {
        key: String,
        result: Result<(Vec<u8>, FontItem), String>,
    },
    GooglePreviewFontLoaded {
        key: String,
        result: Result<(), String>,
    },
    GoogleVariantDownloaded {
        family_id: String,
        variant_id: String,
        result: Result<(), String>,
    },
    RequestFamilyDownload {
        family_id: String,
        family: String,
    },
    ConfirmFamilyDownload,
    CancelFamilyDownload,
    DownloadGoogleFamily(String),
    GoogleFamilyDownloaded {
        family: String,
        result: Result<String, String>,
    },
    CopySnippet {
        id: String,
        kind: SnippetKind,
    },
    CopyFinished(Result<(), String>),
    PreviewRequested(String),
    PreviewBytesReady {
        id: String,
        result: Result<(Vec<u8>, FontItem), String>,
    },
    PreviewFontLoaded {
        id: String,
        family: String,
        result: Result<(), String>,
    },
    DismissToast,
}

impl FontApp {
    pub fn new() -> (Self, Task<Message>) {
        let app = Self {
            paths: None,
            local: None,
            google: None,
            tab: LibraryTab::Local,
            search: String::new(),
            search_generation: 0,
            load_request_id: 0,
            preview_text: DEFAULT_PREVIEW.to_owned(),
            preview_size: 32,
            category: FontCategory::All,
            page: 0,
            fonts: Vec::new(),
            total: 0,
            active_count: 0,
            collections: Vec::new(),
            selected_collection: None,
            collection_name: String::new(),
            google_catalog: Vec::new(),
            selected_google_family: None,
            pending_family_download: None,
            catalog_state: CatalogState::NotLoaded,
            loading: true,
            scanning: false,
            toast: None,
            preview_ready: HashMap::new(),
            google_preview_items: HashMap::new(),
            google_busy: HashSet::new(),
            activation_busy: HashSet::new(),
        };
        let task = Task::perform(
            async {
                tokio::task::spawn_blocking(|| -> Result<_, AppError> {
                    let paths = AppPaths::discover()?;
                    db::migrate(&paths.database)?;
                    let collections = db::list_collections(&paths.database)?;
                    let page = db::list_fonts(
                        &paths.database,
                        make_query(LibraryTab::Local, "", FontCategory::All, None, 0),
                    )?;
                    Ok((paths, collections, page))
                })
                .await
                .map_err(|error| error.to_string())?
                .map_err(|error| error.user_message())
            },
            Message::StartupLoaded,
        );
        (app, task)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::StartupLoaded(Ok((paths, collections, page))) => {
                self.local = Some(Arc::new(LocalLibraryService::new(paths.database.clone())));
                let client = match crate::google_fonts::GoogleFontsClient::new() {
                    Ok(client) => client,
                    Err(error) => {
                        self.toast = Some((error.user_message(), true));
                        self.loading = false;
                        return self.reload_fonts();
                    }
                };
                self.google = Some(Arc::new(GoogleLibraryService::new(client, paths.clone())));
                let local = self.local.clone().expect("local service initialized");
                self.paths = Some(paths);
                self.collections = collections;
                self.fonts = page.items;
                self.total = page.total;
                self.loading = false;
                Task::batch([
                    self.reload_fonts(),
                    self.load_google_catalog(),
                    self.load_active_count(),
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || local.restore_active_fonts())
                                .await
                                .map_err(|error| error.to_string())?
                                .map_err(|error| error.user_message())
                        },
                        Message::SessionRestored,
                    ),
                ])
            }
            Message::StartupLoaded(Err(error)) => {
                self.loading = false;
                self.toast = Some((error, true));
                Task::none()
            }
            Message::FontsLoaded { request_id, result } => {
                if request_id != self.load_request_id {
                    return Task::none();
                }
                self.loading = false;
                match result {
                    Ok(page) => {
                        self.fonts = page.items;
                        self.total = page.total;
                    }
                    Err(error) => self.toast = Some((error, true)),
                }
                Task::none()
            }
            Message::ActiveCountLoaded(Ok(count)) => {
                self.active_count = count;
                Task::none()
            }
            Message::ActiveCountLoaded(Err(error)) => {
                self.toast = Some((error, true));
                Task::none()
            }
            Message::SessionRestored(Ok(report)) => {
                if !report.failed.is_empty() {
                    self.toast = Some((
                        format!(
                            "{} font non ripristinati: {}",
                            report.failed.len(),
                            report.failed.join(", ")
                        ),
                        true,
                    ));
                }
                self.reload_fonts()
            }
            Message::SessionRestored(Err(error)) => {
                self.toast = Some((
                    format!("Ripristino dei font attivi non riuscito: {error}"),
                    true,
                ));
                self.reload_fonts()
            }
            Message::TabSelected(tab) => {
                self.tab = tab;
                self.page = 0;
                self.selected_collection = None;
                self.reload_fonts()
            }
            Message::SearchChanged(search) => {
                self.search = search;
                self.page = 0;
                self.search_generation = self.search_generation.wrapping_add(1);
                self.load_request_id = self.load_request_id.wrapping_add(1);
                let generation = self.search_generation;
                self.loading = true;
                Task::perform(
                    async move {
                        tokio::time::sleep(Duration::from_millis(125)).await;
                        generation
                    },
                    Message::SearchDebounceElapsed,
                )
            }
            Message::SearchDebounceElapsed(generation) => {
                if generation == self.search_generation {
                    self.reload_fonts()
                } else {
                    Task::none()
                }
            }
            Message::PreviewTextChanged(value) => {
                self.preview_text = value;
                Task::none()
            }
            Message::PreviewSizeChanged(size) => {
                self.preview_size = size;
                Task::none()
            }
            Message::CategoryChanged(value) => {
                self.category = category_from_label(&value);
                self.page = 0;
                self.reload_fonts()
            }
            Message::PageRequested(page) => {
                self.page = page;
                self.reload_fonts()
            }
            Message::GoogleCatalogPageRequested(page) => {
                self.page = page;
                Task::none()
            }
            Message::ChooseFolder => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title("Scegli una cartella di font")
                        .pick_folder()
                        .await
                        .map(|handle| handle.path().to_path_buf())
                },
                Message::FolderChosen,
            ),
            Message::FolderChosen(Some(root)) => {
                let Some(service) = self.local.clone() else {
                    self.toast = Some(("L'applicazione non è ancora pronta.".into(), true));
                    return Task::none();
                };
                self.scanning = true;
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || service.scan_directory(&root))
                            .await
                            .map_err(|error| error.to_string())?
                            .map_err(|error| error.user_message())
                    },
                    Message::ScanFinished,
                )
            }
            Message::FolderChosen(None) => Task::none(),
            Message::ScanFinished(result) => {
                self.scanning = false;
                match result {
                    Ok(report) => {
                        self.toast = Some((
                            format!(
                                "Scansione completata: {} nuovi, {} aggiornati, {} errori.",
                                report.imported,
                                report.updated,
                                report.failed.len()
                            ),
                            !report.failed.is_empty(),
                        ));
                        self.reload_fonts()
                    }
                    Err(error) => {
                        self.toast = Some((error, true));
                        Task::none()
                    }
                }
            }
            Message::ToggleActivation(id) => {
                let Some(item) = self.fonts.iter().find(|font| font.id == id).cloned() else {
                    return Task::none();
                };
                if item.missing_on_disk {
                    self.toast = Some(("Il file del font non è più disponibile.".into(), true));
                    return Task::none();
                }
                let Some(service) = self.local.clone() else {
                    return Task::none();
                };
                if !self.activation_busy.insert(id.clone()) {
                    return Task::none();
                }
                let activating = !item.is_active;
                let result_id = id.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            if activating {
                                service.activate(&item.id, &item.file_path).map(|_| ())
                            } else {
                                service.deactivate(&item.id)
                            }
                        })
                        .await
                        .map_err(|error| error.to_string())?
                        .map_err(|error| error.user_message())
                    },
                    move |result| Message::ActivationFinished {
                        id: result_id.clone(),
                        result,
                    },
                )
            }
            Message::ActivationFinished { id, result } => {
                self.activation_busy.remove(&id);
                self.toast = Some(match result {
                    Ok(()) => ("Stato di attivazione aggiornato.".into(), false),
                    Err(error) => (error, true),
                });
                Task::batch([self.reload_fonts(), self.load_active_count()])
            }
            Message::ToggleFavorite(id) => {
                let Some(paths) = self.paths.clone() else {
                    return Task::none();
                };
                let result_id = id.clone();
                let current = self
                    .fonts
                    .iter()
                    .find(|font| font.id == id)
                    .is_some_and(|font| font.is_favorite);
                Task::perform(
                    async move {
                        let update_id = id;
                        tokio::task::spawn_blocking(move || {
                            db::set_font_favorite(&paths.database, &update_id, !current)
                        })
                        .await
                        .map_err(|error| error.to_string())?
                        .map_err(|error| error.user_message())
                    },
                    move |result| Message::FavoriteFinished {
                        id: result_id.clone(),
                        result,
                    },
                )
            }
            Message::FavoriteFinished { result, .. } => {
                if let Err(error) = result {
                    self.toast = Some((error, true));
                }
                self.reload_fonts()
            }
            Message::CreateCollection => {
                let name = self.collection_name.trim().to_owned();
                let Some(paths) = self.paths.clone() else {
                    return Task::none();
                };
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            db::create_collection(&paths.database, &name)
                        })
                        .await
                        .map_err(|error| error.to_string())?
                        .map_err(|error| error.user_message())
                    },
                    Message::CollectionCreated,
                )
            }
            Message::CollectionNameChanged(name) => {
                self.collection_name = name;
                Task::none()
            }
            Message::CollectionCreated(Ok(collection)) => {
                self.collection_name.clear();
                self.collections.push(collection);
                self.toast = Some(("Collezione creata.".into(), false));
                Task::none()
            }
            Message::CollectionCreated(Err(error)) => {
                self.toast = Some((error, true));
                Task::none()
            }
            Message::CollectionSelected(id) => {
                self.selected_collection = id;
                self.tab = LibraryTab::Local;
                self.page = 0;
                self.reload_fonts()
            }
            Message::CollectionsLoaded(Ok(collections)) => {
                self.collections = collections;
                Task::none()
            }
            Message::CollectionsLoaded(Err(error)) => {
                self.toast = Some((error, true));
                Task::none()
            }
            Message::AddFontToCollection {
                font_id,
                collection_name,
            } => {
                let Some(collection) = self
                    .collections
                    .iter()
                    .find(|item| item.name == collection_name)
                else {
                    return Task::none();
                };
                let collection_id = collection.id;
                let Some(paths) = self.paths.clone() else {
                    return Task::none();
                };
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            db::add_font_to_collection(&paths.database, collection_id, &font_id)
                        })
                        .await
                        .map_err(|error| error.to_string())?
                        .map_err(|error| error.user_message())
                    },
                    Message::CollectionItemAdded,
                )
            }
            Message::CollectionItemAdded(Ok(())) => {
                self.toast = Some(("Font aggiunto alla collezione.".into(), false));
                let Some(paths) = self.paths.clone() else {
                    return Task::none();
                };
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || db::list_collections(&paths.database))
                            .await
                            .map_err(|error| error.to_string())?
                            .map_err(|error| error.user_message())
                    },
                    Message::CollectionsLoaded,
                )
            }
            Message::CollectionItemAdded(Err(error)) => {
                self.toast = Some((error, true));
                Task::none()
            }
            Message::GoogleCatalogLoaded(Ok((items, stale))) => {
                self.google_catalog = items;
                self.catalog_state = CatalogState::Ready { stale };
                Task::none()
            }
            Message::GoogleCatalogLoaded(Err(error)) => {
                self.catalog_state = CatalogState::Failed(error.clone());
                self.toast = Some((error, true));
                Task::none()
            }
            Message::RefreshGoogle => self.load_google_catalog(),
            Message::ShowGoogleVariants(family_id) => {
                self.selected_google_family = Some(family_id);
                Task::none()
            }
            Message::DownloadGoogleVariant {
                family_id,
                variant_id,
            } => {
                let operation_key = google_download_key(&family_id, &variant_id);
                if !self.google_busy.insert(operation_key.clone()) {
                    return Task::none();
                }
                let Some(service) = self.google.clone() else {
                    self.google_busy.remove(&operation_key);
                    return Task::none();
                };
                let message_family = family_id.clone();
                let message_variant = variant_id.clone();
                Task::perform(
                    async move {
                        service
                            .install_variant(&family_id, &variant_id)
                            .await
                            .map(|_| ())
                            .map_err(|error| error.user_message())
                    },
                    move |result| Message::GoogleVariantDownloaded {
                        family_id: message_family.clone(),
                        variant_id: message_variant.clone(),
                        result,
                    },
                )
            }
            Message::PreviewGoogleVariant {
                family_id,
                variant_id,
            } => {
                let key = google_preview_key(&family_id, &variant_id);
                if self.preview_ready.contains_key(&key) || !self.google_busy.insert(key.clone()) {
                    return Task::none();
                }
                let Some(service) = self.google.clone() else {
                    self.google_busy.remove(&key);
                    return Task::none();
                };
                Task::perform(
                    async move {
                        let (bytes, metadata) = service
                            .preview_variant(&family_id, Some(&variant_id))
                            .await
                            .map_err(|error| error.user_message())?;
                        let item = FontItem {
                            id: google_preview_key(&family_id, &variant_id),
                            family: metadata.family,
                            subfamily: metadata.subfamily,
                            full_name: metadata.full_name,
                            weight: metadata.weight,
                            is_italic: metadata.is_italic,
                            file_path: PathBuf::new(),
                            source: FontSource::Google,
                            source_family_id: Some(family_id),
                            source_variant_id: Some(variant_id),
                            category: FontCategory::Unknown,
                            is_active: false,
                            is_favorite: false,
                            file_size: metadata.file_size,
                            file_modified_unix_ms: metadata.file_modified_unix_ms,
                            missing_on_disk: false,
                        };
                        Ok::<_, String>((bytes, item))
                    },
                    move |result| Message::GooglePreviewBytesReady {
                        key: key.clone(),
                        result,
                    },
                )
            }
            Message::GooglePreviewBytesReady {
                key,
                result: Ok((bytes, item)),
            } => {
                let font_id = key.clone();
                self.google_busy.remove(&key);
                self.google_preview_items.insert(key.clone(), item);
                iced::font::load(bytes).map(move |result| Message::GooglePreviewFontLoaded {
                    key: font_id.clone(),
                    result: result.map_err(|never| match never {}),
                })
            }
            Message::GooglePreviewBytesReady {
                key,
                result: Err(error),
            } => {
                self.google_busy.remove(&key);
                self.toast = Some((error, true));
                Task::none()
            }
            Message::GooglePreviewFontLoaded {
                key,
                result: Ok(()),
            } => {
                self.google_busy.remove(&key);
                self.preview_ready.insert(key.clone(), key);
                Task::none()
            }
            Message::GooglePreviewFontLoaded {
                key,
                result: Err(error),
            } => {
                self.google_busy.remove(&key);
                self.google_preview_items.remove(&key);
                self.toast = Some((
                    format!("Caricamento anteprima Google Fonts non riuscito: {error}"),
                    true,
                ));
                Task::none()
            }
            Message::GoogleVariantDownloaded {
                family_id,
                variant_id,
                result,
            } => {
                self.google_busy
                    .remove(&google_download_key(&family_id, &variant_id));
                self.toast = Some(match result {
                    Ok(()) => (
                        format!("Variante {variant_id} scaricata nel vault locale."),
                        false,
                    ),
                    Err(error) => (error, true),
                });
                self.reload_fonts()
            }
            Message::RequestFamilyDownload { family_id, family } => {
                self.pending_family_download = Some((family_id, family));
                Task::none()
            }
            Message::CancelFamilyDownload => {
                self.pending_family_download = None;
                Task::none()
            }
            Message::ConfirmFamilyDownload => match self.pending_family_download.take() {
                Some((family_id, _)) => self.download_google_family(family_id),
                None => Task::none(),
            },
            Message::DownloadGoogleFamily(family_id) => {
                let Some(service) = self.google.clone() else {
                    return Task::none();
                };
                let result_family = family_id.clone();
                Task::perform(
                    async move {
                        let report = service
                            .download_family(&family_id)
                            .await
                            .map_err(|error| error.user_message())?;
                        let text = format!(
                            "{} varianti scaricate, {} non riuscite.",
                            report.installed.len(),
                            report.failed.len()
                        );
                        Ok::<_, String>(text)
                    },
                    move |result| Message::GoogleFamilyDownloaded {
                        family: result_family.clone(),
                        result,
                    },
                )
            }
            Message::GoogleFamilyDownloaded { family, result } => {
                self.toast = Some(match result {
                    Ok(summary) => (format!("{family}: {summary}"), false),
                    Err(error) => (error, true),
                });
                if self.tab == LibraryTab::Local {
                    self.reload_fonts()
                } else {
                    Task::none()
                }
            }
            Message::CopySnippet { id, kind } => {
                let Some(item) = self.fonts.iter().find(|font| font.id == id).cloned() else {
                    return Task::none();
                };
                let snippet = match match kind {
                    SnippetKind::Css => Ok(snippets::css_snippet(&item)),
                    SnippetKind::NextJs => snippets::next_font_snippet(&item),
                } {
                    Ok(result) => result,
                    Err(error) => {
                        self.toast = Some((error.user_message(), true));
                        return Task::none();
                    }
                };
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let mut clipboard = arboard::Clipboard::new()
                                .map_err(|error| AppError::Clipboard(error.to_string()))?;
                            clipboard
                                .set_text(snippet)
                                .map_err(|error| AppError::Clipboard(error.to_string()))
                        })
                        .await
                        .map_err(|error| error.to_string())?
                        .map_err(|error| error.user_message())
                    },
                    Message::CopyFinished,
                )
            }
            Message::CopyFinished(Ok(())) => {
                self.toast = Some(("Snippet copiato negli appunti.".into(), false));
                Task::none()
            }
            Message::CopyFinished(Err(error)) => {
                self.toast = Some((error, true));
                Task::none()
            }
            Message::PreviewRequested(id) => {
                let Some(item) = self.fonts.iter().find(|font| font.id == id).cloned() else {
                    return Task::none();
                };
                if item.missing_on_disk {
                    self.toast = Some(("Il file del font non è più disponibile.".into(), true));
                    return Task::none();
                }
                let result_id = id.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let bytes = std::fs::read(&item.file_path).map_err(|error| {
                                AppError::Storage {
                                    path: item.file_path.clone(),
                                    reason: error.to_string(),
                                }
                            })?;
                            Ok::<_, AppError>((bytes, item))
                        })
                        .await
                        .map_err(|error| error.to_string())?
                        .map_err(|error| error.user_message())
                    },
                    move |result| Message::PreviewBytesReady {
                        id: result_id.clone(),
                        result,
                    },
                )
            }
            Message::PreviewBytesReady {
                id,
                result: Ok((bytes, item)),
            } => {
                let family = item.family.clone();
                Task::batch([iced::font::load(bytes).map(move |result| {
                    Message::PreviewFontLoaded {
                        id: id.clone(),
                        family: family.clone(),
                        result: result.map_err(|never| match never {}),
                    }
                })])
            }
            Message::PreviewBytesReady {
                result: Err(error), ..
            } => {
                self.toast = Some((error, true));
                Task::none()
            }
            Message::PreviewFontLoaded {
                id,
                family,
                result: Ok(()),
            } => {
                self.preview_ready.insert(id.clone(), family);
                self.toast = Some((format!("Anteprima caricata per {id}."), false));
                Task::none()
            }
            Message::PreviewFontLoaded {
                result: Err(error), ..
            } => {
                self.toast = Some((error, true));
                Task::none()
            }
            Message::DismissToast => {
                self.toast = None;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let sidebar = self.sidebar();
        let content = self.content();
        row![sidebar, content]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn theme(&self) -> Theme {
        crate::theme::rust_font_manager()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let mut nav = column![
            text("RustFontManager").size(24),
            text(format!(
                "{} font attivi in questa sessione",
                self.active_count
            ))
            .size(13)
        ]
        .spacing(14);
        for (tab, label) in [
            (LibraryTab::Local, "Libreria locale"),
            (LibraryTab::Active, "Attivi ora"),
            (LibraryTab::Favorites, "Preferiti"),
            (LibraryTab::Google, "Google Fonts"),
        ] {
            let label = if self.tab == tab {
                format!("●  {label}")
            } else {
                format!("   {label}")
            };
            nav = nav.push(
                iced::widget::button(text(label))
                    .style(if self.tab == tab {
                        iced::widget::button::primary
                    } else {
                        iced::widget::button::secondary
                    })
                    .width(Length::Fill)
                    .on_press(Message::TabSelected(tab)),
            );
        }
        nav = nav.push(text("Collezioni").size(18));
        for collection in &self.collections {
            nav = nav.push(
                secondary_button(text(format!(
                    "{}  ·  {}",
                    collection.name, collection.font_count
                )))
                .width(Length::Fill)
                .on_press(Message::CollectionSelected(Some(collection.id))),
            );
        }
        nav = nav.push(
            row![
                text_input("Nuova collezione", &self.collection_name)
                    .on_input(Message::CollectionNameChanged)
                    .on_submit(Message::CreateCollection),
                secondary_button(text("Crea")).on_press_maybe(
                    (!self.collection_name.trim().is_empty()).then_some(Message::CreateCollection),
                ),
            ]
            .spacing(6),
        );
        nav = nav.push(
            secondary_button(text(if self.scanning {
                "Scansione in corso…"
            } else {
                "+ Aggiungi cartella locale"
            }))
            .width(Length::Fill)
            .on_press_maybe((!self.scanning).then_some(Message::ChooseFolder)),
        );
        container(scrollable(nav.spacing(8).padding(18)))
            .width(240)
            .height(Length::Fill)
            .into()
    }

    fn content(&self) -> Element<'_, Message> {
        let title = match self.tab {
            LibraryTab::Local => "Libreria locale",
            LibraryTab::Active => "Attivi ora",
            LibraryTab::Favorites => "Preferiti",
            LibraryTab::Google => "Google Fonts",
        };
        let mut body = column![text(title).size(30)].spacing(18).padding(24);
        if self.tab == LibraryTab::Google {
            body = body.push(
                row![
                    text_input("Cerca Google Fonts", &self.search)
                        .on_input(Message::SearchChanged)
                        .width(Length::Fill),
                    secondary_button(text(match self.catalog_state {
                        CatalogState::Loading => "Aggiornamento…",
                        _ => "Aggiorna catalogo",
                    }))
                    .on_press_maybe(
                        (!matches!(self.catalog_state, CatalogState::Loading))
                            .then_some(Message::RefreshGoogle)
                    )
                ]
                .spacing(8),
            );
            body = body.push(
                row![
                    text_input("Testo di anteprima", &self.preview_text)
                        .on_input(Message::PreviewTextChanged)
                        .width(Length::Fill),
                    slider(14.0..=96.0, f32::from(self.preview_size), |value| {
                        Message::PreviewSizeChanged(value.round() as u16)
                    })
                    .step(1.0_f32)
                    .width(Length::Fill),
                ]
                .spacing(12)
                .align_y(alignment::Vertical::Center),
            );
            body = body.push(text(match &self.catalog_state {
                CatalogState::Loading => "Caricamento catalogo…".to_owned(),
                CatalogState::Ready { stale: true } => {
                    "Catalogo in cache, aggiornamento non riuscito.".to_owned()
                }
                CatalogState::Ready { stale: false } => {
                    format!("{} famiglie disponibili", self.google_catalog.len())
                }
                CatalogState::Offline => "Catalogo offline".to_owned(),
                CatalogState::Failed(error) => error.clone(),
                CatalogState::NotLoaded => "Catalogo non ancora caricato".to_owned(),
            }));
            let needle = self.search.to_lowercase();
            let filtered_catalog: Vec<_> = self
                .google_catalog
                .iter()
                .filter(|item| item.family.to_lowercase().contains(&needle))
                .collect();
            let catalog_total = filtered_catalog.len();
            let mut rows = filtered_catalog
                .iter()
                .skip((self.page * PAGE_SIZE) as usize)
                .take(PAGE_SIZE as usize)
                .fold(column![].spacing(10), |rows, item| {
                    let variants = if item.variants.is_empty() {
                        0
                    } else {
                        item.variants.len()
                    };
                    rows.push(
                        container(
                            row![
                                column![
                                    text(&item.family).size(22),
                                    text(format!("{} · {} varianti", item.category, variants))
                                ]
                                .spacing(6),
                                secondary_button(text("Scarica famiglia")).on_press(
                                    Message::RequestFamilyDownload {
                                        family_id: item.id.clone(),
                                        family: item.family.clone()
                                    }
                                ),
                                secondary_button(text("Varianti"))
                                    .on_press(Message::ShowGoogleVariants(item.id.clone()))
                            ]
                            .spacing(14)
                            .align_y(alignment::Vertical::Center),
                        )
                        .padding(14)
                        .width(Length::Fill),
                    )
                });
            if filtered_catalog.is_empty() {
                rows = rows.push(text(if self.google_catalog.is_empty() {
                    "Catalogo vuoto: aggiorna quando la connessione è disponibile."
                } else {
                    "Nessuna famiglia corrisponde alla ricerca."
                }));
            }
            if catalog_total > PAGE_SIZE as usize {
                rows = rows.push(
                    row![
                        secondary_button(text("Pagina precedente")).on_press_maybe(
                            self.page
                                .checked_sub(1)
                                .map(Message::GoogleCatalogPageRequested)
                        ),
                        text(format!(
                            "Pagina {} · {} famiglie",
                            self.page + 1,
                            catalog_total
                        )),
                        secondary_button(text("Pagina successiva")).on_press_maybe(
                            (((self.page + 1) * PAGE_SIZE) < catalog_total as u32)
                                .then_some(Message::GoogleCatalogPageRequested(self.page + 1)),
                        ),
                    ]
                    .spacing(12),
                );
            }
            if let Some(item) = self
                .google_catalog
                .iter()
                .find(|item| self.selected_google_family.as_deref() == Some(item.id.as_str()))
            {
                let variant_rows = item.variants.iter().fold(
                    column![text(format!("Varianti di {}", item.family)).size(18)].spacing(8),
                    |rows, variant_id| {
                        let key = google_preview_key(&item.id, variant_id);
                        let preview_busy = self.google_busy.contains(&key);
                        let preview_ready = self.preview_ready.contains_key(&key);
                        let download_key = google_download_key(&item.id, variant_id);
                        let download_busy = self.google_busy.contains(&download_key);
                        let sample: Element<'_, Message> = match self.google_preview_items.get(&key)
                        {
                            Some(font) if self.preview_ready.contains_key(&key) => {
                                text(self.preview_text.clone())
                                    .font(font_for_item(font))
                                    .size(self.preview_size)
                                    .into()
                            }
                            _ => text("Anteprima non caricata").size(12).into(),
                        };
                        rows.push(
                            column![
                                row![
                                    text(variant_id),
                                    secondary_button(text(if preview_busy {
                                        "Caricamento…"
                                    } else if preview_ready {
                                        "Anteprima pronta"
                                    } else {
                                        "Anteprima"
                                    }))
                                    .on_press_maybe(
                                        (!preview_busy && !preview_ready).then(|| {
                                            Message::PreviewGoogleVariant {
                                                family_id: item.id.clone(),
                                                variant_id: variant_id.clone(),
                                            }
                                        })
                                    ),
                                    secondary_button(text(if download_busy {
                                        "Download…"
                                    } else {
                                        "Scarica variante"
                                    }))
                                    .on_press_maybe(
                                        (!download_busy).then(|| {
                                            Message::DownloadGoogleVariant {
                                                family_id: item.id.clone(),
                                                variant_id: variant_id.clone(),
                                            }
                                        })
                                    )
                                ]
                                .spacing(10)
                                .align_y(alignment::Vertical::Center),
                                sample,
                            ]
                            .spacing(6),
                        )
                    },
                );
                rows = rows.push(container(variant_rows).padding(12));
            }
            if let Some((_, family)) = &self.pending_family_download {
                rows = rows.push(container(column![
                    text(format!("Scaricare tutte le varianti di {family}?")),
                    text("Il download può includere molti file, salvati nel vault locale dell'app."),
                    row![
                        secondary_button(text("Conferma download")).on_press(Message::ConfirmFamilyDownload),
                        secondary_button(text("Annulla")).on_press(Message::CancelFamilyDownload),
                    ].spacing(8),
                ].spacing(8).padding(14)).width(Length::Fill));
            }
            let mut downloaded = column![text("Scaricati nel vault locale").size(20)].spacing(12);
            let downloaded_fonts: Vec<_> = self
                .fonts
                .iter()
                .filter(|font| font.source == FontSource::Google)
                .collect();
            if downloaded_fonts.is_empty() {
                downloaded =
                    downloaded.push(text("Non hai ancora scaricato varianti Google Fonts."));
            } else {
                for font in downloaded_fonts {
                    downloaded = downloaded.push(self.font_card(font));
                }
            }
            rows = rows.push(downloaded);
            body = body.push(scrollable(rows).height(Length::Fill));
        } else {
            body = body.push(
                row![
                    text_input("Cerca per famiglia o stile…", &self.search)
                        .on_input(Message::SearchChanged)
                        .width(Length::Fill),
                    text_input("Il tuo testo di anteprima", &self.preview_text)
                        .on_input(Message::PreviewTextChanged)
                        .width(Length::Fill),
                ]
                .spacing(10)
                .align_y(alignment::Vertical::Center),
            );
            body = body.push(
                row![
                    text("Dimensione"),
                    slider(14.0..=96.0, f32::from(self.preview_size), |value| {
                        Message::PreviewSizeChanged(value.round() as u16)
                    })
                    .step(1.0_f32)
                    .width(Length::Fill),
                    pick_list(
                        category_labels(),
                        Some(category_label(self.category)),
                        Message::CategoryChanged
                    )
                    .width(180),
                ]
                .spacing(12)
                .align_y(alignment::Vertical::Center),
            );
            if self.loading {
                body = body.push(text("Caricamento font…"));
            } else if self.fonts.is_empty() {
                body = body.push(text(
                    "Nessun font in questa vista. Aggiungi una cartella o modifica i filtri.",
                ));
            }
            let rows = self.fonts.iter().fold(column![].spacing(12), |rows, item| {
                rows.push(self.font_card(item))
            });
            body = body.push(scrollable(rows).height(Length::Fill));
            if self.total > PAGE_SIZE as usize {
                body = body.push(
                    row![
                        secondary_button(text("Pagina precedente"))
                            .on_press_maybe(self.page.checked_sub(1).map(Message::PageRequested)),
                        text(format!("Pagina {} · {} font", self.page + 1, self.total)),
                        secondary_button(text("Pagina successiva")).on_press_maybe(
                            (((self.page + 1) * PAGE_SIZE) < self.total as u32)
                                .then_some(Message::PageRequested(self.page + 1)),
                        ),
                    ]
                    .spacing(12),
                );
            }
        }
        if self.scanning {
            body = body.push(text("Scansione cartella in corso…"));
        }
        if let Some((message, is_error)) = &self.toast {
            body = body.push(
                row![
                    text(if *is_error {
                        format!("Errore: {message}")
                    } else {
                        message.clone()
                    }),
                    secondary_button(text("Chiudi")).on_press(Message::DismissToast)
                ]
                .spacing(10),
            );
        }
        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn font_card<'a>(&self, item: &'a FontItem) -> Element<'a, Message> {
        let source = if item.source == FontSource::Google {
            "GOOGLE"
        } else {
            "LOCALE"
        };
        let preview_text = self.preview_text.clone();
        let preview = if self.preview_ready.contains_key(&item.id) {
            text(preview_text)
                .font(font_for_item(item))
                .size(self.preview_size)
        } else {
            text(preview_text).size(self.preview_size)
        };
        let preview_label = if self.preview_ready.contains_key(&item.id) {
            "Anteprima caricata"
        } else {
            "Anteprima non caricata"
        };
        let status = if item.missing_on_disk {
            "FILE MANCANTE".to_owned()
        } else if item.is_active {
            "Attivo su Windows".to_owned()
        } else {
            "Disattivato".to_owned()
        };
        let activation_busy = self.activation_busy.contains(&item.id);
        let actions = row![
            secondary_button(text(if activation_busy {
                "Aggiornamento…"
            } else if item.is_active {
                "Disattiva"
            } else {
                "Attiva su Windows"
            }))
            .on_press_maybe(
                (!item.missing_on_disk && !activation_busy)
                    .then(|| Message::ToggleActivation(item.id.clone()))
            ),
            secondary_button(text(if item.is_favorite {
                "Rimuovi dai preferiti"
            } else {
                "Aggiungi ai preferiti"
            }))
            .on_press(Message::ToggleFavorite(item.id.clone())),
            secondary_button(text("Anteprima")).on_press_maybe(
                (!item.missing_on_disk).then(|| Message::PreviewRequested(item.id.clone()))
            ),
            secondary_button(text("Copia CSS")).on_press_maybe((!item.missing_on_disk).then(
                || {
                    Message::CopySnippet {
                        id: item.id.clone(),
                        kind: SnippetKind::Css,
                    }
                }
            )),
            secondary_button(text("Copia Next.js")).on_press_maybe((!item.missing_on_disk).then(
                || {
                    Message::CopySnippet {
                        id: item.id.clone(),
                        kind: SnippetKind::NextJs,
                    }
                }
            )),
        ]
        .spacing(8);
        let collection_picker: Element<'a, Message> = if self.collections.is_empty() {
            text("Crea una collezione per organizzare i font").into()
        } else {
            pick_list(
                self.collections
                    .iter()
                    .map(|collection| collection.name.clone())
                    .collect::<Vec<_>>(),
                None::<String>,
                {
                    let font_id = item.id.clone();
                    move |collection_name| Message::AddFontToCollection {
                        font_id: font_id.clone(),
                        collection_name,
                    }
                },
            )
            .placeholder("Aggiungi a collezione")
            .into()
        };
        container(
            column![
                row![
                    column![
                        text(&item.family).size(22),
                        text(format!(
                            "{} · {} · peso {}{}",
                            source,
                            item.category_label(),
                            item.weight,
                            if item.is_italic { " · corsivo" } else { "" }
                        ))
                        .size(13)
                    ]
                    .spacing(4),
                    text(status)
                ]
                .spacing(12)
                .align_y(alignment::Vertical::Center),
                text(preview_label).size(12),
                preview,
                actions,
                collection_picker,
            ]
            .spacing(12)
            .padding(16),
        )
        .width(Length::Fill)
        .into()
    }

    fn reload_fonts(&mut self) -> Task<Message> {
        let Some(paths) = self.paths.clone() else {
            return Task::none();
        };
        let query = make_query(
            self.tab,
            &self.search,
            self.category,
            self.selected_collection,
            self.page,
        );
        self.load_request_id = self.load_request_id.wrapping_add(1);
        let request_id = self.load_request_id;
        self.loading = true;
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || db::list_fonts(&paths.database, query))
                    .await
                    .map_err(|error| error.to_string())?
                    .map_err(|error| error.user_message())
            },
            move |result| Message::FontsLoaded { request_id, result },
        )
    }

    fn load_google_catalog(&mut self) -> Task<Message> {
        let Some(service) = self.google.clone() else {
            return Task::none();
        };
        self.catalog_state = CatalogState::Loading;
        Task::perform(
            async move {
                service
                    .refresh_catalog()
                    .await
                    .map(|snapshot| (snapshot.items, snapshot.stale))
                    .map_err(|error| error.user_message())
            },
            Message::GoogleCatalogLoaded,
        )
    }

    fn load_active_count(&self) -> Task<Message> {
        let Some(paths) = self.paths.clone() else {
            return Task::none();
        };
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || db::count_active_fonts(&paths.database))
                    .await
                    .map_err(|error| error.to_string())?
                    .map_err(|error| error.user_message())
            },
            Message::ActiveCountLoaded,
        )
    }

    fn download_google_family(&self, family_id: String) -> Task<Message> {
        let Some(service) = self.google.clone() else {
            return Task::none();
        };
        let family = family_id.clone();
        Task::perform(
            async move {
                let report = service
                    .download_family(&family_id)
                    .await
                    .map_err(|error| error.user_message())?;
                Ok::<_, String>(format!(
                    "{} varianti scaricate, {} non riuscite.",
                    report.installed.len(),
                    report.failed.len()
                ))
            },
            move |result| Message::GoogleFamilyDownloaded {
                family: family.clone(),
                result,
            },
        )
    }
}

fn make_query(
    tab: LibraryTab,
    search: &str,
    category: FontCategory,
    collection_id: Option<i64>,
    page: u32,
) -> FontQuery {
    FontQuery {
        tab,
        search: search.to_owned(),
        category,
        collection_id,
        page,
        page_size: PAGE_SIZE,
    }
}

fn google_preview_key(family_id: &str, variant_id: &str) -> String {
    format!("google:{family_id}:{variant_id}")
}

fn google_download_key(family_id: &str, variant_id: &str) -> String {
    format!("download:{family_id}:{variant_id}")
}

fn category_labels() -> Vec<String> {
    ["Tutte", "Sans-serif", "Serif", "Display", "Monospace"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}
fn category_label(value: FontCategory) -> String {
    match value {
        FontCategory::All | FontCategory::Unknown => "Tutte",
        FontCategory::SansSerif => "Sans-serif",
        FontCategory::Serif => "Serif",
        FontCategory::Display => "Display",
        FontCategory::Monospace => "Monospace",
    }
    .to_owned()
}
fn category_from_label(value: &str) -> FontCategory {
    match value {
        "Sans-serif" => FontCategory::SansSerif,
        "Serif" => FontCategory::Serif,
        "Display" => FontCategory::Display,
        "Monospace" => FontCategory::Monospace,
        _ => FontCategory::All,
    }
}

trait CategoryLabel {
    fn category_label(&self) -> &'static str;
}
impl CategoryLabel for FontItem {
    fn category_label(&self) -> &'static str {
        match self.category {
            FontCategory::All => "Tutte",
            FontCategory::Unknown => "Non classificato",
            FontCategory::SansSerif => "Sans-serif",
            FontCategory::Serif => "Serif",
            FontCategory::Display => "Display",
            FontCategory::Monospace => "Monospace",
        }
    }
}

fn secondary_button<'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Button<'a, Message> {
    iced::widget::button(content).style(iced::widget::button::text)
}
