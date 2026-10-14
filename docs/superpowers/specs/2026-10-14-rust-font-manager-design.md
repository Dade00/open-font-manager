# RustFontManager — Specifica tecnica di progetto

> Documento normativo per implementatori. Ogni requisito marcato **MUST** è obbligatorio. Non introdurre WebView, Tauri, Electron, JavaScript, Node.js o `package.json`.

## 1. Obiettivo e confini

### 1.1 Prodotto

RustFontManager è un’applicazione desktop Windows 11 scritta in Rust nativo. Deve catalogare font locali, caricare/rimuovere font temporaneamente nella sessione Windows, consultare Google Fonts senza API key, scaricare font in un vault locale e organizzarli in collezioni.

L’artefatto finale è un eseguibile Windows `rust-font-manager.exe`. L’app può usare DLL fornite da Windows (`Gdi32.dll`, `User32.dll`, stack grafico GPU), ma non richiede runtime, database o servizi locali esterni.

### 1.2 Funzioni incluse nella prima versione

1. Selezione di una o più cartelle locali e scansione ricorsiva di `.ttf` e `.otf`.
2. Lettura di metadata OpenType senza installare il font durante la scansione.
3. Archivio SQLite locale di font, directory scansionate, preferiti e collezioni.
4. Caricamento/rimozione temporanei della sessione Windows mediante GDI.
5. Broadcast `WM_FONTCHANGE` senza bloccare il thread dell’interfaccia.
6. Ricerca, filtri, preview custom e paginazione da massimo 50 card.
7. Catalogo remoto Google Fonts tramite `google-webfonts-helper` con cache offline.
8. Caricamento della preview Google on demand.
9. Download di una singola variante Google, validazione, vault locale, registrazione, caricamento Iced e attivazione Windows in un’azione.
10. Download esplicito dell’intera famiglia Google come azione separata.
11. Preferiti, collezioni, copia CSS e snippet `next/font/google` negli appunti.
12. Ripristino degli elementi segnati attivi al lancio della nuova sessione Windows.

### 1.3 Non inclusi nella prima versione

- Installazione permanente nel Registro o in `C:\Windows\Fonts`.
- Modifica, conversione o creazione di file font.
- Sincronizzazione cloud o account utente.
- Supporto `.ttc`, `.woff`, `.woff2`, Type 1 e font bitmap. Tali file sono ignorati, con contatore nel report di scansione.
- Tray icon. Il crate `tray-icon` non deve essere incluso in v1: richiede un event loop Win32 sul thread proprietario e la sua integrazione con Iced/Winit va progettata e testata separatamente.
- Garanzia che qualunque applicazione terza aggiorni istantaneamente l’elenco font. L’app invia la notifica prevista da Windows; ogni applicazione gestisce il refresh autonomamente.

### 1.4 Vincoli non negoziabili

- Target: Windows 11 x86_64, toolchain `x86_64-pc-windows-msvc`.
- Rust edition 2021.
- GUI: Iced 0.13.x con renderer GPU predefinito WGPU.
- Nessun `unwrap()`, `expect()`, `todo!()`, `unimplemented!()` o panic deliberato nel codice di produzione.
- Tutti gli input filesystem, database, font binari e HTTP sono inaffidabili e devono generare `Result`/messaggi utente, non crash.
- Le operazioni che possono richiedere I/O non devono eseguirsi nel thread UI.
- Ogni query SQL con dati esterni deve usare parametri, mai interpolazione di stringhe.

---

## 2. Decisioni tecniche validate

| Area | Decisione | Motivazione |
|---|---|---|
| GUI | `iced = 0.13.1` | `iced::application`, `Task`, `iced::font::load` e `Font::with_name` sono disponibili e verificati. |
| Parser font | `skrifa` + crate Fontations necessario | `ttf-parser` è segnalato non mantenuto da RustSec (`RUSTSEC-2026-0192`) e non deve essere introdotto. |
| Database | `rusqlite 0.32.1` con `bundled` | SQLite incorporato, nessuna dipendenza di sistema. |
| HTTP | `reqwest 0.12` con Rustls | TLS portabile e prevedibile senza dipendere dalla configurazione TLS nativa. |
| Font di sessione | `windows 0.58` e GDI | API Win32 verificate per `AddFontResourceW`, `RemoveFontResourceW` e broadcast. |
| Directory app | `directories 5.0.1` | `data_local_dir()` corrisponde a `%LOCALAPPDATA%`. |
| Dialog | `rfd 0.15` | Dialog nativo asincrono per cartelle. |
| Clipboard | `arboard 3.4` | Clipboard testuale nativa. |

Le versioni devono essere bloccate nel `Cargo.lock`, generato e committato insieme al manifest.

---

## 3. Manifest Cargo obbligatorio

