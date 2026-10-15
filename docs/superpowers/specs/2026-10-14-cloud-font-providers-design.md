# Integrazione dei provider cloud di font

## 1. Obiettivo e vincoli

Aggiungere Fontsource, Fontshare e Nerd Fonts al catalogo Cloud già esistente, mantenendo Google Fonts operativo. I quattro provider devono essere presentati tramite un contratto remoto comune, scaricare font installabili nel vault locale e integrarsi con lo scanner e l’attivazione Windows esistenti.

Vincoli: Rust 2021/MSRV 1.80, Tauri 2, React/TypeScript, nessuna API key; archivi ZIP elaborati esclusivamente in memoria; estrazione limitata a TTF/OTF validi. Le modifiche concorrenti dell’altro agente in `ui/src/App.tsx`, `ui/src/styles.css`, test associati e ogni altro file non vanno sovrascritte: l’implementatore deve ricontrollare `git status` e integrare i cambiamenti dopo la conclusione dell’agente.

## 2. Stato del repository osservato

- Google Fonts è implementato in `src/google_fonts.rs` e `src/services/google_library.rs`; i comandi Tauri Google sono in `src-tauri/src/{backend.rs,commands.rs,lib.rs}`.
- I font installati sono registrati come `FontSource::{Local, Google}`. La tabella SQLite limita `source` a `local` e `google` (`src/db.rs`), perciò aggiungere provider richiede una migrazione compatibile con i database esistenti.
- Google scaricato viene scritto e registrato dal servizio remoto; l’attivazione Windows passa da `LocalLibraryService::activate`. Il nuovo flusso deve orchestrare ingest/scansione e attivazione senza duplicare l’implementazione Win32.
- La CSP Tauri attuale consente solo font self/data/blob e non abilita CDN remoti; la preview remota richiede un aggiornamento restrittivo di `font-src` e, se necessario, `connect-src`.
- `ui/src/App.tsx` è già modificato in working tree dall’altro agente: UI e test devono essere ispezionati nuovamente prima dell’integrazione.

## 3. Architettura

### 3.1 Modello remoto condiviso

Introdurre un modello serializzabile di catalogo remoto con ID e nome famiglia, provider (`google`, `fontsource-extra`, `fontshare`, `nerd-fonts`), categoria, varianti/pesi, URL di anteprima opzionale e strategia di download (`direct font files` oppure `ZIP archive`). Gli adapter convertono le risposte API nel modello comune; la UI non dipende dai JSON grezzi.

Il selettore mostra quattro cataloghi: Google Fonts (integrazione esistente), Fontsource Extra, Fontshare e Nerd Fonts. L’endpoint Fontsource include anche `type=google`: tali voci non devono creare un quinto catalogo né duplicare Google Fonts. Usare `type=other` per Fontsource Extra; mantenere la fonte Google esistente come catalogo Google visibile.

I provider devono avere identificatori stabili e namespaced per evitare collisioni. Estendere `FontSource` e la colonna `fonts.source` con valori distinti per i nuovi provider, mantenendo la lettura dei record esistenti. Aggiornare serializzazione, query del database, migrazioni e test di compatibilità.

### 3.2 Client e mapping

- **Fontsource**: `GET https://api.fontsource.org/v1/fonts`; usare le voci `type=other` per il catalogo Fontsource Extra e non esporre `type=google` come catalogo duplicato. Costruire la preview jsDelivr da `id`, `defSubset` e peso 400 o primo peso presente. Al download, interrogare `/v1/fonts/{id}`, raccogliere URL TTF da `variants[weight][style][defSubset].url.ttf` e scaricarli in parallelo con limiti di concorrenza e dimensione.
- **Fontshare**: `GET https://api.fontshare.com/v2/fonts?offset=0&limit=150`, leggere `response`, identificare con `slug`, mostrare `name`; scegliere preview regular non italic o il primo stile e normalizzare URL `//` a HTTPS. Scaricare lo ZIP da `/v2/fonts/download/{slug}`.
- **Nerd Fonts**: `GET https://api.github.com/repos/ryanoasis/nerd-fonts/releases/latest` con `User-Agent` applicativo; mantenere asset `.zip`, escludendo `FontPatcher.zip`, ricavare la famiglia dal nome asset e usare `browser_download_url`. Mostrare placeholder monospace, senza preview remota.
- Usare timeout, redirect HTTPS-only, limiti alla dimensione delle risposte e validazione di ID/URL prima del download. I cataloghi devono conservare il comportamento cache/fallback del provider Google ove compatibile; i problemi di un provider non devono disabilitare gli altri.

### 3.3 Download, estrazione e ingest

