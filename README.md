# RustFontManager

Applicazione desktop nativa per Windows 11 che indicizza font locali, permette anteprime e attivazione temporanea nella sessione Windows, consulta Google Fonts e organizza i caratteri in collezioni.

Interfaccia e logica sono scritte in Rust con Iced. Il progetto non usa WebView, Tauri, Electron, JavaScript, Node.js né un servizio locale esterno.

> **Stato:** versione iniziale in sviluppo. Non è ancora distribuito un installer. L’attivazione GDI è stata implementata, ma va verificata sul sistema e nelle applicazioni terze che usi.

## Funzioni

- Scansione ricorsiva di cartelle di font `.ttf` e `.otf`, senza installarli durante l’indicizzazione.
- Catalogo locale SQLite con ricerca, anteprime, preferiti, collezioni e paginazione.
- Attivazione e rimozione temporanee tramite Windows GDI (`AddFontResourceW` / `RemoveFontResourceW`) con broadcast `WM_FONTCHANGE`.
- Ripristino all’avvio dei font registrati come attivi nella sessione precedente.
- Catalogo Google Fonts con cache locale, anteprima su richiesta, download di una variante o di un’intera famiglia.
- Copia negli appunti di snippet CSS e `next/font`.
- Dati applicativi e cache separati nelle directory locali individuate da `directories`.

## Limitazioni note

- Target supportato: Windows 11 x86_64 con toolchain MSVC.
- Formati locali supportati: `.ttf` e `.otf`. `.ttc`, `.woff`, `.woff2`, Type 1 e font bitmap non sono inclusi.
- L’attivazione usa GDI solo per la sessione Windows dell’applicazione. Non installa font in `C:\Windows\Fonts` e non modifica il Registro di Windows.
- `WM_FONTCHANGE` è una notifica: ogni applicazione terza decide autonomamente se aggiornare subito la propria lista di font.
- Il catalogo online usa il servizio pubblico `google-webfonts-helper` (`gwfh.mranftl.com`), che è un servizio di terze parti e può essere indisponibile o cambiare. La cache può mantenere disponibile l’ultimo catalogo salvato.
- Non è inclusa una system tray.
- Non è ancora stato eseguito un collaudo manuale completo su più applicazioni Windows né sono pubblicati installer o release binarie.
- Iced 0.13 usa il renderer Tiny-Skia; il renderer WGPU è disabilitato e le dipendenze `lru` con advisory di soundness sono state rimosse dal lockfile. Il motore testo di Iced dipende ancora transitivamente da `ttf-parser` e `rustybuzz`, segnalati come non mantenuti (`RUSTSEC-2026-0192` e `RUSTSEC-2026-0206`); lo scanner di metadati dell’app usa `skrifa` e `read-fonts`. RustSec non riporta versioni corrette per questi due advisory.

## Avvio rapido

### Requisiti

- Windows 11 x64.
- Rust 1.80 o successivo con target MSVC (`x86_64-pc-windows-msvc`) e linker di Visual Studio Build Tools.

### Compilazione ed esecuzione

```powershell
rustup target add x86_64-pc-windows-msvc
cargo run --target x86_64-pc-windows-msvc
```

Per creare l’eseguibile ottimizzato:

```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

L’eseguibile viene creato in `target\x86_64-pc-windows-msvc\release\rust-font-manager.exe`.

### Utilizzo

1. Avvia l’app e scegli **Aggiungi cartella locale** per indicizzare una o più directory.
2. Cerca i font, modifica testo e dimensione dell’anteprima, filtra per categoria e gestisci preferiti o collezioni.
3. Usa **Attiva su Windows** per aggiungere temporaneamente il font alla sessione GDI; usa **Disattiva** per rimuoverlo.
4. Apri **Google Fonts** per cercare il catalogo. La prima consultazione richiede una connessione; i download vengono salvati nel vault locale.
5. Copia lo snippet CSS o Next.js dagli strumenti della scheda del font.

## Test e controlli

I test ordinari non richiedono accesso a Internet:

```powershell
cargo fmt --all -- --check
cargo test --target x86_64-pc-windows-msvc
cargo clippy --target x86_64-pc-windows-msvc --all-targets -- -D warnings
cargo audit --target-arch x86_64 --target-os windows
```

La verifica RustSec segnala dipendenze non mantenute nel grafo Windows, ma nessun advisory di vulnerabilità o soundness attivo per quel target. `cargo audit` riporta inoltre pacchetti opzionali presenti nel lockfile anche se non fanno parte del grafo Windows effettivamente compilato; Tiny-Skia è abilitato e WGPU è disabilitato.

Il test d’integrazione Google Fonts è marcato `#[ignore]` perché dipende dal servizio pubblico. Può essere eseguito esplicitamente con:

```powershell
cargo test --target x86_64-pc-windows-msvc --test google_fonts_tests -- --ignored
```

## Struttura

- `src/app.rs`: schermate Iced, navigazione e flussi utente.
- `src/theme.rs`: palette dell’interfaccia.
- `src/font_scanner.rs`, `src/font_names.rs`: lettura dei metadati OpenType e gestione dei nomi per l’anteprima.
- `src/win_font_loader.rs`: wrapper GDI Windows.
- `src/db.rs`, `src/paths.rs`: archivio SQLite e percorsi applicativi.
- `src/google_fonts.rs`, `src/services/google_library.rs`: client HTTP, cache e download Google Fonts.
- `src/services/local_library.rs`: scansione, attivazione e ripristino dei font locali.
- `src/snippets.rs`: generazione di snippet.
- `docs/superpowers/specs/`: specifica tecnica dettagliata.

## Privacy e dati

La libreria, i preferiti, le collezioni e il vault dei font scaricati rimangono sul dispositivo. Le richieste al catalogo e ai file Google Fonts contattano il servizio pubblico `gwfh.mranftl.com` e gli host di distribuzione dei font. I percorsi locali scansionati non sono inviati dal client.

I file font scaricati sono salvati nel vault locale dell’app; la scansione dei font esistenti non li copia né li installa.

## Licenza

Il repository è pubblico, ma al momento non contiene una licenza open source. La pubblicazione non concede automaticamente diritti di riutilizzo o ridistribuzione. La licenza è da definire.
