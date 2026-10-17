use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures::StreamExt;
use reqwest::{header, redirect::Policy, Client, Response, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tempfile::NamedTempFile;

use crate::{
    errors::AppError,
    models::{
        FontCategory, FontDownloadStrategy, GoogleFontCatalogItem, RemoteFontCatalogItem,
        RemoteFontCatalogSnapshot, RemoteFontProvider, RemoteFontVariant,
    },
};

const FONTSOURCE_CATALOG_URL: &str = "https://api.fontsource.org/v1/fonts";
const FONTSOURCE_DETAILS_URL: &str = "https://api.fontsource.org/v1/fonts";
const FONTSHARE_CATALOG_URL: &str = "https://api.fontshare.com/v2/fonts?offset=0&limit=150";
const NERD_FONTS_RELEASE_URL: &str =
    "https://api.github.com/repos/ryanoasis/nerd-fonts/releases/latest";
const USER_AGENT: &str = "OpenFontManager/0.1";
const JSON_RESPONSE_LIMIT: u64 = 24 * 1024 * 1024;
const FONT_RESPONSE_LIMIT: u64 = 32 * 1024 * 1024;
const ARCHIVE_RESPONSE_LIMIT: u64 = 512 * 1024 * 1024;
const CACHE_RESPONSE_LIMIT: u64 = 24 * 1024 * 1024;
const CATALOG_MAX_AGE_MS: i64 = 24 * 60 * 60 * 1000;
const CACHE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontsourceVariantFile {
    pub variant_id: String,
    pub weight: u16,
    pub is_italic: bool,
    pub url: String,
    pub is_variable: bool,
}

#[derive(Debug, Clone)]
pub struct CloudFontsClient {
    client: Client,
}

impl CloudFontsClient {
    pub fn new() -> Result<Self, AppError> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(12))
            .timeout(Duration::from_secs(90))
            .user_agent(USER_AGENT)
            .redirect(Policy::custom(|attempt| {
                let target = attempt.url();
                let secure_chain = attempt
                    .previous()
                    .iter()
                    .all(|url| url.scheme() == "https" && is_redirect_host_allowed(url.host_str()));
                if target.scheme() == "https"
                    && is_redirect_host_allowed(target.host_str())
                    && secure_chain
                {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }))
            .build()
            .map_err(|error| AppError::Http {
                url: FONTSOURCE_CATALOG_URL.into(),
                reason: error.to_string(),
            })?;
        Ok(Self { client })
    }

    pub async fn fetch_catalog(
        &self,
        provider: RemoteFontProvider,
    ) -> Result<Vec<RemoteFontCatalogItem>, AppError> {
        match provider {
            RemoteFontProvider::FontsourceExtra => {
                let value = self.fetch_json(FONTSOURCE_CATALOG_URL).await?;
                parse_fontsource_catalog_value(&value)
            }
            RemoteFontProvider::Fontshare => {
                let value = self.fetch_json(FONTSHARE_CATALOG_URL).await?;
                parse_fontshare_catalog_value(&value)
            }
            RemoteFontProvider::NerdFonts => {
                let value = self.fetch_json(NERD_FONTS_RELEASE_URL).await?;
                parse_nerd_fonts_release_value(&value)
            }
            RemoteFontProvider::Google => Err(AppError::InvalidInput(
                "Google Fonts catalog remains on its existing client.".into(),
            )),
        }
    }

    pub async fn fetch_fontsource_details(
        &self,
        family_id: &str,
    ) -> Result<Vec<FontsourceVariantFile>, AppError> {
        validate_identifier(family_id, "Fontsource family ID")?;
        let mut url = Url::parse(FONTSOURCE_DETAILS_URL).map_err(|error| AppError::Http {
            url: FONTSOURCE_DETAILS_URL.into(),
            reason: error.to_string(),
        })?;
        url.path_segments_mut()
            .map_err(|_| AppError::InvalidInput("Invalid Fontsource details endpoint.".into()))?
            .push(family_id);
        let value = self.fetch_json(url.as_str()).await?;
        parse_fontsource_details_value(&value, family_id)
    }

    pub async fn download_fontsource_ttf(&self, url: &str) -> Result<Vec<u8>, AppError> {
        let url = validate_download_url(RemoteFontProvider::FontsourceExtra, url)?;
        self.fetch_bytes(url, FONT_RESPONSE_LIMIT).await
    }

    pub async fn download_archive(
        &self,
        provider: RemoteFontProvider,
        url: &str,
    ) -> Result<Vec<u8>, AppError> {
        if !matches!(
            provider,
            RemoteFontProvider::Fontshare | RemoteFontProvider::NerdFonts
        ) {
            return Err(AppError::InvalidInput(
                "ZIP archives are supported only for Fontshare and Nerd Fonts.".into(),
            ));
        }
        let url = validate_download_url(provider, url)?;
        self.fetch_bytes(url, ARCHIVE_RESPONSE_LIMIT).await
    }

    async fn fetch_json(&self, url: &str) -> Result<Value, AppError> {
        let parsed = Url::parse(url).map_err(|error| AppError::RemoteFormat {
            endpoint: url.into(),
            reason: error.to_string(),
        })?;
        if parsed.scheme() != "https" || !is_redirect_host_allowed(parsed.host_str()) {
            return Err(AppError::RemoteFormat {
                endpoint: url.into(),
                reason: "API URL is not an allowlisted HTTPS origin".into(),
            });
        }
        let response = self
            .client
            .get(parsed.clone())
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|error| AppError::Http {
                url: parsed.to_string(),
                reason: error.to_string(),
            })?;
        let response = require_success(response, parsed.as_str())?;
        let bytes = read_limited(response, parsed.as_str(), JSON_RESPONSE_LIMIT).await?;
        serde_json::from_slice(&bytes).map_err(|error| AppError::RemoteFormat {
            endpoint: parsed.to_string(),
            reason: error.to_string(),
        })
    }

    async fn fetch_bytes(&self, url: Url, limit: u64) -> Result<Vec<u8>, AppError> {
        let response =
            self.client
                .get(url.clone())
                .send()
                .await
                .map_err(|error| AppError::Http {
                    url: url.to_string(),
                    reason: error.to_string(),
                })?;
        let response = require_success(response, url.as_str())?;
        read_limited(response, url.as_str(), limit).await
    }
}

