use std::time::Duration;

use futures::StreamExt;
use reqwest::{header, redirect::Policy, Client, Response, Url};

use crate::{
    errors::AppError,
    models::{GoogleFontCatalogItem, GoogleFontDetails},
};

const CATALOG_URL: &str = "https://gwfh.mranftl.com/api/fonts";
const CATALOG_LIMIT: u64 = 10 * 1024 * 1024;
const DETAIL_LIMIT: u64 = 10 * 1024 * 1024;
const FONT_LIMIT: u64 = 25 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogResponse {
    NotModified,
    Updated {
        items: Vec<GoogleFontCatalogItem>,
        etag: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct GoogleFontsClient {
    client: Client,
}

impl GoogleFontsClient {
    pub fn new() -> Result<Self, AppError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .redirect(Policy::custom(|attempt| {
                let from_https = attempt
                    .previous()
                    .last()
                    .is_some_and(|url| url.scheme() == "https");
                if !from_https || attempt.url().scheme() == "https" {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }))
            .build()
            .map_err(|error| AppError::Http {
                url: CATALOG_URL.into(),
                reason: error.to_string(),
            })?;
        Ok(Self { client })
    }

    pub async fn fetch_catalog(&self, etag: Option<&str>) -> Result<CatalogResponse, AppError> {
        let mut request = self.client.get(CATALOG_URL);
        if let Some(etag) = etag {
            request = request.header(header::IF_NONE_MATCH, etag);
        }
        let response = request.send().await.map_err(|error| AppError::Http {
            url: CATALOG_URL.into(),
            reason: error.to_string(),
        })?;
        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            return Ok(CatalogResponse::NotModified);
        }
        let response = require_success(response, CATALOG_URL)?;
        let response_etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let bytes = read_limited(response, CATALOG_URL, CATALOG_LIMIT).await?;
        let items =
            serde_json::from_slice::<Vec<GoogleFontCatalogItem>>(&bytes).map_err(|error| {
                AppError::RemoteFormat {
                    endpoint: CATALOG_URL.into(),
                    reason: error.to_string(),
                }
            })?;
        if items
            .iter()
            .any(|item| item.id.trim().is_empty() || item.family.trim().is_empty())
        {
            return Err(AppError::RemoteFormat {
                endpoint: CATALOG_URL.into(),
                reason: "catalog contains an item without an ID or family name".into(),
            });
        }
        Ok(CatalogResponse::Updated {
            items,
            etag: response_etag,
        })
    }

    pub async fn fetch_details(&self, family_id: &str) -> Result<GoogleFontDetails, AppError> {
        let url = endpoint_url(family_id)?;
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
        let bytes = read_limited(response, url.as_str(), DETAIL_LIMIT).await?;
        let details = serde_json::from_slice::<GoogleFontDetails>(&bytes).map_err(|error| {
            AppError::RemoteFormat {
                endpoint: url.to_string(),
                reason: error.to_string(),
            }
        })?;
        if details.id != family_id {
            return Err(AppError::RemoteFormat {
                endpoint: url.to_string(),
                reason: "family ID in response does not match the request".into(),
            });
        }
        Ok(details)
    }

    pub async fn download_ttf(&self, url: &str) -> Result<Vec<u8>, AppError> {
        let parsed = Url::parse(url).map_err(|error| AppError::RemoteFormat {
            endpoint: url.into(),
            reason: error.to_string(),
        })?;
        if parsed.scheme() != "https"
            || parsed.host_str() != Some("fonts.gstatic.com")
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err(AppError::RemoteFormat {
                endpoint: url.into(),
                reason: "font URL is not an HTTPS fonts.gstatic.com address".into(),
            });
        }
        let response = self
            .client
            .get(parsed.clone())
            .send()
            .await
            .map_err(|error| AppError::Http {
                url: parsed.to_string(),
                reason: error.to_string(),
            })?;
        let response = require_success(response, parsed.as_str())?;
        read_limited(response, parsed.as_str(), FONT_LIMIT).await
    }
}

fn endpoint_url(family_id: &str) -> Result<Url, AppError> {
    if family_id.trim().is_empty() || family_id.contains('/') || family_id.contains('\\') {
        return Err(AppError::InvalidInput(
            "Invalid Google Fonts family ID.".into(),
        ));
    }
    let mut url = Url::parse(CATALOG_URL).map_err(|error| AppError::Http {
        url: CATALOG_URL.into(),
        reason: error.to_string(),
    })?;
    url.path_segments_mut()
        .map_err(|_| AppError::InvalidInput("Google Fonts endpoint cannot accept a path.".into()))?
        .push(family_id);
    Ok(url)
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
        let next_len = bytes.len().saturating_add(chunk.len());
        if next_len as u64 > limit {
            return Err(AppError::DownloadTooLarge {
                url: url.to_owned(),
                limit_bytes: limit,
            });
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
