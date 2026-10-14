# Open Font Manager

Applicazione desktop nativa per Windows 11 che indicizza font locali, mostra anteprime e consente di installare i font per l’utente Windows corrente. Consulta anche Google Fonts e organizza i caratteri in collezioni.

L’interfaccia desktop usa Tauri 2, React, TypeScript e DaisyUI; scanner, database, installazione dei font e servizi restano nel core Rust. Le operazioni di filesystem e SQLite vengono eseguite fuori dal thread della UI.

> **Stato:** versione iniziale in sviluppo. Non è ancora distribuito un installer. L’installazione persistente per l’utente corrente è implementata, ma va verificata manualmente nelle applicazioni Windows che usi.

## Funzioni

- Indicizzazione automatica all’avvio dei font installati per il sistema e per l’utente corrente; è possibile aggiungere anche altre cartelle.
- Lettura dei formati `.ttf`, `.otf` e `.ttc`, incluse tutte le facce dei font collection.
- Catalogo locale SQLite con ricerca, anteprime, preferiti, collezioni e paginazione.
- Installazione persistente per l’utente Windows corrente e rimozione da Windows dei soli font gestiti da OFM.
- Riconoscimento nella Libreria locale dei font già presenti nelle cartelle standard di Windows; OFM non offre la rimozione di installazioni esterne.
- Ripristino all’avvio dei font installati tramite OFM e notifica `WM_FONTCHANGE` alle applicazioni.
- Catalogo Google Fonts con cache locale, anteprima su richiesta, download di una variante o di un’intera famiglia.
- Copia negli appunti di snippet CSS e `next/font`.
- Dati applicativi e cache separati nelle directory locali individuate da `directories`.
- Importazione automatica di ZIP trascinate nell’app: estrae solo font `.ttf` e `.otf` validi nella cartella gestita e li aggiunge alla Libreria locale.

## Limitazioni note

- Target supportato: Windows 11 x86_64 con toolchain MSVC.
- Formati locali supportati: `.ttf`, `.otf` e `.ttc`. `.woff`, `.woff2`, Type 1 e font bitmap non sono inclusi.
- L’installazione usa la cartella font dell’utente corrente e la registrazione HKCU; non richiede privilegi amministrativi e non scrive in `C:\Windows\Fonts`.
- `WM_FONTCHANGE` è una notifica: ogni applicazione terza decide autonomamente se aggiornare subito la propria lista di font.
- Il catalogo online usa il servizio pubblico `google-webfonts-helper` (`gwfh.mranftl.com`), che è un servizio di terze parti e può essere indisponibile o cambiare. La cache può mantenere disponibile l’ultimo catalogo salvato.
- Non è inclusa una system tray.
- Non è ancora stato eseguito un collaudo manuale completo su più applicazioni Windows né sono pubblicati installer o release binarie.
- Il passaggio a Tauri non è ancora stato valutato con un profilo frame-time né collaudato trascinando la finestra tra monitor con scaling DPI diverso. Fluidità e stabilità vanno verificate nell’uso reale.

## Avvio rapido

### Requisiti

- Windows 11 x64.
- Rust 1.80 o successivo con target MSVC (`x86_64-pc-windows-msvc`) e linker di Visual Studio Build Tools.
- Node.js e npm per Vite, React e la CLI Tauri.
- WebView2 Runtime, normalmente già presente su Windows 11.

### Compilazione ed esecuzione

```powershell
rustup target add x86_64-pc-windows-msvc
npm install
npm run tauri -- dev
```

Per creare l’eseguibile ottimizzato:

```powershell
npm run tauri -- build --no-bundle --target x86_64-pc-windows-msvc
```

L’eseguibile viene creato in `target\x86_64-pc-windows-msvc\release\open-font-manager-app.exe`. La configurazione attuale non genera installer o bundle.

### Utilizzo

1. Avvia l’app: i font nelle cartelle Windows e nella cartella font dell’utente corrente vengono indicizzati automaticamente. Usa **Aggiungi cartella locale** per altre directory.
2. Cerca i font, modifica testo e dimensione dell’anteprima, filtra per categoria e gestisci preferiti o collezioni.
3. Usa **Installa** per rendere il font disponibile alle applicazioni Windows anche dopo la chiusura di OFM; usa **Disinstalla** per rimuovere da Windows una copia installata da OFM. I font già installati esternamente sono indicati come **Installato** e non vengono rimossi da OFM.
4. Apri **Google Fonts** per cercare il catalogo. La prima consultazione richiede una connessione; le varianti scaricate compaiono direttamente nella Libreria locale.
5. Copia lo snippet CSS o Next.js dagli strumenti della scheda del font.
6. Trascina un archivio `.zip` nella finestra per aggiungere i font desktop validi alla Libreria locale; l’importazione non li installa nel Registro o in `C:\Windows\Fonts`.

## Test e controlli

I test ordinari non richiedono accesso a Internet:

```powershell
cargo fmt --all -- --check
cargo test --workspace --target x86_64-pc-windows-msvc
cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings
npm test -- --reporter=dot
npm run build
```

Il test live Google Fonts è marcato `#[ignore]` perché dipende dal servizio pubblico. Può essere eseguito esplicitamente con:

```powershell
cargo test -p rust-font-manager --target x86_64-pc-windows-msvc --test google_fonts_tests -- --ignored
```

## Struttura

- `src-tauri/src/lib.rs`, `src-tauri/src/commands.rs`: runtime Tauri e comandi IPC.
- `src-tauri/src/backend.rs`: adattatore desktop verso i servizi del core Rust.
- `ui/src/App.tsx`, `ui/src/styles.css`: interfaccia React e tema DaisyUI.
- `src/font_scanner.rs`: lettura dei metadati OpenType con Fontations.
- `src/win_font_loader.rs`: notifica Windows del cambiamento dei font.
- `src/db.rs`, `src/paths.rs`: archivio SQLite e percorsi applicativi.
- `src/google_fonts.rs`, `src/services/google_library.rs`: client HTTP, cache e download Google Fonts.
- `src/services/local_library.rs`: scansione della libreria locale.
- `src/services/font_install.rs`: installazione persistente e gestione delle registrazioni Windows per utente.
- `src/services/zip_import.rs`: estrazione sicura e validazione dei font da archivi ZIP.
- `src/snippets.rs`: generazione di snippet.


## Privacy e dati

La libreria, i preferiti, le collezioni e i file scaricati rimangono sul dispositivo. Le richieste al catalogo e ai file Google Fonts contattano il servizio pubblico `gwfh.mranftl.com` e gli host di distribuzione dei font. I percorsi locali scansionati non sono inviati dal client.

I file font scaricati sono conservati nell’archivio dati dell’app e indicizzati nella Libreria locale; la scansione dei font esistenti non li copia né li installa.

## Licenza

Il repository è pubblico, ma al momento non contiene una licenza open source. La pubblicazione non concede automaticamente diritti di riutilizzo o ridistribuzione. La licenza è da definire.
