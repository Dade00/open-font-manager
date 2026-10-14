use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Storage operation failed at {path}: {reason}")]
    Storage { path: PathBuf, reason: String },
    #[error("Database operation failed: {0}")]
    Database(String),
    #[error("Unable to parse font at {path}: {reason}")]
    FontParse { path: PathBuf, reason: String },
    #[error("Invalid font metadata at {path}: {reason}")]
    InvalidFontMetadata { path: PathBuf, reason: String },
    #[error("HTTP request failed for {url}: {reason}")]
    Http { url: String, reason: String },
    #[error("HTTP request failed for {url} with status {status}")]
    HttpStatus { url: String, status: u16 },
    #[error("Invalid remote response from {endpoint}: {reason}")]
    RemoteFormat { endpoint: String, reason: String },
    #[error("Download from {url} exceeded the {limit_bytes} byte limit")]
    DownloadTooLarge { url: String, limit_bytes: u64 },
    #[error("Could not load font into Windows GDI: {path}")]
    Win32FontLoad { path: PathBuf },
    #[error("Could not remove font from Windows GDI: {path}")]
    Win32FontRemove { path: PathBuf },
    #[error("Windows font notification failed: {0}")]
    Win32Broadcast(String),
    #[error("Clipboard operation failed: {0}")]
    Clipboard(String),
    #[error("This operation is available only on Windows")]
    UnsupportedPlatform,
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("The requested item was not found: {0}")]
    NotFound(String),
}

impl AppError {
    pub fn user_message(&self) -> String {
        match self {
            Self::Storage { .. } => "Impossibile accedere ai dati locali dell'applicazione.".into(),
            Self::Database(_) => "Impossibile leggere o aggiornare la libreria locale.".into(),
            Self::FontParse { .. } | Self::InvalidFontMetadata { .. } => {
                "Il file non contiene un font valido o leggibile.".into()
            }
            Self::Http { .. } | Self::HttpStatus { .. } => {
                "Servizio Google Fonts non raggiungibile. Riprova più tardi.".into()
            }
            Self::RemoteFormat { .. } => "Il servizio remoto ha restituito dati non validi.".into(),
            Self::DownloadTooLarge { .. } => {
                "Il file scaricato supera il limite consentito.".into()
            }
            Self::Win32FontLoad { .. } => "Windows non è riuscito ad attivare il font.".into(),
            Self::Win32FontRemove { .. } => "Windows non è riuscito a disattivare il font.".into(),
            Self::Win32Broadcast(_) => {
                "Font aggiornato; alcune applicazioni potrebbero richiedere un riavvio.".into()
            }
            Self::Clipboard(_) => "Gli appunti di sistema non sono disponibili.".into(),
            Self::UnsupportedPlatform => "Questa funzione è disponibile solo su Windows.".into(),
            Self::InvalidInput(message) | Self::NotFound(message) => message.clone(),
        }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Database(value.to_string())
    }
}