#[derive(Debug, Clone)]
pub struct CloudCatalogCache {
    directory: PathBuf,
}

impl CloudCatalogCache {
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    pub fn load(
        &self,
        provider: RemoteFontProvider,
    ) -> Result<Option<RemoteFontCatalogSnapshot>, AppError> {
        let path = self.path_for(provider);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(storage_error(&path, error)),
        };
        if bytes.len() as u64 > CACHE_RESPONSE_LIMIT {
            return Err(AppError::DownloadTooLarge {
                url: path.display().to_string(),
                limit_bytes: CACHE_RESPONSE_LIMIT,
            });
        }
        let cached: CachedRemoteCatalog =
            serde_json::from_slice(&bytes).map_err(|error| AppError::RemoteFormat {
                endpoint: path.display().to_string(),
                reason: error.to_string(),
            })?;
        if cached.schema_version != CACHE_SCHEMA_VERSION || cached.provider != provider {
            return Err(AppError::RemoteFormat {
                endpoint: path.display().to_string(),
                reason: "unsupported cloud catalog cache schema or provider".into(),
            });
        }
        if cached.items.iter().any(|item| item.provider != provider) {
            return Err(AppError::RemoteFormat {
                endpoint: path.display().to_string(),
                reason: "cached catalog contains an item from another provider".into(),
            });
        }
        Ok(Some(RemoteFontCatalogSnapshot {
            provider,
            items: cached.items,
            fetched_at_unix_ms: cached.fetched_at_unix_ms,
            stale: now_ms().saturating_sub(cached.fetched_at_unix_ms) >= CATALOG_MAX_AGE_MS,
            last_error: None,
        }))
    }

    pub fn store(&self, snapshot: &RemoteFontCatalogSnapshot) -> Result<(), AppError> {
        if snapshot
            .items
            .iter()
            .any(|item| item.provider != snapshot.provider)
        {
            return Err(AppError::InvalidInput(
                "Cloud catalog item provider does not match its snapshot.".into(),
            ));
        }
        let cached = CachedRemoteCatalog {
            schema_version: CACHE_SCHEMA_VERSION,
            provider: snapshot.provider,
            fetched_at_unix_ms: snapshot.fetched_at_unix_ms,
            items: snapshot.items.clone(),
        };
        let bytes = serde_json::to_vec(&cached).map_err(|error| AppError::RemoteFormat {
            endpoint: self.path_for(snapshot.provider).display().to_string(),
            reason: error.to_string(),
        })?;
        let path = self.path_for(snapshot.provider);
        atomic_write(&path, &bytes)
    }

    fn path_for(&self, provider: RemoteFontProvider) -> PathBuf {
        self.directory
            .join(format!("{}.json", provider_cache_id(provider)))
    }
}