```toml
[package]
name = "rust-font-manager"
version = "0.1.0"
edition = "2021"
rust-version = "1.80"

[dependencies]
iced = { version = "0.13.1", features = ["tokio", "advanced", "lazy", "image"] }
tokio = { version = "1.40", features = ["rt-multi-thread", "macros", "fs", "sync", "time"] }
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "stream"] }
futures = "0.3"
walkdir = "2.5"
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
directories = "5.0"
arboard = "3.4"
rfd = "0.15"
sha2 = "0.10"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Versione compatibile e attivamente mantenuta da verificare al momento del lock.
skrifa = "0.30"

[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_Graphics_Gdi",
    "Win32_UI_WindowsAndMessaging",
] }

[dev-dependencies]
tempfile = "3"
```

Non aggiungere `tray-icon` in v1. Se una versione concreta di `skrifa` richiede API diverse, aggiornare questa specifica e il lockfile nella stessa modifica: non reintrodurre `ttf-parser` come scorciatoia.

---

## 4. Struttura del repository

```text
rust-font-manager/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── IDEA.md                         # brief originario, non sorgente normativa
├── docs/
│   └── superpowers/specs/
│       └── 2026-10-14-rust-font-manager-design.md
├── src/
│   ├── main.rs
│   ├── app.rs
│   ├── theme.rs
│   ├── models.rs
│   ├── errors.rs
│   ├── paths.rs
│   ├── db.rs
│   ├── font_names.rs
│   ├── font_scanner.rs
│   ├── win_font_loader.rs
│   ├── google_fonts.rs
│   ├── snippets.rs
│   └── services/
│       ├── mod.rs
│       ├── local_library.rs
│       └── google_library.rs
└── tests/
    ├── db_tests.rs
    ├── scanner_tests.rs
    ├── google_fonts_tests.rs
    ├── snippets_tests.rs
    └── fixtures/
        ├── valid-font.ttf
        └── invalid-font.ttf
```

Ogni modulo ha responsabilità unica:

- `main.rs`: logging, bootstrap, esecuzione Iced. Nessuna logica business.
- `app.rs`: stato UI, enum `Message`, `update`, `view`, task mapping.
- `theme.rs`: palette, style function Iced, costanti UI.
- `models.rs`: tipi serializzabili e DTO condivisi.
- `errors.rs`: errori di dominio (`AppError`) e conversioni non UI.
- `paths.rs`: risoluzione/creazione directory applicative.
- `db.rs`: schema, migrazioni, query, transazioni; nessun widget Iced.
- `font_names.rs`: interning stabile dei nomi di famiglia necessari a Iced.
- `font_scanner.rs`: enumerazione file e parsing OpenType sicuro.
- `win_font_loader.rs`: solo wrapper Windows; esposto con API pura e testabile dove possibile.
- `google_fonts.rs`: DTO API, cache, download, validazione e persistenza atomica.
- `snippets.rs`: generazione pura di CSS e Next.js.
- `services/*`: orchestration I/O invocata dai task Iced.

---

## 5. Directory e file locali

### 5.1 Radice

Usare:

```rust
ProjectDirs::from("com", "RustFontManager", "RustFontManager")
```

Se restituisce `None`, l’avvio fallisce con un messaggio GUI: `Impossibile determinare la directory dati locale di Windows.`

### 5.2 Percorsi obbligatori

| Nome logico | Percorso ottenuto | Uso |
|---|---|---|
| local data | `ProjectDirs::data_local_dir()` | database e vault persistente |
| cache | `ProjectDirs::cache_dir()` | catalogo e preview scaricate |
| database | `<local-data>/library.db` | SQLite |
| vault Google | `<local-data>/vault/google/<family-id>/` | TTF scaricati validati |
| catalog cache | `<cache>/gfonts_catalog.json` | ultimo catalogo valido |
| metadata cache | `<cache>/gfonts_catalog.meta.json` | ETag/fetched time/versione schema |
| preview cache | `<cache>/previews/<family-id>-<variant>.ttf` | TTF caricabili in Iced |
| log | `<local-data>/logs/rust-font-manager.log` | diagnostica rotabile |

Alla creazione dell’app devono essere create tutte le directory padre con `std::fs::create_dir_all`, propagando gli errori come `AppError::Storage`.

### 5.3 Scrittura atomica

Ogni JSON cache e ogni TTF scaricato deve seguire questa sequenza:

1. creare un file temporaneo nella stessa directory destinazione;
2. scrivere tutti i byte;
3. eseguire `sync_all()`;
4. validare il TTF dal file temporaneo;
5. rinominare nella destinazione finale;
6. se un file finale esiste, usare strategia di sostituzione Windows sicura oppure cancellare il temporaneo e mantenere il file validato esistente;
7. solo dopo il successo aggiornare SQLite.

Un download interrotto non deve mai produrre un record DB attivo o un file `.ttf` finale parziale.

---

## 6. Modelli e contratti di dominio

