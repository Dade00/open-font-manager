use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;
const MAX_PIXELS: u64 = 24_000_000;
const MAX_BOARDS: usize = 500;
const MAX_COMPOSITIONS: usize = 500;
const MAX_ITEMS: usize = 5_000;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspirationDocument {
    pub version: u8,
    pub revision: u64,
    pub boards: Vec<Board>,
    pub compositions: Vec<Composition>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspirationFontRef {
    pub provider: String,
    pub family_id: Option<String>,
    pub variant_id: Option<String>,
    pub local_font_id: Option<String>,
    pub family: String,
    pub is_variable: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionRole {
    pub font: Option<InspirationFontRef>,
    pub text: String,
    pub size: f64,
    pub line_height: f64,
    pub letter_spacing: f64,
    pub color: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Composition {
    pub id: String,
    pub title: String,
    pub heading: CompositionRole,
    pub subheading: Option<CompositionRole>,
    pub body: CompositionRole,
    pub background: String,
    pub narrow: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspirationAsset {
    pub id: String,
    pub mime: String,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum BoardItem {
    Note {
        id: String,
        title: String,
        text: String,
    },
    Link {
        id: String,
        title: String,
        text: String,
        url: String,
    },
    Image {
        id: String,
        title: String,
        text: String,
        asset: InspirationAsset,
    },
    Font {
        id: String,
        title: String,
        font: InspirationFontRef,
    },
    Composition {
        id: String,
        title: String,
        #[serde(rename = "compositionId")]
        composition_id: String,
    },
}
impl BoardItem {
    fn id(&self) -> &str {
        match self {
            Self::Note { id, .. }
            | Self::Link { id, .. }
            | Self::Image { id, .. }
            | Self::Font { id, .. }
            | Self::Composition { id, .. } => id,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub id: String,
    pub title: String,
    pub description: String,
    pub items: Vec<BoardItem>,
}

fn invalid(message: &str) -> String {
    message.to_owned()
}
fn text_ok(value: &str, max: usize, label: &str) -> Result<(), String> {
    if value.chars().count() > max || value.contains('\0') {
        Err(format!("{label} è troppo lungo o non valido."))
    } else {
        Ok(())
    }
}
fn font_ok(font: &InspirationFontRef) -> Result<(), String> {
    if !matches!(
        font.provider.as_str(),
        "local" | "google" | "fontsource-extra" | "fontshare" | "nerd-fonts"
    ) {
        return Err(invalid("Provider font non valido."));
    }
    text_ok(&font.family, 300, "Nome famiglia")?;
    for value in [&font.family_id, &font.variant_id, &font.local_font_id]
        .into_iter()
        .flatten()
    {
        text_ok(value, 500, "Riferimento font")?;
    }
    match font.provider.as_str() {
        "local" => {
            if font.local_font_id.as_deref().unwrap_or("").is_empty() {
                return Err(invalid("Riferimento font locale mancante."));
            }
        }
        _ => {
            if font.family_id.as_deref().unwrap_or("").is_empty() {
                return Err(invalid("ID famiglia remoto mancante."));
            }
        }
    }
    Ok(())
}
pub fn validate_document_font(font: &InspirationFontRef) -> Result<(), String> {
    font_ok(font)
}
fn role_ok(role: &CompositionRole) -> Result<(), String> {
    text_ok(&role.text, 20_000, "Testo ruolo")?;
    text_ok(&role.color, 64, "Colore")?;
    if !role.size.is_finite()
        || !(1.0..=500.0).contains(&role.size)
        || !role.line_height.is_finite()
        || !(0.5..=5.0).contains(&role.line_height)
        || !role.letter_spacing.is_finite()
        || !(-20.0..=100.0).contains(&role.letter_spacing)
    {
        return Err(invalid("Parametri tipografici fuori limite."));
    }
    if let Some(font) = &role.font {
        font_ok(font)?;
    }
    Ok(())
}
fn validate(document: &InspirationDocument) -> Result<(), String> {
    if document.version != 1 {
        return Err(invalid("Versione documento Ispirazione non supportata."));
    }
    if document.revision >= i64::MAX as u64 {
        return Err(invalid("Revisione non valida."));
    }
    if document.boards.len() > MAX_BOARDS || document.compositions.len() > MAX_COMPOSITIONS {
        return Err(invalid("Il documento contiene troppi elementi."));
    }
    let mut ids = HashSet::new();
    for composition in &document.compositions {
        if !ids.insert(composition.id.clone()) {
            return Err(invalid("ID duplicato nel documento."));
        }
        text_ok(&composition.id, 128, "ID composizione")?;
        text_ok(&composition.title, 300, "Titolo composizione")?;
        text_ok(&composition.background, 64, "Sfondo")?;
        role_ok(&composition.heading)?;
        role_ok(&composition.body)?;
        if let Some(role) = &composition.subheading {
            role_ok(role)?;
        }
    }
    let compositions: HashSet<_> = document
        .compositions
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    let mut count = 0;
    for board in &document.boards {
        if !ids.insert(board.id.clone()) {
            return Err(invalid("ID duplicato nel documento."));
        }
        text_ok(&board.id, 128, "ID board")?;
        text_ok(&board.title, 300, "Titolo board")?;
        text_ok(&board.description, 10_000, "Descrizione board")?;
        count += board.items.len();
        if count > MAX_ITEMS {
            return Err(invalid("La board contiene troppi elementi."));
        }
        for item in &board.items {
            if !ids.insert(item.id().to_owned()) {
                return Err(invalid("ID duplicato nel documento."));
            }
            text_ok(item.id(), 128, "ID elemento")?;
            match item {
                BoardItem::Note { title, text, .. } => {
                    text_ok(title, 300, "Titolo")?;
                    text_ok(text, 20_000, "Nota")?
                }
                BoardItem::Link {
                    title, text, url, ..
                } => {
                    text_ok(title, 300, "Titolo")?;
                    text_ok(text, 20_000, "Nota")?;
                    validate_link(url)?;
                }
                BoardItem::Image {
                    title, text, asset, ..
                } => {
                    text_ok(title, 300, "Titolo")?;
                    text_ok(text, 20_000, "Nota")?;
                    asset_path(Path::new("."), &asset.id)?;
                    if !matches!(
                        asset.mime.as_str(),
                        "image/png" | "image/jpeg" | "image/webp"
                    ) || asset.width == 0
                        || asset.height == 0
                    {
                        return Err(invalid("Metadati immagine non validi."));
                    }
                }
                BoardItem::Font { title, font, .. } => {
                    text_ok(title, 300, "Titolo")?;
                    font_ok(font)?;
                }
                BoardItem::Composition {
                    title,
                    composition_id,
                    ..
                } => {
                    text_ok(title, 300, "Titolo")?;
                    if !compositions.contains(composition_id.as_str()) {
                        return Err(invalid("Riferimento a composizione inesistente."));
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn validate_link(url: &str) -> Result<(), String> {
    let parsed =
        url::Url::parse(url).map_err(|_| invalid("Il collegamento non è un URL valido."))?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(invalid(
            "Sono consentiti solo link HTTP o HTTPS senza credenziali.",
        ));
    }
    Ok(())
}

fn init(path: &Path) -> Result<(), String> {
    let c = crate::db::open_connection(path).map_err(|e| e.user_message())?;
    c.execute_batch("CREATE TABLE IF NOT EXISTS inspiration_document(id INTEGER PRIMARY KEY CHECK(id=1),revision INTEGER NOT NULL CHECK(revision>=0),snapshot_json TEXT NOT NULL); CREATE TABLE IF NOT EXISTS inspiration_assets(id TEXT PRIMARY KEY CHECK(length(id)=64),mime TEXT NOT NULL,width INTEGER NOT NULL,height INTEGER NOT NULL,relative_path TEXT NOT NULL);").map_err(|e|format!("Database Ispirazione non disponibile: {e}"))?;
    Ok(())
}
pub fn load(database: &Path) -> Result<InspirationDocument, String> {
    init(database)?;
    let c = crate::db::open_connection(database).map_err(|e| e.user_message())?;
    let row: Option<(i64, String)> = c
        .query_row(
            "SELECT revision,snapshot_json FROM inspiration_document WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some((revision, json)) = row {
        let mut doc: InspirationDocument = serde_json::from_str(&json)
            .map_err(|_| invalid("Documento Ispirazione salvato non valido."))?;
        doc.revision = u64::try_from(revision).map_err(|_| invalid("Revisione non valida."))?;
        validate(&doc)?;
        Ok(doc)
    } else {
        Ok(InspirationDocument {
            version: 1,
            revision: 0,
            boards: vec![],
            compositions: vec![],
        })
    }
}
pub fn save(
    database: &Path,
    _data_dir: &Path,
    mut document: InspirationDocument,
    expected_revision: u64,
) -> Result<InspirationDocument, String> {
    if expected_revision >= i64::MAX as u64 {
        return Err(invalid("Revisione non valida."));
    }
    document.revision = expected_revision;
    validate(&document)?;
    init(database)?;
    let mut c = crate::db::open_connection(database).map_err(|e| e.user_message())?;
    let tx = c
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let current: Option<i64> = tx
        .query_row(
            "SELECT revision FROM inspiration_document WHERE id=1",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let actual = current.unwrap_or(0);
    if actual != expected_revision as i64 {
        return Err(format!("Conflitto di salvataggio: revisione attesa {expected_revision}, corrente {actual}. Ricarica prima di salvare."));
    }
    validate_references(&tx, &document)?;
    document.revision = expected_revision + 1;
    let json = serde_json::to_string(&document).map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO inspiration_document(id,revision,snapshot_json) VALUES(1,?1,?2) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,snapshot_json=excluded.snapshot_json",params![document.revision as i64,json]).map_err(|e|e.to_string())?;
    // Keep unreferenced imports: a snapshot save must not race pending imports/other clients.
    tx.commit().map_err(|e| e.to_string())?;
    Ok(document)
}
fn validate_references(
    tx: &rusqlite::Transaction<'_>,
    document: &InspirationDocument,
) -> Result<(), String> {
    for item in document.boards.iter().flat_map(|b| &b.items) {
        if let BoardItem::Image { asset, .. } = item {
            let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM inspiration_assets WHERE id=?1 AND mime=?2 AND width=?3 AND height=?4)",params![asset.id,asset.mime,asset.width,asset.height],|r|r.get(0)).map_err(|e|e.to_string())?;
            if !exists {
                return Err(invalid("Immagine non importata nell’archivio Ispirazione."));
            }
        }
    }
    Ok(())
}

pub fn asset_path(data_dir: &Path, id: &str) -> Result<PathBuf, String> {
    if id.len() != 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(invalid("ID immagine non valido."));
    }
    Ok(data_dir
        .join("inspiration-assets")
        .join(format!("{id}.image")))
}
pub fn get_asset(data_dir: &Path, id: &str) -> Result<Vec<u8>, String> {
    let path = asset_path(data_dir, id)?;
    use std::io::Read;
    let file = fs::File::open(path).map_err(|_| invalid("Immagine non disponibile."))?;
    let mut bytes = Vec::new();
    file.take(MAX_IMAGE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid("Immagine non disponibile."))?;
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        return Err(invalid("Immagine superiore a 20 MiB."));
    }
    Ok(bytes)
}
pub fn import_image(
    database: &Path,
    data_dir: &Path,
    source: &Path,
) -> Result<InspirationAsset, String> {
    let metadata = fs::metadata(source)
        .map_err(|_| invalid("Impossibile leggere il file immagine selezionato."))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_IMAGE_BYTES {
        return Err(invalid("Immagine vuota o superiore a 20 MiB."));
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(source)
        .map_err(|_| invalid("Impossibile leggere il file immagine selezionato."))?
        .take(MAX_IMAGE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid("Impossibile leggere il file immagine selezionato."))?;
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        return Err(invalid("Immagine superiore a 20 MiB."));
    }
    use image::io::Reader as ImageReader;
    let reader = ImageReader::new(std::io::Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|_| invalid("Formato immagine non supportato."))?;
    let format = reader
        .format()
        .ok_or_else(|| invalid("Formato immagine non supportato."))?;
    let mime = match format {
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::WebP => "image/webp",
        _ => return Err(invalid("Sono supportate solo immagini PNG, JPEG e WebP.")),
    };
    let (width, height) = reader
        .into_dimensions()
        .map_err(|_| invalid("Immagine corrotta o non valida."))?;
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or_else(|| invalid("Dimensioni immagine non valide."))?;
    if width == 0 || height == 0 || pixels > MAX_PIXELS {
        return Err(invalid(
            "L’immagine supera il limite di 24 milioni di pixel.",
        ));
    }
    let mut decoder = ImageReader::new(std::io::Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|_| invalid("Formato immagine non supportato."))?;
    let mut limits = image::io::Limits::default();
    limits.max_image_width = Some(u32::MAX);
    limits.max_image_height = Some(u32::MAX);
    limits.max_alloc = Some(MAX_PIXELS * 16);
    decoder.limits(limits);
    let decoded = decoder
        .decode()
        .map_err(|_| invalid("Immagine corrotta o non valida."))?;
    if decoded.width() != width || decoded.height() != height {
        return Err(invalid("Dimensioni immagine non valide."));
    }
    let id = format!("{:x}", Sha256::digest(&bytes));
    let asset = InspirationAsset {
        id: id.clone(),
        mime: mime.into(),
        width,
        height,
    };
    let final_path = asset_path(data_dir, &id)?;
    fs::create_dir_all(final_path.parent().unwrap())
        .map_err(|_| invalid("Archivio immagini non disponibile."))?;
    if !final_path.exists() {
        let temp = final_path.with_extension(format!(
            "tmp-{}",
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&temp, &bytes)
            .map_err(|_| invalid("Impossibile scrivere l’immagine importata."))?;
        if fs::rename(&temp, &final_path).is_err() {
            let _ = fs::remove_file(&temp);
            if !final_path.exists() {
                return Err(invalid("Impossibile finalizzare l’immagine importata."));
            }
        }
    }
    init(database)?;
    let c = crate::db::open_connection(database).map_err(|e| e.user_message())?;
    let relative = Path::new("inspiration-assets")
        .join(format!("{id}.image"))
        .to_string_lossy()
        .replace('\\', "/");
    c.execute("INSERT INTO inspiration_assets(id,mime,width,height,relative_path) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO NOTHING",params![asset.id,asset.mime,asset.width,asset.height,relative]).map_err(|e|e.to_string())?;
    Ok(asset)
}
