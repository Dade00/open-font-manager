use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontSource {
    Local,
    Google,
    Fontsource,
    Fontshare,
    #[serde(rename = "nerd-fonts")]
    NerdFonts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FontCategory {
    #[default]
    #[serde(rename = "all")]
    All,
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "sans-serif")]
    SansSerif,
    #[serde(rename = "serif")]
    Serif,
    #[serde(rename = "display")]
    Display,
    #[serde(rename = "handwriting")]
    Handwriting,
    #[serde(rename = "monospace")]
    Monospace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoogleFontCatalogItem {
    pub id: String,
    pub family: String,
    #[serde(default)]
    pub subsets: Vec<String>,
    #[serde(default)]
    pub category: String,
    #[serde(default, rename = "defVariant")]
    pub default_variant: String,
    #[serde(default, rename = "defSubset")]
    pub default_subset: String,
    #[serde(default)]
    pub variants: Vec<String>,
    #[serde(default)]
    pub popularity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoogleFontVariant {
    pub id: String,
    #[serde(default, rename = "fontWeight")]
    pub font_weight: String,
    #[serde(default, rename = "fontStyle")]
    pub font_style: String,
    #[serde(default)]
    pub ttf: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoogleFontDetails {
    pub id: String,
    pub family: String,
    #[serde(default)]
    pub subsets: Vec<String>,
    #[serde(default)]
    pub category: String,
    #[serde(default, rename = "defVariant")]
    pub default_variant: String,
    #[serde(default, rename = "defSubset")]
    pub default_subset: String,
    #[serde(default)]
    pub variants: Vec<GoogleFontVariant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LibraryTab {
    #[default]
    Local,
    Active,
    Favorites,
    Google,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontActivation {
    Active,
    Inactive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontItem {
    pub id: String,
    pub family: String,
    pub subfamily: String,
    pub full_name: String,
    pub weight: u16,
    pub is_italic: bool,
    pub is_variable: bool,
    pub file_path: PathBuf,
    pub source: FontSource,
    pub source_family_id: Option<String>,
    pub source_variant_id: Option<String>,
    pub category: FontCategory,
    pub is_active: bool,
    pub is_installed: bool,
    pub is_favorite: bool,
    pub file_size: u64,
    pub file_modified_unix_ms: i64,
    pub missing_on_disk: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanIssue {
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub root: PathBuf,
    pub discovered_candidates: usize,
    pub imported: usize,
    pub updated: usize,
    pub skipped_unchanged: usize,
    pub skipped_unsupported: usize,
    pub failed: Vec<ScanIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub font_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRoot {
    pub path: PathBuf,
    pub added_at_unix_ms: i64,
    pub last_scanned_at_unix_ms: Option<i64>,
    pub is_enabled: bool,
    pub last_discovered_candidates: usize,
    pub last_imported: usize,
    pub last_updated: usize,
    pub last_skipped_unchanged: usize,
    pub last_skipped_unsupported: usize,
    pub last_failed: usize,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontQuery {
    pub tab: LibraryTab,
    pub search: String,
    pub category: FontCategory,
    pub collection_id: Option<i64>,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontPage {
    pub items: Vec<FontItem>,
    pub total: usize,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpsertOutcome {
    Inserted,
    Updated,
    Unchanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnippetKind {
    Css,
    NextJs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedFontMetadata {
    pub family: String,
    pub subfamily: String,
    pub full_name: String,
    pub weight: u16,
    pub is_italic: bool,
    pub is_variable: bool,
    pub file_size: u64,
    pub file_modified_unix_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RemoteFontProvider {
    Google,
    FontsourceExtra,
    Fontshare,
    NerdFonts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontDownloadStrategy {
    DirectFiles,
    ZipArchive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFontVariant {
    pub id: String,
    pub label: String,
    pub weight: u16,
    pub is_italic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFontCatalogItem {
    pub id: String,
    pub provider: RemoteFontProvider,
    pub family: String,
    pub category: FontCategory,
    pub variants: Vec<RemoteFontVariant>,
    pub preview_variant_id: Option<String>,
    pub download_strategy: FontDownloadStrategy,
    pub download_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFontCatalogSnapshot {
    pub provider: RemoteFontProvider,
    pub items: Vec<RemoteFontCatalogItem>,
    pub fetched_at_unix_ms: i64,
    pub stale: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFontFailure {
    pub variant_id: String,
    pub stage: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFontInstallReport {
    pub provider: RemoteFontProvider,
    pub family_id: String,
    pub family: String,
    pub saved_files: usize,
    pub indexed_fonts: usize,
    pub activated_fonts: usize,
    pub failures: Vec<RemoteFontFailure>,
}