### 6.1 Enumerazioni

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontSource {
    Local,
    Google,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryTab {
    Local,
    Active,
    Favorites,
    Google,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontCategory {
    All,
    SansSerif,
    Serif,
    Display,
    Monospace,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontActivation {
    Active,
    Inactive,
}
```

`FontCategory::Unknown` è obbligatoria per font locali privi di classificazione affidabile. Non dedurre una categoria dal nome del file.

### 6.2 Font locale persistito

```rust
pub struct FontItem {
    pub id: String,
    pub family: String,
    pub subfamily: String,
    pub full_name: String,
    pub weight: u16,
    pub is_italic: bool,
    pub file_path: PathBuf,
    pub source: FontSource,
    pub source_family_id: Option<String>,
    pub source_variant_id: Option<String>,
    pub category: FontCategory,
    pub is_active: bool,
    pub is_favorite: bool,
    pub file_size: u64,
    pub file_modified_unix_ms: i64,
    pub missing_on_disk: bool,
}
```

`id` è un hash SHA-256 esadecimale dei byte UTF-8 di `canonical_path + "\n" + source_variant_id.unwrap_or("")`. Non usare `DefaultHasher` come chiave persistente.

### 6.3 Risultato scansione

```rust
pub struct ScanReport {
    pub root: PathBuf,
    pub discovered_candidates: usize,
    pub imported: usize,
    pub updated: usize,
    pub skipped_unchanged: usize,
    pub skipped_unsupported: usize,
    pub failed: Vec<ScanIssue>,
}

pub struct ScanIssue {
    pub path: PathBuf,
    pub reason: String,
}
```

Un file corrotto o senza tabelle necessarie deve apparire in `failed`, non fermare la scansione.

### 6.4 Google DTO

I tipi API devono tollerare campi additivi e campi `ttf` mancanti.

```rust
pub struct GoogleFontCatalogItem {
    pub id: String,
    pub family: String,
    pub variants: Vec<String>,
    pub subsets: Vec<String>,
    pub category: String,
    pub def_subset: Option<String>,
    pub def_variant: Option<String>,
}

pub struct GoogleFontDetail {
    pub id: String,
    pub family: String,
    pub category: String,
    pub def_subset: Option<String>,
    pub def_variant: Option<String>,
    pub variants: Vec<GoogleFontVariant>,
}

pub struct GoogleFontVariant {
    pub id: String,
    pub font_family: Option<String>,
    pub font_style: Option<String>,
    pub font_weight: Option<String>,
    pub ttf: Option<Url>,
}
```

Un’azione download deve fallire in modo esplicito con `Nessun file TTF disponibile per questa variante.` se `ttf` è assente.

---

## 7. Database SQLite

### 7.1 Apertura

Ogni connessione deve eseguire nell’ordine:

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
```

Per v1 si apre una connessione per task bloccante; `rusqlite::Connection` non deve essere conservata nello stato Iced né passata fra thread.

### 7.2 Schema v1

```sql
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at_unix_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS scan_roots (
  path TEXT PRIMARY KEY,
  added_at_unix_ms INTEGER NOT NULL,
  last_scanned_at_unix_ms INTEGER,
  is_enabled INTEGER NOT NULL DEFAULT 1 CHECK (is_enabled IN (0, 1))
);

CREATE TABLE IF NOT EXISTS fonts (
  id TEXT PRIMARY KEY,
  family TEXT NOT NULL,
  subfamily TEXT NOT NULL,
  full_name TEXT NOT NULL,
  weight INTEGER NOT NULL CHECK (weight BETWEEN 1 AND 1000),
  is_italic INTEGER NOT NULL CHECK (is_italic IN (0, 1)),
  file_path TEXT NOT NULL UNIQUE,
  source TEXT NOT NULL CHECK (source IN ('local', 'google')),
  source_family_id TEXT,
  source_variant_id TEXT,
  category TEXT NOT NULL,
  is_active INTEGER NOT NULL DEFAULT 0 CHECK (is_active IN (0, 1)),
  is_favorite INTEGER NOT NULL DEFAULT 0 CHECK (is_favorite IN (0, 1)),
  file_size INTEGER NOT NULL,
  file_modified_unix_ms INTEGER NOT NULL,
  missing_on_disk INTEGER NOT NULL DEFAULT 0 CHECK (missing_on_disk IN (0, 1)),
  created_at_unix_ms INTEGER NOT NULL,
  updated_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_fonts_family ON fonts(family COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_fonts_active ON fonts(is_active);
CREATE INDEX IF NOT EXISTS idx_fonts_favorite ON fonts(is_favorite);
CREATE INDEX IF NOT EXISTS idx_fonts_source ON fonts(source);

CREATE TABLE IF NOT EXISTS collections (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  created_at_unix_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS collection_items (
  collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
  font_id TEXT NOT NULL REFERENCES fonts(id) ON DELETE CASCADE,
  PRIMARY KEY (collection_id, font_id)
);
```

### 7.3 Query obbligatorie

Il modulo deve offrire almeno:

```rust
pub fn migrate(db_path: &Path) -> Result<(), AppError>;
pub fn list_fonts(query: FontQuery) -> Result<Vec<FontItem>, AppError>;
pub fn upsert_font(item: &FontItem) -> Result<UpsertOutcome, AppError>;
pub fn set_font_active(id: &str, active: bool) -> Result<(), AppError>;
pub fn set_font_favorite(id: &str, favorite: bool) -> Result<(), AppError>;
pub fn list_active_fonts() -> Result<Vec<FontItem>, AppError>;
pub fn mark_root_missing_files(root: &Path, present_paths: &[PathBuf]) -> Result<(), AppError>;
pub fn add_scan_root(root: &Path) -> Result<(), AppError>;
pub fn list_scan_roots() -> Result<Vec<ScanRoot>, AppError>;
pub fn create_collection(name: &str) -> Result<Collection, AppError>;
pub fn rename_collection(id: i64, name: &str) -> Result<(), AppError>;
pub fn delete_collection(id: i64) -> Result<(), AppError>;
pub fn add_font_to_collection(collection_id: i64, font_id: &str) -> Result<(), AppError>;
pub fn remove_font_from_collection(collection_id: i64, font_id: &str) -> Result<(), AppError>;
pub fn list_collections() -> Result<Vec<Collection>, AppError>;
```

`FontQuery` deve includere tab, testo query, categoria, collection opzionale, page e page size. Il page size UI è fisso a `50`; il repository può ricevere solo valori `1..=100` e deve rifiutare gli altri.

### 7.4 Semantica di scansione

Durante un rescan di una root:

1. aggiornare o inserire ogni font valido trovato;
2. non cambiare `is_active` e `is_favorite` di un record esistente;
3. marcare `missing_on_disk = 1` i record precedentemente associati alla root e non più presenti;
4. non eliminare automaticamente record Google;
5. un font attivo diventato mancante deve rimanere DB-attivo ma deve mostrare errore e non essere attivato al prossimo avvio.

---

## 8. Scanner OpenType

### 8.1 Interfaccia

```rust
pub fn scan_directory(root: &Path) -> Result<ScanReport, AppError>;
pub fn parse_font_file(path: &Path) -> Result<ParsedFontMetadata, AppError>;
pub fn is_supported_font_path(path: &Path) -> bool;
```

### 8.2 Regole di filesystem

- Usare `WalkDir` con `follow_links(false)`.
- Non seguire junction, symlink o directory reparse point.
- Ignorare entry che non sono file regolari.
- Estensioni case-insensitive supportate: `ttf`, `otf`.
- Non fidarsi dell’estensione: il parser deve comunque validare i bytes.
- Leggere un file per volta; non accumulare tutti i bytes della directory in RAM.
- I/O e parsing devono essere eseguiti in `Task::perform`/runtime worker.

### 8.3 Estrazione metadata

Il parser deve estrarre:

1. family: Name ID 16, fallback Name ID 1;
2. subfamily: Name ID 17, fallback Name ID 2, fallback stringa `Regular`;
3. full name: Name ID 4, fallback `family + " " + subfamily` normalizzato;
4. weight: valore OS/2 se disponibile; fallback `400`; clamp `1..=1000`;
5. italic: flag OpenType o macStyle; fallback `false`;
6. categoria: `Unknown` per font locali in v1;
7. dimensione e modifica file dal metadata filesystem.

Per record name multipli, scegliere una stringa Unicode decodificabile con priorità Windows English (language id `0x0409`), poi Unicode, poi qualsiasi stringa Unicode valida. Trim degli spazi esterni; stringa vuota equivale a assente.

### 8.4 Protezione input ostile

- Errore di parsing: `AppError::FontParse { path, source }`.
- Nome invalido o assente: fallback controllato; se manca anche family, scartare il file.
- Mai eseguire font, macro o codice dal file.
- Nessun parser `unsafe` aggiuntivo senza revisione esplicita.

---

## 9. Font Iced e interning dei nomi

### 9.1 Vincolo

In Iced 0.13 `Font::with_name` riceve `&'static str`. I nomi arrivano dal filesystem/rete e sono `String` dinamiche.

### 9.2 API

```rust
pub fn intern_font_family(name: &str) -> &'static str;
pub fn font_for_item(item: &FontItem) -> iced::Font;
```

### 9.3 Implementazione richiesta

Usare:

```rust
static FONT_NAMES: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
```

Algoritmo:

1. acquisire il mutex;
2. cercare un valore già presente con contenuto uguale;
3. se presente, restituirlo;
4. se assente, fare `Box::leak(name.to_owned().into_boxed_str())`;
5. inserire e restituire il riferimento leaked.

Il leak è intenzionale e limitato ai nomi effettivamente caricati in Iced nel processo. Non internare automaticamente l’intero catalogo Google. Non richiamare `Box::leak` per nomi già presenti.

### 9.4 Variante

`font_for_item` deve usare family internata e mappare peso `1..=1000` ai nove `iced::font::Weight` disponibili; deve impostare stile italic quando `is_italic` è `true`. Non usare sempre il peso normale.

### 9.5 Caricamento asincrono

`iced::font::load(bytes)` restituisce `Task<Result<(), iced::font::Error>>`. L’app deve generare un messaggio `FontBytesReady`, quindi restituire il task `iced::font::load(bytes).map(...)`. Solo al messaggio successivo `IcedFontLoaded(Ok(()))` la card può usare il font come preview pronto.

Stati per ogni preview:

```rust
pub enum PreviewState {
    NotRequested,
    Downloading,
    LoadingIntoIced,
    Ready,
    Failed(String),
}
```

---

## 10. Wrapper Win32 per font temporanei

### 10.1 API pubblica

```rust
pub fn activate_font(path: &Path) -> Result<u32, AppError>;
pub fn deactivate_font(path: &Path) -> Result<(), AppError>;
pub fn broadcast_font_change() -> Result<(), AppError>;
```

La build non Windows deve offrire gli stessi simboli e restituire `AppError::UnsupportedPlatform`, così test e UI rimangono compilabili con `cfg` appropriati.

### 10.2 Conversione path

Su Windows:

```rust
path.as_os_str()
    .encode_wide()
    .chain(std::iter::once(0))
    .collect::<Vec<u16>>()
```

Il path deve esistere ed essere file regolare prima dell’invocazione GDI.

### 10.3 Attivazione

1. costruire UTF-16 null-terminated;
2. chiamare `unsafe { AddFontResourceW(PCWSTR(wide.as_ptr())) }`;
3. se ritorna `0`, restituire `AppError::Win32FontLoad`;
4. richiamare `broadcast_font_change()`;
5. se il broadcast fallisce, loggare warning e restituire successo di attivazione: il font è stato aggiunto comunque;
6. restituire il numero di font aggiunti.

### 10.4 Disattivazione

1. chiamare `RemoveFontResourceW` con lo stesso path;
2. se ritorna `BOOL(0)`, restituire `AppError::Win32FontRemove`;
3. notificare Windows con lo stesso criterio warning non bloccante.

### 10.5 Broadcast

```rust
SendMessageTimeoutW(
    HWND_BROADCAST,
    WM_FONTCHANGE,
    WPARAM(0),
    LPARAM(0),
    SMTO_ABORTIFHUNG,
    1000,
    Some(&mut result),
)
```

Il broadcast deve essere chiamato in un worker. Con `HWND_BROADCAST`, 1000 ms è il limite per singola finestra e il tempo totale può essere maggiore. L’UI mostra `Aggiornamento font notificato alle applicazioni.` in caso di successo e `Font attivo; alcune applicazioni potrebbero richiedere un riavvio.` in caso di timeout/failure.

### 10.6 Invariante dei riferimenti

Una chiamata `AddFontResourceW` incrementa il riferimento del processo/sessione. L’app deve chiamare `RemoveFontResourceW` una volta per ogni attivazione effettuata nella sessione dall’app. Non provare cicli aggressivi di rimozione, perché potrebbero interferire con caricamenti appartenenti ad altri processi.

---

## 11. Client Google Fonts

### 11.1 Endpoint

- Catalogo: `GET https://gwfh.mranftl.com/api/fonts`
- Dettaglio: `GET https://gwfh.mranftl.com/api/fonts/{id}`

Non richiedere API key. Non costruire URL GitHub come fallback automatico in v1: una fonte distinta con schema diverso aumenta il rischio. Quando l’endpoint non è raggiungibile, usare cache valida o mostrare stato offline.

### 11.2 Cache catalogo

Metadata cache:

```json
{
  "schema_version": 1,
  "fetched_at_unix_ms": 0,
  "etag": null
}
```

Algoritmo `load_catalog()`:

1. se cache presente e età < 24 ore, caricarla subito;
2. in background tentare refresh usando `If-None-Match` se ETag presente;
3. su `304`, aggiornare solo `fetched_at`;
4. su risposta JSON valida, scrivere catalogo e metadata atomicamente;
5. su rete/HTTP/JSON error, conservare cache esistente e mostrare warning non modale;
6. se non esiste cache valida, mostrare `Google Fonts non disponibile offline.` con pulsante `Riprova`.

Timeout HTTP: 20 secondi per richiesta. Redirect abilitati solo HTTPS→HTTPS. Risposte non 2xx sono errori. Il body massimo del catalogo è 10 MiB; un body più grande deve essere rifiutato.

### 11.3 Preview Google

Quando l’utente preme `Carica anteprima`:

1. richiedere dettaglio famiglia, usando cache in memoria se disponibile;
2. selezionare `def_variant`, fallback `regular`, fallback prima variante avente `ttf`;
3. scaricare massimo 25 MiB;
4. validare il TTF con lo stesso parser font;
5. scrivere nella preview cache atomicamente;
6. inoltrare i byte a `iced::font::load`;
7. aggiornare `PreviewState`.

Una preview non deve creare record nel database né attivare font Windows.

### 11.4 Download + attivazione di una variante

Quando l’utente sceglie `Scarica e attiva`:

1. disabilitare la sola azione di quella variante e indicare progresso;
2. scaricare/validare il TTF come sopra;
3. salvarlo in `<vault>/google/<family-id>/<variant-id>.ttf`;
4. eseguire parser e ottenere nome interno reale;
5. costruire `FontItem` con `source=Google`, `source_family_id`, `source_variant_id`, categoria da API normalizzata;
6. in una transazione DB, upsertare il record con `is_active = 0` iniziale;
7. caricare i byte in Iced e attendere successo;
8. chiamare `activate_font` in worker;
9. se GDI riesce, aggiornare `is_active = 1` nel DB;
10. se Iced o GDI fallisce, lasciare il file vault valido e record DB inattivo; mostrare il messaggio specifico.

Non impostare `is_active = 1` prima del successo GDI.

### 11.5 Download intera famiglia

Azione distinta: `Scarica famiglia`. Mostrare conferma con numero varianti scaricabili. Ogni variante segue il flusso atomico individuale. Alla prima failure non cancellare quelle già riuscite; il report finale elenca successi e fallimenti. Questa azione scarica, ma non attiva automaticamente ogni variante; l’utente può attivare una o più varianti dalla libreria locale.

---

## 12. UI Iced

### 12.1 Tema

```text
background:       #0F172A
surface/card:     #1E293B
surface/hover:    #334155
accent:           #38BDF8
accent-secondary: #6366F1
success:          #22C55E
warning:          #F59E0B
error:            #F87171
text-primary:     #F8FAFC
text-muted:       #94A3B8
border:           #334155
```

L’interfaccia deve avere contrasto leggibile e focus visibile da tastiera. Nessuna azione può essere distinguibile solo dal colore.

### 12.2 Layout desktop

- finestra iniziale: 1280×800, minimo 1000×680;
- sidebar sinistra fissa: 240 px;
- area contenuto: riempie lo spazio rimanente;
- top bar sticky nell’area contenuto;
- area card `Scrollable` verticale;
- niente virtualizzazione custom in v1: query paginata e massimo 50 card renderizzate.

### 12.3 Sidebar

Ordine obbligatorio:

1. nome `RustFontManager`;
2. badge: `N attivi in questa sessione`;
3. tab: `Libreria locale`, `Attivi ora`, `Preferiti`, `Google Fonts`;
4. heading `Collezioni`;
5. lista collezioni;
6. input `Nuova collezione` e bottone `Crea`;
7. in fondo bottone `+ Aggiungi cartella locale`.

`N` è il numero di record DB attivi che esistono ancora su disco, non la quantità di font che Windows effettivamente enumera.

### 12.4 Top bar

Da sinistra a destra:

1. ricerca placeholder `Cerca per famiglia o stile…`;
2. input preview placeholder `Il tuo testo di anteprima` con default `The quick brown fox jumps over the lazy dog`;
3. slider dimensione 14–96, default 32, step 1;
4. selector categoria: `Tutte`, `Sans-serif`, `Serif`, `Display`, `Monospace`.

Ricerca e filtro devono aggiornarsi immediatamente con debounce massimo 150 ms. Cambiare dimensione o testo non deve rileggere DB né rete.

### 12.5 Card font locale/scaricato

Header:

- family;
- subfamily;
- badge `LOCALE` oppure `GOOGLE`;
- badge categoria se disponibile;
- peso e corsivo.

Body:

- testo anteprima;
- font Iced caricato se `PreviewState::Ready`, font di sistema con label `Anteprima non caricata` altrimenti.

Footer:

- toggle `Attivo su Windows` / `Disattivato`;
- bottone preferito;
- menu `Copia snippet` con `CSS` e `Next.js`;
- menu `Aggiungi a collezione`.

Un record `missing_on_disk` disabilita toggle e copy, mostra badge `FILE MANCANTE` e bottone `Rimuovi dalla libreria` con conferma.

### 12.6 Card Google cloud

Header: family, categoria, numero di varianti disponibili.

Body: preview di sistema iniziale e stato `Anteprima non caricata`.

Footer:

- `Carica anteprima`;
- `Scarica e attiva` per variante predefinita;
- selector variante;
- azione secondaria `Scarica famiglia`.

### 12.7 Stato e messaggi

Usare una coda di toast non bloccanti per successi/warning/errori. Gli errori che chiedono una decisione devono usare dialog modale soltanto per:

- conferma rimozione record/font da collezione;
- conferma download famiglia;
- errore irreversibile di inizializzazione database.

Ogni task deve avere un indicatore di progresso localizzato alla card o root interessata; non usare una spinner globale per una singola preview.

---

## 13. Stato applicazione e messaggi

### 13.1 Stato minimo

```rust
pub struct AppState {
    pub tab: LibraryTab,
    pub query: String,
    pub preview_text: String,
    pub preview_size: u16,
    pub category: FontCategory,
    pub current_page: u32,
    pub fonts: Vec<FontItem>,
    pub total_results: usize,
    pub collections: Vec<Collection>,
    pub selected_collection: Option<i64>,
    pub preview_states: HashMap<String, PreviewState>,
    pub google_catalog: CatalogState,
    pub operations: HashMap<String, OperationState>,
    pub toasts: VecDeque<Toast>,
}
```

### 13.2 Regole `Message`

L’enum deve separare input UI, risultati DB, risultati filesystem/rete e risultati Iced. Esempi obbligatori:

```rust
TabSelected(LibraryTab),
SearchChanged(String),
PreviewTextChanged(String),
PreviewSizeChanged(u16),
CategorySelected(FontCategory),
PageRequested(u32),
FontsLoaded(Result<FontPage, AppError>),
ChooseFolderPressed,
FolderChosen(Option<PathBuf>),
ScanFinished(Result<ScanReport, AppError>),
ToggleFontActivation(String),
FontActivationFinished { id: String, target: FontActivation, result: Result<(), AppError> },
ToggleFavorite(String),
FavoriteUpdated { id: String, result: Result<(), AppError> },
GoogleCatalogLoaded(Result<Vec<GoogleFontCatalogItem>, AppError>),
GooglePreviewRequested { family_id: String, variant_id: Option<String> },
FontBytesReady { key: String, bytes: Vec<u8>, item: FontItem },
IcedFontLoaded { key: String, result: Result<(), iced::font::Error> },
GoogleInstallRequested { family_id: String, variant_id: String },
GoogleInstallFinished { id: String, result: Result<FontItem, AppError> },
CopySnippetRequested { id: String, kind: SnippetKind },
CollectionCreateRequested,
CollectionCreated(Result<Collection, AppError>),
ToastDismissed(u64),
```

I byte non devono essere conservati in `AppState` dopo la trasformazione in `iced::font::load`; la cache filesystem è la sorgente per ricaricamenti futuri.

### 13.3 Avvio

1. inizializzare paths e logging;
2. migrare DB;
3. caricare collezioni e prima pagina locale;
4. query record `is_active=1 AND missing_on_disk=0`;
5. per ogni record: leggere bytes, inviare `iced::font::load`, poi chiamare GDI worker;
6. se file non leggibile o GDI fallisce: impostare `is_active=0`, mantenere il record e mostrare warning;
7. in parallelo caricare catalogo Google da cache/rete senza bloccare la libreria locale.

Il ripristino deve essere serializzato o limitato a 4 task concorrenti per evitare picco disco/UI.

---

## 14. Snippet clipboard

### 14.1 CSS

Per un font Google:

```css
font-family: "Roboto", sans-serif;
font-weight: 400;
font-style: normal;
```

Per un font locale:

```css
font-family: "Family Name", sans-serif;
font-weight: 400;
font-style: normal;
```

Non generare `@font-face` per un font locale, perché l’app non conosce una URL distribuibile. Se il nome contiene `"`, `\\` o controllo, deve essere escapato CSS.

### 14.2 Next.js

Solo per font con `source=Google` e `source_family_id` presente:

```ts
import { Roboto } from "next/font/google";

const roboto = Roboto({
  subsets: ["latin"],
  weight: ["400"],
  style: ["normal"],
});
```

Per font locale:

```ts
import localFont from "next/font/local";

const familyName = localFont({
  src: "./FamilyName-Regular.ttf",
  display: "swap",
});
```

Lo snippet locale è indicativo e deve includere toast: `Snippet generato: copia il file font nel tuo progetto.`

Il clipboard handler crea `arboard::Clipboard` nella funzione worker e chiama `set_text`. Se fallisce, l’errore deve indicare che gli appunti non sono disponibili.

---

## 15. Errori e logging

### 15.1 Errori dominio

`AppError` deve almeno distinguere:

- `Storage { path, source }`
- `Database { source }`
- `FontParse { path, reason }`
- `InvalidFontMetadata { path, reason }`
- `Http { url, reason }`
- `HttpStatus { url, status }`
- `RemoteFormat { endpoint, reason }`
- `DownloadTooLarge { url, limit_bytes }`
- `Win32FontLoad { path }`
- `Win32FontRemove { path }`
- `Win32Broadcast { reason }`
- `Clipboard { reason }`
- `UnsupportedPlatform`
- `IcedFontLoad { family, reason }`

Non esporre stack trace grezzi all’utente. Loggare il dettaglio tecnico con `tracing`; mostrare un testo orientato all’azione.

### 15.2 Logging

- livello `info`: avvio, scansione conclusa, download completato, attivazione completata;
- livello `warn`: font corrotto ignorato, broadcast fallito, cache stale, file mancante;
- livello `error`: migrazione DB, impossibilità di accedere al vault, risposta remota non valida;
- non loggare contenuti clipboard, anteprime, percorsi oltre quanto necessario alla diagnostica locale;
- rotazione semplice: massimo 5 file da 2 MiB, oppure log unico troncato al lancio.

---

## 16. Test obbligatori

### 16.1 Unit test

| Modulo | Caso |
|---|---|
| `paths` | usa directory locale e produce path deterministici per DB/cache/vault |
| `font_names` | stesso nome restituisce lo stesso `&'static str`; nomi diversi restano diversi |
| `snippets` | CSS/Next corretto per Google, locale e caratteri da escape |
| `db` | schema nuovo, migrazione idempotente, foreign key attiva, collezione case-insensitive, duplicate collection item non duplica |
| `db` | upsert conserva `is_active` e `is_favorite` |
| `db` | paginazione 50 elementi e filtro case-insensitive |
| `scanner` | `.TTF`, `.otf` supportati; `.woff2` ignorato |
| `scanner` | bytes invalidi producono errore e non panic |
| `scanner` | fallback ID family/subfamily/full name |
| `google_fonts` | catalogo JSON reale semplificato deserializza campi noti e ignora extra |
| `google_fonts` | variante senza `ttf` produce errore esplicito |
| `google_fonts` | cache stale resta disponibile dopo errore HTTP |
| `google_fonts` | rifiuta body oltre limite |

### 16.2 Test integrazione

1. DB temporaneo: creare font, collezione, assegnazione, delete collezione; verificare cascata.
2. Directory temporanea: un font fixture valido e un file corrotto; report indica un import e una failure.
3. Vault temporaneo: simulare download TTF valido, verificare che il file finale esista solo dopo validazione.
4. Avvio service: record attivo con path mancante deve essere disattivato nel DB e produrre warning.

### 16.3 Controlli statici obbligatori

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release --target x86_64-pc-windows-msvc
```

Tutti devono terminare con exit code 0. La build release è necessaria ma non prova il comportamento GDI/GPU.

### 16.4 Test manuale Windows obbligatorio

1. avviare l’app con rete disponibile e senza dati precedenti;
2. selezionare una cartella con almeno 20 font e un file corrotto;
3. verificare report scansione, ricerca, filtro, paginazione e preferito;
4. attivare e disattivare un font locale; controllare in un programma già aperto che riceva, quando supportato, l’aggiornamento;
5. scaricare preview Google e verificare render reale;
6. scaricare e attivare una variante Google, chiudere e riaprire l’app; verificare ripristino;
7. disabilitare rete e verificare catalogo cache/offline;
8. controllare che `C:\Windows\Fonts` e Registro non siano stati modificati;
9. testare a 100%, 150% e 200% DPI;
10. chiudere l’app e verificare che non rimangano processi dell’app.

---

## 17. Criteri di accettazione

Il progetto può essere dichiarato pronto per una prima distribuzione solo se ogni condizione seguente è verificata con output reale:

1. non esiste `package.json` nel repository;
2. `cargo fmt`, Clippy, test e build release sono verdi;
3. non esistono `todo!`, `unimplemented!`, `unwrap()` o `expect()` nel codice produzione, eccetto test dove la causa è esplicita;
4. scanner importa font validi e sopravvive a file corrotti;
5. SQLite mantiene font, stato, collezioni e relazioni dopo riavvio;
6. GDI attiva/disattiva un font senza installazione permanente;
7. broadcast viene chiamato senza eseguire I/O/attese nel thread UI;
8. Google catalogo funziona da rete e da cache offline;
9. download non produce file/record parziali su errore;
10. preview Iced usa il font scaricato solo dopo successo di `iced::font::load`;
11. UI non renderizza più di 50 card per pagina;
12. manual QA Windows è documentata con versione Windows, GPU, font usati e risultati.

---

## 18. Sequenza di sviluppo obbligatoria

### Fase 1 — Fondazioni

Creare manifest, errori, paths, modelli, database e test DB. Nessuna GUI complessa, rete o Win32 prima del DB verde.

### Fase 2 — Scanner

Integrare parser Fontations, fixture, scan root e persistenza. Verificare fallback metadata e font corrotti.

### Fase 3 — Attivazione Windows

Implementare wrapper GDI isolato e usare un piccolo test/manual probe Windows. Aggiungere integration service che aggiorna DB solo dopo esito GDI.

### Fase 4 — Libreria locale UI

Costruire app Iced: sidebar, ricerca, filtri, pagina, preview locale, preferiti, snippet, collezioni.

### Fase 5 — Google Fonts

Catalogo cache, dettaglio, preview task, vault atomico, install/activate singola variante, poi famiglia.

### Fase 6 — Ripristino e resilienza

Implementare bootstrap dei font attivi, file mancanti, limiti concorrenza, toast, logging e offline.

### Fase 7 — Hardening e release

Clippy, test completi, build release, manual QA Windows, README utente e note limitazioni.

Non saltare una fase perché una UI appare funzionante: l’ordine protegge l’integrità DB, la sicurezza dei parser e la semantica di attivazione sessione.

---

## 19. Limiti dichiarati all’utente

- I font sono attivi solo fino al riavvio Windows o alla loro rimozione dalla sessione; non vengono installati permanentemente.
- Alcuni programmi aggiornano l’elenco font soltanto alla riapertura anche dopo `WM_FONTCHANGE`.
- Il catalogo Google dipende dal servizio remoto e dalla rete; la cache permette consultazione offline dell’ultimo catalogo valido.
- Le font collection `.ttc`, i formati web e i font legacy non sono gestiti nella prima versione.
- Il rendering Iced è GPU accelerated quando il backend WGPU è disponibile; il sistema può applicare fallback in base al driver.