#[derive(Debug, Clone)]
pub struct CloudFontsService {
    client: CloudFontsClient,
    cache: CloudCatalogCache,
}

impl CloudFontsService {
    pub fn new(cache_directory: impl Into<PathBuf>) -> Result<Self, AppError> {
        Ok(Self {
            client: CloudFontsClient::new()?,
            cache: CloudCatalogCache::new(cache_directory),
        })
    }

    pub fn load_cached_catalog(
        &self,
        provider: RemoteFontProvider,
    ) -> Result<Option<RemoteFontCatalogSnapshot>, AppError> {
        self.cache.load(provider)
    }

    pub fn client(&self) -> &CloudFontsClient {
        &self.client
    }

    pub async fn refresh_catalog(
        &self,
        provider: RemoteFontProvider,
    ) -> Result<RemoteFontCatalogSnapshot, AppError> {
        match self.client.fetch_catalog(provider).await {
            Ok(items) => {
                let snapshot = RemoteFontCatalogSnapshot {
                    provider,
                    items,
                    fetched_at_unix_ms: now_ms(),
                    stale: false,
                    last_error: None,
                };
                self.cache.store(&snapshot)?;
                Ok(snapshot)
            }
            Err(error) => match self.cache.load(provider)? {
                Some(mut cached) => {
                    cached.stale = true;
                    cached.last_error = Some(error.to_string());
                    Ok(cached)
                }
                None => Err(error),
            },
        }
    }
}

pub fn parse_fontsource_catalog(bytes: &[u8]) -> Result<Vec<RemoteFontCatalogItem>, AppError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| AppError::RemoteFormat {
        endpoint: FONTSOURCE_CATALOG_URL.into(),
        reason: error.to_string(),
    })?;
    parse_fontsource_catalog_value(&value)
}

fn parse_fontsource_catalog_value(value: &Value) -> Result<Vec<RemoteFontCatalogItem>, AppError> {
    let entries = value
        .as_array()
        .ok_or_else(|| remote_format(FONTSOURCE_CATALOG_URL, "catalog root must be an array"))?;
    let mut items = Vec::new();
    for entry in entries {
        if entry.get("type").and_then(Value::as_str) != Some("other") {
            continue;
        }
        let id = required_identifier(entry.get("id"), FONTSOURCE_CATALOG_URL)?;
        let family = required_string(entry.get("family"), FONTSOURCE_CATALOG_URL, "family")?;
        let mut weights = entry
            .get("weights")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(parse_weight)
            .collect::<Vec<_>>();
        if weights.is_empty() {
            weights.push(400);
        }
        weights.sort_unstable();
        weights.dedup();

        let mut styles = entry
            .get("styles")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter(|style| matches!(*style, "normal" | "italic"))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if styles.is_empty() {
            styles.push("normal".into());
        }
        styles.dedup();

        let variants = weights
            .iter()
            .flat_map(|weight| {
                styles.iter().map(move |style| RemoteFontVariant {
                    id: format!("{weight}-{style}"),
                    label: format!(
                        "{weight} {}",
                        if style == "italic" {
                            "italic"
                        } else {
                            "normal"
                        }
                    ),
                    weight: *weight,
                    is_italic: style == "italic",
                })
            })
            .collect::<Vec<_>>();
        let default_weight = if weights.contains(&400) {
            400
        } else {
            weights[0]
        };
        let default_style = if styles.iter().any(|style| style == "normal") {
            "normal"
        } else {
            styles[0].as_str()
        };
        items.push(RemoteFontCatalogItem {
            id,
            provider: RemoteFontProvider::FontsourceExtra,
            family,
            category: category_from_string(entry.get("category").and_then(Value::as_str)),
            variants,
            preview_variant_id: Some(format!("{default_weight}-{default_style}")),
            download_strategy: FontDownloadStrategy::DirectFiles,
            download_url: None,
        });
    }
    Ok(items)
}