Il download deve produrre un report per famiglia, con file installati, fallimenti e motivi. Le risposte ZIP restano in un buffer RAM e sono lette tramite `Cursor<Vec<u8>>` e `ZipArchive`; nessun `.zip` viene scritto su disco.

Durante l’estrazione:
- Accettare solo estensioni `.ttf` e `.otf` case-insensitive, ignorare directory, file nascosti, `__MACOSX`, metadata, WOFF/WOFF2, CSS e licenze.
- Usare nomi/percorso interni verificati (es. `enclosed_name`), rifiutare traversal e path assoluti; non materializzare entry come symlink né usare nomi ZIP come destinazioni. Appiattire nell’area vault dedicata alla famiglia usando un nome file sicuro derivato dall’entry e validato dal parser font.
- Applicare limiti all’archivio, al numero di entry, alla dimensione decompressa per file e totale per evitare ZIP bomb e consumo eccessivo di memoria.
- Per Nerd Fonts ignorare nomi contenenti `Windows Compatible` e `NerdFontPropo`.
- Validare i bytes con il parser font esistente prima della scrittura atomica. Scannerizzare il solo sottoalbero familiare così che la libreria indicizzi i file; attivare gli elementi importati attraverso `LocalLibraryService`, senza chiamate Win32 nuove. Il report deve distinguere font salvati, indicizzati, attivati e falliti. Se l’attivazione di un elemento fallisce, mantenerlo disponibile nella libreria e riportare il fallimento invece di perdere il download.

### 3.4 Backend e contratti Tauri

Aggiungere servizi/client distinti o adapter coesi, senza concentrare tutta la logica in `GoogleLibraryService`. `AppBackend` orchestra provider, scanner locale e attivazione. Esporre comandi Tauri comuni per elenco/refresh del catalogo, preview metadata se necessaria e download/installazione; preservare i comandi Google esistenti finché la UI e i test non usano il nuovo contratto.

Estendere `StartupSnapshot` e i tipi TypeScript solo se serve al caricamento iniziale; evitare di scaricare cataloghi completi sincronicamente all’avvio. Gli errori devono essere isolati per provider e mostrati come stato caricamento/cache/fallimento specifico.

### 3.5 UI e sicurezza preview

La navigazione Cloud espone esattamente Google Fonts, Fontsource Extra, Fontshare e Nerd Fonts; non deve mostrare il `type=google` di Fontsource come catalogo duplicato.

Le card con preview URL caricano un `FontFace` soltanto quando entrano nel viewport, usando `IntersectionObserver`; rimuovere observer e risorse/font non più necessari. Nerd Fonts usa un fallback monospace. La CSP Tauri va aggiornata con origini HTTPS ristrette e verificate per i CDN effettivi; non consentire wildcard `https:` o URL forniti dall’API senza validare schema e host.

### 3.6 Impostazioni

Aggiungere una sezione Impostazioni nella navigazione desktop, mantenendo componenti e token esistenti. Deve permettere di:
- elencare cartelle font monitorate, aggiungerne una tramite dialogo nativo, avviare una nuova scansione, attivare/disattivare una cartella o rimuoverla dall’elenco senza cancellare i file originali;
- mostrare per ogni cartella ultimo scan, conteggio importati/aggiornati/errori e stato accessibile/non accessibile;
- mostrare percorsi dati, cache, vault locale e directory font installati Windows, con azioni di copia dove già supportate;
- mostrare stato dei cataloghi Google Fonts, Fontsource Extra, Fontshare e Nerd Fonts (cache presente, ultima riuscita, errore/aggiornamento) senza bloccare l’avvio dell’app.

La gestione cartelle deve riusare `scan_roots`; le cartelle disabilitate non vengono scandite automaticamente. Rimuovere una cartella rimuove solo la configurazione, non i file né le registrazioni dei font già indicizzati.

## 4. Test e criteri di accettazione

1. Test unitari dei mapping per tutti gli endpoint, incluse risposte incomplete, array vuoti, pesi/stili di fallback, URL `//` e asset esclusi.
2. Test ZIP in-memory per TTF/OTF validi, estensioni case-insensitive, cartelle annidate, appiattimento, file non-font, metadati, path traversal, symlink e filtri Nerd Fonts; test dei limiti per entry/archive e decompressione.
3. Test migrazione database da schema esistente con record `local`/`google`; ogni nuovo `FontSource` deve round-trip e comparire nelle query previste.
4. Test del flusso di ingest: file validato e scritto atomicamente, scanner che lo indicizza, attivazione instradata tramite servizio locale e report dei fallimenti parziali.
5. Test UI: selezione dei quattro cataloghi, stati caricamento/errore, fallback monospace Nerd Fonts e caricamento/rimozione lazy di `FontFace`.
6. Test live degli endpoint marcati ignorati o separati dai test deterministici, per non rendere la suite dipendente dalla rete.
7. Accettazione: `cargo test`, `npm run test`, `npm run build` e build Tauri superati; prova manuale di un font per ciascuna strategia di download su Windows; Graphify aggiornato dopo l’implementazione.

