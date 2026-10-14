> **Specifica originaria superata:** questo documento descriveva la versione Iced. Il progetto attuale usa Tauri 2 con React e TypeScript; per architettura e avvio fare riferimento a `README.md` e ai manifest correnti. Il resto è conservato come traccia storica, non come vincolo attuale.

un manager dei font
Project Specification: RustFontManager (100% Pure Rust Edition)

1. Ruolo e Obiettivo per l'Agente AI

Agisci come un Senior Rust Systems & GUI Engineer.
L'obiettivo è sviluppare un Font Manager desktop nativo per Windows 11, scritto al 100% in Rust puro (nessuna WebView, nessun Tauri/Electron, nessun JavaScript). L'applicazione deve compilare in un singolo binario .exe standalone, ultra-leggero, accelerato via GPU e con un consumo minimo di RAM.

L'applicazione deve permettere di:

Scansionare e catalogare cartelle di font locali (.ttf, .otf) leggendone le tabelle binarie senza installarli nel sistema.

Attivare/disattivare i font "on-the-fly" nella sessione di Windows tramite API Win32 GDI (AddFontResourceW / RemoveFontResourceW) e notificare in tempo reale i software aperti (Adobe, Affinity, CAD, IDE) via WM_FONTCHANGE.

Esplorare l'intero catalogo di Google Fonts (senza richiedere API key grazie all'endpoint pubblico google-webfonts-helper), renderizzando le anteprime in tempo reale direttamente nella GUI Rust e permettendo il Download + Attivazione su Windows con 1-Click.

Organizzare i font in "Collezioni" locali ed esportare al volo i file o gli snippet CSS/next/font negli appunti.

2. Tech Stack (100% Cargo / Pure Rust)

Non deve esistere alcun package.json. Tutta l'architettura risiede in un unico workspace Cargo.

[package]
name = "rust-font-manager"
version = "0.1.0"
edition = "2021"

[dependencies]
# GUI Nativa GPU-accelerated (Architettura Elm)
iced = { version = "0.13", features = ["tokio", "advanced", "lazy", "image"] }

# Async & Networking (Google Fonts API + Download TTF)
tokio = { version = "1.40", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "stream"] }
futures = "0.3"

# Parsing Binario dei Font
ttf-parser = "0.24"
walkdir = "2.5"

# Database & Persistenza Locale
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
directories = "5.0" # Per trovare AppData/Local in modo pulito

# Integrazione Sistema Operativo & Utility
arboard = "3.4"     # Gestione Clipboard nativa per copiare snippet
rfd = "0.15"        # Native File Dialogs per scegliere le cartelle locali
tray-icon = "0.19"  # System Tray nativa per esecuzione in background

# Windows Win32 API (GDI Font Loading + Broadcast)
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_Graphics_Gdi",
    "Win32_UI_WindowsAndMessaging",
] }


3. Architettura dei Moduli (src/)

Struttura della directory richiesta:

src/
├── main.rs              # Entrypoint, inizializzazione Iced e System Tray
├── app.rs               # Stato principale Iced (State, Message, update, view)
├── theme.rs             # Palette colori Dark Mode professionale e stili custom Iced
├── models.rs            # Struct condivise (FontItem, GoogleFontItem, Collection)
├── db.rs                # Gestione SQLite (rusqlite) per font locali, attivi e collezioni
├── win_font_loader.rs   # Chiamate unsafe Win32 GDI (AddFontResourceW / Broadcast)
├── font_scanner.rs      # Scansione cartelle con walkdir e parsing con ttf-parser
└── google_fonts.rs      # Client API Google Fonts + caching TTF per anteprime GUI


4. Specifiche Tecniche Dettagliate dei Moduli

Modulo 1: win_font_loader.rs (Attivazione Temporanea Win32)

Non deve sporcare C:\Windows\Fonts né il Registro di Windows. Usa il caricamento in sessione GDI:

Conversione Stringhe: Converte &Path in UTF-16 null-terminated:
path.as_os_str().encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>()