pub fn parse_fontsource_details(
    bytes: &[u8],
    family_id: &str,
) -> Result<Vec<FontsourceVariantFile>, AppError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| AppError::RemoteFormat {
        endpoint: format!("{FONTSOURCE_DETAILS_URL}/{family_id}"),
        reason: error.to_string(),
    })?;
    parse_fontsource_details_value(&value, family_id)
}

fn parse_fontsource_details_value(
    value: &Value,
    family_id: &str,
) -> Result<Vec<FontsourceVariantFile>, AppError> {
    validate_identifier(family_id, "Fontsource family ID")?;
    let endpoint = format!("{FONTSOURCE_DETAILS_URL}/{family_id}");
    if value.get("id").and_then(Value::as_str) != Some(family_id) {
        return Err(remote_format(
            &endpoint,
            "family ID does not match the request",
        ));
    }

    let variants = value
        .get("variants")
        .and_then(Value::as_object)
        .ok_or_else(|| remote_format(&endpoint, "variants must be an object"))?;
    let default_subset = value.get("defSubset").and_then(Value::as_str);
    let mut files = Vec::new();
    // A family's variable flag does not make its weight-specific TTF URLs variable.
    // The scanner checks fvar in the actual downloaded file; preserve every supplied style.
    for (weight_key, styles) in variants {
        let Ok(weight) = weight_key.parse::<u16>() else {
            continue;
        };
        if !(1..=1000).contains(&weight) {
            continue;
        }
        let Some(styles) = styles.as_object() else {
            continue;
        };
        for (style, subsets) in styles {
            if !matches!(style.as_str(), "normal" | "italic") {
                continue;
            }
            let Some(subsets) = subsets.as_object() else {
                continue;
            };
            let chosen_subset = match default_subset {
                Some(subset) => subsets.get(subset),
                None => subsets.values().next(),
            };
            let Some(ttf) = chosen_subset
                .and_then(|subset| subset.get("url"))
                .and_then(|url| url.get("ttf"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            let url = validate_download_url(RemoteFontProvider::FontsourceExtra, ttf)?;
            files.push(FontsourceVariantFile {
                variant_id: format!("{weight}-{style}"),
                weight,
                is_italic: style == "italic",
                url: url.to_string(),
                is_variable: false,
            });
        }
    }
    files.sort_by_key(|file| (file.weight, file.is_italic));
    if files.is_empty() {
        return Err(remote_format(&endpoint, "no valid TTF variants were found"));
    }
    Ok(files)
}

pub fn parse_fontshare_catalog(bytes: &[u8]) -> Result<Vec<RemoteFontCatalogItem>, AppError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| AppError::RemoteFormat {
        endpoint: FONTSHARE_CATALOG_URL.into(),
        reason: error.to_string(),
    })?;
    parse_fontshare_catalog_value(&value)
}