## 5. Verifica tecnica Context7 (2026-10-14)

| Punto | Esito Context7 | Impatto sulla specifica |
|---|---|---|
| Fontsource `GET /v1/fonts` e `/v1/fonts/{id}` | Verificati nei documenti API ufficiali: array con `id`, `family`, `weights`, `styles`, `defSubset`, `type`; dettaglio `variants` organizzato per peso/stile/subset e URL `ttf`/`woff`/`woff2`. Esempio ufficiale usa URL jsDelivr nel formato indicato. | Confermati endpoint, campi e costruzione della preview. La doc descrive `type` come stringa e mostra `google`, ma non enumera `other`: la presenza effettiva di `type=other` va confermata da una risposta live prima di codificare il filtro. |
| Fontshare v2 | Context7 non ha risolto una documentazione ufficiale per Fontshare API v2. | Endpoint, campi `response`/`slug`/`styles` e download ZIP derivano dal brief fornito, non sono certificati da questa verifica Context7. Aggiungere test contrattuali con una risposta live/campione prima di fissare il mapping. |
| Nerd Fonts via GitHub Releases | Documentazione REST conferma endpoint latest release e il download asset tramite `browser_download_url`. Documentazione GitHub conferma che tutte le richieste REST devono inviare un `User-Agent` valido e che senza header si riceve 403. | Confermati endpoint/URL asset e requisito obbligatorio User-Agent. Filtri `FontPatcher.zip`, `Windows Compatible` e `NerdFontPropo` sono regole di prodotto del brief, non prescrizioni GitHub. |
| ZIP in memoria e percorsi | zip-rs documenta lettura da `Cursor`/stream, `ZipArchive`, `ZipFile` come `Read` e `enclosed_name()` per rifiutare nomi assoluti/traversal. La documentazione dello stream reader avverte che le entry symlink possono apparire come file regolari. | Buffer RAM e controllo dei nomi sono supportati. Non usare estrazione generica su filesystem; per symlink verificare il metodo sul crate/versione selezionati o trattare ogni entry solo come bytes validati e testare l’input. Scegliere una versione compatibile con MSRV 1.80. |
| Tauri 2 | Docs ufficiali confermano `#[tauri::command]`, `generate_handler!`, stato gestito, `Result` e invocazione JS con argomenti camelCase. | Contratto di comandi proposto è coerente con Tauri 2. |
| React e CSP Tauri | React documenta `IntersectionObserver` in effect e cleanup con `disconnect()`. Docs Tauri confermano direttive CSP specifiche `font-src`/`connect-src` e raccomandano allowlist limitate ai domini fidati. Context7 non attesta gli host/CDN effettivi dei provider né la loro risposta CORS. | Preview lazy e cleanup sono conformi; prima di fissare la CSP provare gli URL di preview reali nel WebView e verificare CORS. Consentire solo host HTTPS necessari, senza wildcard. Se il provider non offre CORS compatibile, passare la preview dal backend anziché indebolire la CSP. |

### Fonti Context7

- Fontsource API: https://github.com/fontsource/fontsource/blob/main/website/docs/api/fonts.mdx e https://github.com/fontsource/fontsource/blob/main/website/docs/api/font-id.mdx
- GitHub REST API: https://docs.github.com/en/rest/using-the-rest-api/getting-started-with-the-rest-api e https://docs.github.com/en/rest/releases/assets
- zip-rs: https://github.com/zip-rs/zip2/blob/master/_autodocs/api-reference/ziparchive.md, https://github.com/zip-rs/zip2/blob/master/src/read/zipfile.rs, https://github.com/zip-rs/zip2/blob/master/_autodocs/api-reference/zipstreamreader.md
- Tauri: https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/develop/calling-rust.mdx e https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/security/csp.mdx
- React: https://github.com/reactjs/react.dev/blob/main/src/content/reference/react/useEffect.md

## 6. Riferimenti nel repository

- `src/models.rs`, `src/db.rs`, `src/paths.rs`
- `src/google_fonts.rs`, `src/services/google_library.rs`, `src/services/local_library.rs`, `src/font_scanner.rs`
- `src-tauri/src/backend.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json`
- `ui/src/types.ts`, `ui/src/App.tsx`, test UI e test Rust pertinenti