pub fn activate_font(path: &Path) -> Result<(), String>:

Chiama unsafe { AddFontResourceW(PCWSTR(wide_path.as_ptr())) }.

Se ritorna 0, restituisce Err.

Chiama broadcast_font_change().

pub fn deactivate_font(path: &Path) -> Result<(), String>:

Chiama unsafe { RemoveFontResourceW(PCWSTR(wide_path.as_ptr())) }.

Se ritorna BOOL(0), restituisce Err.

Chiama broadcast_font_change().

pub fn broadcast_font_change():

Usa obbligatoriamente SendMessageTimeoutW con flag SMTO_ABORTIFHUNG (timeout 1000ms) verso HWND_BROADCAST con messaggio WM_FONTCHANGE per evitare che eventuali finestre bloccate su Windows congelino il thread della GUI.

Modulo 2: font_scanner.rs (Parsing ultra-rapido con ttf-parser)

Scansiona una directory fornita dall'utente tramite rfd::AsyncFileDialog::new().pick_folder().

Per ogni file .ttf o .otf, legge i byte e invoca ttf_parser::Face::parse(&bytes, 0).

Estrae dalla tabella name (cercando record Unicode/Windows English):

Family Name (Name ID 16 se presente, altrimenti Name ID 1).

Subfamily / Style (Name ID 17 o Name ID 2).

Full Name (Name ID 4).

Estrae weight (face.weight().to_number()) e is_italic (face.is_italic()).

Ritorna una struct LocalFontMetadata pronta per essere salvata in SQLite.

Modulo 3: google_fonts.rs (Catalogo Zero-Config & 1-Click Install)

Per evitare che l'utente debba configurare una chiave API su Google Cloud, usa l'API pubblica e gratuita di google-webfonts-helper:

Endpoint Catalogo: https://gwfh.mranftl.com/api/fonts (Restituisce JSON con id, family, variants, subsets, category).

Endpoint Dettaglio & URL TTF diretto: https://gwfh.mranftl.com/api/fonts/{id} (Contiene gli URL diretti ai file .ttf per ogni peso).

Alternativa/Fallback diretto: Usa il manifesto JSON ufficiale o costruisci il fetch dei .ttf da GitHub (google/fonts repository raw) o dall'endpoint gwfh.

Come gestire l'anteprima e il "1-Click Download & Activate" in Rust puro:

fetch_catalog(): Scarica la lista all'avvio (e la salva in cache locale in AppData/Local/RustFontManager/gfonts_catalog.json).

Anteprima Live in iced (load_preview_font):