fn parse_fontshare_catalog_value(value: &Value) -> Result<Vec<RemoteFontCatalogItem>, AppError> {
    let root = value
        .get("fonts")
        .or_else(|| value.get("response"))
        .and_then(Value::as_array)
        .ok_or_else(|| remote_format(FONTSHARE_CATALOG_URL, "fonts/response must be an array"))?;
    let mut items = Vec::new();
    for entry in root {
        let slug = required_identifier(entry.get("slug"), FONTSHARE_CATALOG_URL)?;
        let family = entry
            .get("name")
            .or_else(|| entry.get("family"))
            .and_then(Value::as_str)
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| remote_format(FONTSHARE_CATALOG_URL, "font has no family name"))?
            .to_owned();
        let styles = entry
            .get("styles")
            .and_then(Value::as_array)
            .filter(|styles| !styles.is_empty())
            .ok_or_else(|| remote_format(FONTSHARE_CATALOG_URL, "font has no styles"))?;
        let mut variants = Vec::with_capacity(styles.len());
        let mut preview_candidates = Vec::with_capacity(styles.len());
        for (index, style) in styles.iter().enumerate() {
            let weight_object = style.get("weight");
            let weight = weight_object
                .and_then(|value| value.get("number").or_else(|| value.get("weight")))
                .and_then(parse_weight)
                .unwrap_or(400);
            let label = weight_object
                .and_then(|value| value.get("label"))
                .and_then(Value::as_str)
                .filter(|label| !label.trim().is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| weight_label(weight));
            let is_italic = style
                .get("is_italic")
                .and_then(Value::as_bool)
                .unwrap_or_else(|| label.to_ascii_lowercase().contains("italic"));
            let raw_id = style.get("id").and_then(Value::as_str).unwrap_or_default();
            let id = safe_variant_id(raw_id).unwrap_or_else(|| {
                format!(
                    "{weight}-{}-{index}",
                    if is_italic { "italic" } else { "normal" }
                )
            });
            variants.push(RemoteFontVariant {
                id: id.clone(),
                label,
                weight,
                is_italic,
            });
            let is_default = style
                .get("default")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            preview_candidates.push((id, weight, is_italic, is_default));
        }
        let preview = preview_candidates
            .iter()
            .find(|(_, weight, italic, is_default)| *weight == 400 && !*italic && *is_default)
            .or_else(|| {
                preview_candidates
                    .iter()
                    .find(|(_, weight, italic, _)| *weight == 400 && !*italic)
            })
            .or_else(|| preview_candidates.iter().find(|(_, _, italic, _)| !*italic))
            .or_else(|| preview_candidates.first());
        let download_url = fontshare_download_url(&slug)?;
        items.push(RemoteFontCatalogItem {
            id: slug,
            provider: RemoteFontProvider::Fontshare,
            family,
            category: FontCategory::Unknown,
            variants,
            preview_variant_id: preview.map(|(id, _, _, _)| id.clone()),
            download_strategy: FontDownloadStrategy::ZipArchive,
            download_url: Some(download_url.to_string()),
        });
    }
    Ok(items)
}

pub fn parse_nerd_fonts_release(bytes: &[u8]) -> Result<Vec<RemoteFontCatalogItem>, AppError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| AppError::RemoteFormat {
        endpoint: NERD_FONTS_RELEASE_URL.into(),
        reason: error.to_string(),
    })?;
    parse_nerd_fonts_release_value(&value)
}

fn parse_nerd_fonts_release_value(value: &Value) -> Result<Vec<RemoteFontCatalogItem>, AppError> {
    let assets = value
        .get("assets")
        .and_then(Value::as_array)
        .ok_or_else(|| remote_format(NERD_FONTS_RELEASE_URL, "assets must be an array"))?;
    let mut items = Vec::new();
    for asset in assets {
        let Some(name) = asset.get("name").and_then(Value::as_str) else {
            continue;
        };
        if !name.to_ascii_lowercase().ends_with(".zip")
            || name.to_ascii_lowercase().contains("fontpatcher")
        {
            continue;
        }
        if asset
            .get("size")
            .and_then(Value::as_u64)
            .is_some_and(|size| size > ARCHIVE_RESPONSE_LIMIT)
        {
            continue;
        }
        let Some(url) = asset.get("browser_download_url").and_then(Value::as_str) else {
            continue;
        };
        let Ok(url) = validate_download_url(RemoteFontProvider::NerdFonts, url) else {
            continue;
        };
        let stem = &name[..name.len() - 4];
        if validate_identifier(stem, "Nerd Fonts asset name").is_err() {
            continue;
        }
        items.push(RemoteFontCatalogItem {
            id: stem.to_owned(),
            provider: RemoteFontProvider::NerdFonts,
            family: stem.to_owned(),
            category: FontCategory::Monospace,
            variants: vec![RemoteFontVariant {
                id: "archive".into(),
                label: "ZIP archive".into(),
                weight: 400,
                is_italic: false,
            }],
            preview_variant_id: None,
            download_strategy: FontDownloadStrategy::ZipArchive,
            download_url: Some(url.to_string()),
        });
    }
    items.sort_by(|left, right| {
        left.family
            .to_ascii_lowercase()
            .cmp(&right.family.to_ascii_lowercase())
    });
    Ok(items)
}