iced espone il comando iced::font::load(bytes: impl Into<Cow<'static, [u8]>>).

Quando l'utente clicca su "Anteprima" o seleziona un Google Font nella lista, un task asincrono scarica il file .ttf della variante regular in memoria (e lo salva nella cartella cache/previews/), poi passa il Vec<u8> a iced::font::load(bytes).

Una volta caricato in iced, il testo della card viene renderizzato usando iced::Font::with_name("Nome Esatto Family").

Bottone "1-Click Scarica & Attiva" (install_and_activate_google_font):

Scarica tutti i .ttf della famiglia (o la variante selezionata) in AppData/Local/RustFontManager/vault/google/{family_id}/.

Li analizza con font_scanner::parse_ttf_file per leggere il nome interno esatto della tabella OpenType.

Li inserisce nella tabella fonts di SQLite con is_active = 1 e source = 'google'.

Chiama iced::font::load(bytes) per renderizzare subito il font nella GUI.

Chiama win_font_loader::activate_font(path) per renderlo immediatamente attivo su tutto Windows.

Modulo 4: db.rs (Persistenza SQLite)

Il database SQLite viene salvato in AppData/Local/RustFontManager/library.db.

Tabella fonts:

id TEXT PRIMARY KEY (hash del path o {google_id}_{variant})

family TEXT NOT NULL

subfamily TEXT NOT NULL

full_name TEXT NOT NULL

weight INTEGER NOT NULL

is_italic INTEGER NOT NULL

file_path TEXT NOT NULL UNIQUE

source TEXT NOT NULL ('local' | 'google')

is_active INTEGER NOT NULL DEFAULT 0

is_favorite INTEGER NOT NULL DEFAULT 0

Tabella collections: id INTEGER PRIMARY KEY, name TEXT UNIQUE.

Tabella collection_items: collection_id INTEGER, font_id TEXT.

Hook di Avvio: All'apertura dell'app, esegui una query per tutti i record con is_active = 1:

Carica i loro byte in iced::font::load per le anteprime GUI.

Chiama win_font_loader::activate_font su ciascuno per ripristinare i font attivi nella sessione Windows corrente.

5. Design della GUI in iced (app.rs & theme.rs)

Usa un tema custom Dark moderno (sfondo #0F172A, card #1E293B, accento #38BDF8 o #6366F1).

Struttura Layout (Row e Column di iced::widget)

Sidebar Sinistra (width: Fixed(240.0)):

Titolo App + Contatore badge dei Font attualmente attivi in RAM su Windows.

Bottoni di navigazione (Tab):

Libreria Locale

Attivi Ora

Preferiti

Google Fonts (Cloud)

Sezione Collezioni: lista delle collezioni + input rapido per crearne una nuova.

Bottone in basso: + Aggiungi Cartella Locale (apre il selettore nativo rfd).

Top Bar (Controlli Globali):

TextInput per cercare un font per nome (filtro istantaneo in memoria).

TextInput per il Testo di Anteprima Custom (es. "The quick brown fox..." o il nome del logo su cui si sta lavorando).

Slider (14.0..=96.0) per regolare in tempo reale la dimensione del testo di anteprima.

Filtro rapido per categoria (All, Sans-Serif, Serif, Display, Monospace).

Area Centrale (Scrollable con Paginazione / Virtualizzazione leggera):

Per mantenere la GUI a 120 FPS anche con 1.600+ font, mostra i risultati filtrati in blocchi (es. 50 card per pagina o caricamento progressivo).

Ogni FontCard contiene:

Header della Card: Nome family, badge categoria/origine (LOCAL o GOOGLE), numero di stili/pesi disponibili.

Body della Card: Il testo di anteprima renderizzato con .font(Font::with_name(family_name)).size(preview_size). (Se è un Google Font non ancora in cache, mostra un pulsante "Carica Anteprima" o caricalo automaticamente quando visibile).

Footer Azioni della Card:

Se Locale o Google già scaricato: Toggle Button [ Attivo su Windows / Disattivato ] (verde quando attivo), bottone [ Copia CSS / Next.js ] (usa arboard), menu Aggiungi a Collezione.

Se Google Font nel Cloud: Bottone primario [ ⬇ 1-Click Scarica & Attiva ] che scarica il .ttf, lo registra nel DB e lo attiva via Win32 in un solo passaggio.

6. Istruzioni di Esecuzione per l'Agente

Procedi con l'implementazione seguendo rigorosamente queste regole:

Zero codice incompleto: Non lasciare todo!() o funzioni vuote. Tutte le chiamate windows e le query rusqlite devono essere complete e gestire gli errori senza fare unwrap() rischiosi su file corrotti.

Borrowing dei nomi Font in iced: Ricorda che in iced 0.13, iced::Font::with_name richiede una &'static str. Crea un piccolo helper con una OnceLock<Mutex<HashSet<&'static str>>> (usando Box::leak solo per le stringhe uniche dei nomi delle famiglie di font registrate) così da poter passare &'static str a Font::with_name() in modo sicuro e senza memory leak ripetuti.

Ordine di sviluppo:

Step 1: Genera Cargo.toml, src/models.rs, src/win_font_loader.rs e src/font_scanner.rs.

Step 2: Genera src/db.rs e src/google_fonts.rs.

Step 3: Genera src/theme.rs, src/app.rs e src/main.rs collegando tutta la logica reattiva tramite iced::Task (o iced::Command).

Inizia subito dallo Step 1, assicurandoti che tutte le dipendenze e i tipi siano allineati.