pub fn map_google_catalog(items: &[GoogleFontCatalogItem]) -> Vec<RemoteFontCatalogItem> {
    items
        .iter()
        .map(|item| {
            let variants = item
                .variants
                .iter()
                .map(|id| {
                    let (weight, is_italic) = parse_google_variant(id);
                    RemoteFontVariant {
                        id: id.clone(),
                        label: google_variant_label(weight, is_italic),
                        weight,
                        is_italic,
                    }
                })
                .collect::<Vec<_>>();
            let preview_variant_id = if item
                .variants
                .iter()
                .any(|variant| variant == &item.default_variant)
            {
                Some(item.default_variant.clone())
            } else {
                item.variants.first().cloned()
            };
            RemoteFontCatalogItem {
                id: item.id.clone(),
                provider: RemoteFontProvider::Google,
                family: item.family.clone(),
                category: category_from_string(Some(&item.category)),
                variants,
                preview_variant_id,
                download_strategy: FontDownloadStrategy::DirectFiles,
                download_url: None,
            }
        })
        .collect()
}

pub fn validate_download_url(provider: RemoteFontProvider, value: &str) -> Result<Url, AppError> {
    let value = if value.starts_with("//") {
        format!("https:{value}")
    } else {
        value.to_owned()
    };
    let url = Url::parse(&value).map_err(|error| AppError::RemoteFormat {
        endpoint: value.clone(),
        reason: error.to_string(),
    })?;
    let host = url.host_str().unwrap_or_default();
    let host_allowed = match provider {
        RemoteFontProvider::Google => matches!(host, "gwfh.mranftl.com" | "fonts.gstatic.com"),
        RemoteFontProvider::FontsourceExtra => {
            matches!(host, "api.fontsource.org" | "cdn.jsdelivr.net")
        }
        RemoteFontProvider::Fontshare => matches!(host, "api.fontshare.com" | "cdn.fontshare.com"),
        RemoteFontProvider::NerdFonts => matches!(
            host,
            "github.com"
                | "objects.githubusercontent.com"
                | "release-assets.githubusercontent.com"
                | "github-releases.githubusercontent.com"
        ),
    };
    if url.scheme() != "https"
        || !host_allowed
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|port| port != 443)
        || url.fragment().is_some()
    {
        return Err(remote_format(
            url.as_str(),
            "URL must use HTTPS and an allowlisted provider host without credentials or fragments",
        ));
    }
    Ok(url)
}

fn fontshare_download_url(slug: &str) -> Result<Url, AppError> {
    validate_identifier(slug, "Fontshare slug")?;
    let url = format!("https://api.fontshare.com/v2/fonts/download/{slug}");
    validate_download_url(RemoteFontProvider::Fontshare, &url)
}

fn parse_google_variant(id: &str) -> (u16, bool) {
    let lower = id.to_ascii_lowercase();
    let is_italic = lower.ends_with("italic");
    let base = if is_italic {
        &lower[..lower.len() - "italic".len()]
    } else {
        lower.as_str()
    };
    let weight = match base.trim_end_matches('-') {
        "thin" => 100,
        "extralight" | "extra-light" => 200,
        "light" => 300,
        "regular" | "normal" | "" => 400,
        "medium" => 500,
        "semibold" | "semi-bold" => 600,
        "bold" => 700,
        "extrabold" | "extra-bold" => 800,
        "black" => 900,
        numeric => numeric
            .parse::<u16>()
            .ok()
            .filter(|weight| (1..=1000).contains(weight))
            .unwrap_or(400),
    };
    (weight, is_italic)
}

fn google_variant_label(weight: u16, is_italic: bool) -> String {
    let mut label = weight_label(weight);
    if is_italic {
        label.push_str(" Italic");
    }
    label
}

fn weight_label(weight: u16) -> String {
    match weight {
        100 => "Thin".into(),
        200 => "Extra Light".into(),
        300 => "Light".into(),
        400 => "Regular".into(),
        500 => "Medium".into(),
        600 => "Semi Bold".into(),
        700 => "Bold".into(),
        800 => "Extra Bold".into(),
        900 => "Black".into(),
        _ => weight.to_string(),
    }
}

fn category_from_string(value: Option<&str>) -> FontCategory {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "sans-serif" | "sans serif" => FontCategory::SansSerif,
        "serif" => FontCategory::Serif,
        "display" => FontCategory::Display,
        "handwriting" => FontCategory::Handwriting,
        "monospace" | "mono" => FontCategory::Monospace,
        _ => FontCategory::Unknown,
    }
}

fn parse_weight(value: &Value) -> Option<u16> {
    let weight = value
        .as_u64()
        .and_then(|value| u16::try_from(value).ok())
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))?;
    (1..=1000).contains(&weight).then_some(weight)
}

fn required_identifier(value: Option<&Value>, endpoint: &str) -> Result<String, AppError> {
    let value = value
        .and_then(Value::as_str)
        .ok_or_else(|| remote_format(endpoint, "font has no string identifier"))?;
    validate_identifier(value, "remote font identifier")?;
    Ok(value.to_owned())
}

fn required_string(value: Option<&Value>, endpoint: &str, field: &str) -> Result<String, AppError> {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| remote_format(endpoint, &format!("font has no valid {field}")))
}

fn validate_identifier(value: &str, label: &str) -> Result<(), AppError> {
    if value.is_empty()
        || value.len() > 128
        || value == "."
        || value == ".."
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(AppError::InvalidInput(format!("Invalid {label}.")));
    }
    Ok(())
}

fn safe_variant_id(value: &str) -> Option<String> {
    if !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        Some(value.to_owned())
    } else {
        None
    }
}

fn is_redirect_host_allowed(host: Option<&str>) -> bool {
    matches!(
        host,
        Some(
            "api.fontsource.org"
                | "cdn.jsdelivr.net"
                | "api.fontshare.com"
                | "cdn.fontshare.com"
                | "api.github.com"
                | "github.com"
                | "objects.githubusercontent.com"
                | "release-assets.githubusercontent.com"
                | "github-releases.githubusercontent.com"
                | "gwfh.mranftl.com"
                | "fonts.gstatic.com"
        )
    )
}

fn remote_format(endpoint: &str, reason: &str) -> AppError {
    AppError::RemoteFormat {
        endpoint: endpoint.to_owned(),
        reason: reason.to_owned(),
    }
}

fn require_success(response: Response, url: &str) -> Result<Response, AppError> {
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(AppError::HttpStatus {
            url: url.to_owned(),
            status: response.status().as_u16(),
        })
    }
}

async fn read_limited(response: Response, url: &str, limit: u64) -> Result<Vec<u8>, AppError> {
    if response
        .content_length()
        .is_some_and(|length| length > limit)
    {
        return Err(AppError::DownloadTooLarge {
            url: url.to_owned(),
            limit_bytes: limit,
        });
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| AppError::Http {
            url: url.to_owned(),
            reason: error.to_string(),
        })?;
        if bytes.len().saturating_add(chunk.len()) as u64 > limit {
            return Err(AppError::DownloadTooLarge {
                url: url.to_owned(),
                limit_bytes: limit,
            });
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn provider_cache_id(provider: RemoteFontProvider) -> &'static str {
    match provider {
        RemoteFontProvider::Google => "google",
        RemoteFontProvider::FontsourceExtra => "fontsource-extra",
        RemoteFontProvider::Fontshare => "fontshare",
        RemoteFontProvider::NerdFonts => "nerd-fonts",
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(|| AppError::Storage {
        path: path.to_path_buf(),
        reason: "cache file has no parent directory".into(),
    })?;
    fs::create_dir_all(parent).map_err(|error| storage_error(parent, error))?;
    let mut temporary =
        NamedTempFile::new_in(parent).map_err(|error| storage_error(parent, error))?;
    temporary
        .write_all(bytes)
        .map_err(|error| storage_error(path, error))?;
    temporary
        .persist(path)
        .map_err(|error| storage_error(path, error.error))?;
    Ok(())
}

fn storage_error(path: &Path, error: std::io::Error) -> AppError {
    AppError::Storage {
        path: path.to_path_buf(),
        reason: error.to_string(),
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or_default()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CachedRemoteCatalog {
    schema_version: u32,
    provider: RemoteFontProvider,
    fetched_at_unix_ms: i64,
    items: Vec<RemoteFontCatalogItem>,
}
