# Coda evolutiva Open Font Manager

Aggiornamento: 2026-10-15.
Questo file distingue implementazioni verificate ed estensioni in valutazione. Checkpoint cloud: commit `de4d142`; Ricerca Globale e Ispirazione sono integrate nella working tree, senza un secondo commit o push. Nessun nuovo provider aggiunto.

## Direzione approvata

Navigazione primaria, in ordine: **Locali**, **Ricerca Globale**, **Ispirazione**, **Cataloghi vari**. I cataloghi individuali restano accessibili dentro Cataloghi vari; preferiti e collezioni sono strumenti di Locali; Impostazioni resta accesso di servizio.

## Prima della coda: tranche corrente da chiudere

- [x] Allineamento visuale cataloghi/Locali/Google Fonts, categorie radio con glifi, rimozione numeri decorativi; render delle viste e controlli responsive verificati.
- [x] Installa in un click: smoke reale Fontsource Adwaita Mono, quattro registrazioni Windows/file verificati, selezione locale e stato remoto Installato; regressione UI sul font oltre la prima pagina passata. Google download+attivazione verificato nella suite UI, non con un'installazione live distinta.
- [ ] Supporto variable come file originale, senza generare istanze statiche; test parser, persistenza e download effettivi.
- [ ] Chiudere review frontend, debiti tecnici, suite/build/render e aggiornamento Graphify preservando output precedente. Nessuna voce è completata soltanto in base al report di un subagent.

Verifiche integrate: suite Rust e UI verdi (31 test UI), build web e Tauri release riuscite; corretto anche un errore di migrazione dello schema intermedio che impediva l'avvio sul database esistente. Debiti Ponytail originari risolti. Graphify codice aggiornato con backup, semantica documenti e naming community non ancora certificati. Variable: parser/persistenza/file originali verificati con Bahnschrift; download originali variable Google/Fontsource ancora aperto, senza etichettare TTF statici come variable.

Approfondimento variable: il round di ricerca aggiuntivo non ha modificato codice. Il subagent riporta WOFF2 con fvar ottenuti da Google CSS v2 e CDN Fontsource; queste prove riguardano font web, non originali TTF/OTF installabili su Windows. Non chiudono il requisito desktop e non autorizzano conversioni WOFF2 o endpoint dedotti per convenzione. Per Google rimangono da valutare file TTF originali dal repository ufficiale o Developer API con chiave opzionale; per Fontsource Extra serve un contratto verificato verso gli originali desktop. Non aggiungere chiavi/configurazione o nuovi provider automaticamente.

## Coda ordinata

### Ricerca Globale e nuova navigazione

Stato: **implementata nella working tree**. Revisione UX: apertura già popolata, ordine alfabetico e raggruppamento visuale per nome famiglia; fonti/varianti/azioni e stato installato restano distinti e selezionabili. Quattro sezioni primarie, cataloghi individuali nel selettore interno, provider indipendenti, filtri, paginazione, campioni e installazione con il backend esistente. Configurazioni di nuovi provider/credenziali ancora future.
Specifica: [Ricerca Globale](superpowers/specs/2026-10-15-global-font-search-design.md).
Dipende dalla tranche corrente e dalla verifica dei diritti dei provider.
Consegna: quattro tab concordate, ricerca cache-first federata, risultati progressivi, categorie comuni, identità/deduplicazione affidabile, stati provider, azione Installa e selezione locale esatta.

### Ispirazione: Abbinamenti e Moodboard

Stato: **implementata e verificata nella working tree**. Editor titolo/sottotitolo/corpo, preview e parametri, salvataggio/duplicazione, board con note/link/immagini/font/composizioni e riordino tastiera/drag. Persistenza nel SQLite esistente con revisione ottimistica; immagini validate e copiate nell'archivio locale.
Specifica: [Ispirazione](superpowers/specs/2026-10-15-inspiration-design.md).
Dipende dal contratto font condiviso della Ricerca Globale.
Consegna a tranche: editor abbinamenti persistente, poi board a griglia con immagini locali copiate, link, note, font e composizioni collegate. Tutto locale, senza canvas infinita, scraping o AI nel primo rilascio.

### Estensioni cataloghi e API configurabili

Stato: **ricerca completata, integrazioni in valutazione; nessun nuovo adapter approvato automaticamente**.
Ricerca: [Cataloghi/API](superpowers/specs/2026-10-15-font-catalog-research.md).

- [ ] Gate Fontshare FFL: verificare diritti di integrazione/preview/download prima della distribuzione; separare freeware FFL e OFL.
- [ ] Valutare Google Developer API opzionale con chiave utente, senza duplicare Google Fonts.
- [ ] Spike Font Squirrel: API ufficiale storica documentata ma probe HTTPS oggi senza JSON; verificare disponibilità e licenze prima di aggiungerla.
- [ ] Valutare Font Library e Velvetyne tramite fonti/repository autorizzati; nessuna API stabile verificata.
- [ ] Valutare Monotype con account/contratto API adeguato, non come catalogo free.
- [ ] Eventuale manifest custom JSON versionato con sicurezza host/credenziali, dopo contratto e piano dedicati.
- [ ] DaFont: riferimento esterno/import manuale fino a permesso/API ufficiale; nessuno scraping in coda come implementazione approvata. FAQ italiana aggiuntiva verificata: nessun contratto API o permesso d'integrazione aggiuntivo; incongruenza di traduzione annotata nella ricerca.
- [ ] Bunny e Adobe: solo opzioni future specifiche, non fonti automatiche di nuovi TTF desktop.

## Revisione UX del 2026-10-15

- Riprodotto nel WebView reale il difetto dei cataloghi: il selettore ereditava la riga Grid `1fr` e comprimeva l'elenco a 24 px. Corrette le quattro righe della shell. Tutti i cataloghi verificati a 1440 e 640 px con righe realmente visibili, non soltanto assenza di overflow. Regressione ripetibile: `uv run scripts/check_catalog_layout.py --output <cartella>` con Vite e debug WebView2 su porta 9223.
- Ricerca Globale reale: raccolte tutte le pagine e confrontate con le cache dei quattro provider. 2249 voci, 2202 famiglie uniche, 47 gruppi con più fonti; ordine alfabetico e completezza verificati anche a query vuota. Provato il cambio fonte per Montserrat.
- Ispirazione: percorsi Abbinamenti/Moodboard espliciti, contenuti prima dei form, menu unico di aggiunta, nome moodboard prima della creazione, bozze protette. **Editor Abbinamenti inline nella sidebar destra, con anteprima live a sinistra; impilato nella stessa pagina su finestre strette.** Bozza iniziale pulita, prima modifica dirty, guardia al cambio abbinamento e seed dai cataloghi coperti da regressioni. Nessun dialog per creare/modificare; restano le conferme distruttive.
- Verifica finale personale: **51 test UI passati**, build web e Tauri release riuscite, asset release `index-C7XOk8be.js` osservato su `http://tauri.localhost/`. Ripetuti sulla release tutti i cataloghi a 1440/640 px, tutte le pagine della Ricerca Globale e workflow Ispirazione: due font reali, salvataggio/rilettura, board con cinque tipi, immagine importata via IPC reale e ancora leggibile dopo eliminazione sorgente, modifica nota e reload. Screenshot e risultati in scratch `ofm-workflow-verification`, `ofm-sidebar-responsive`, `ofm-release-catalog-regression` e `ofm-global-verification.json`. Contenuti di prova rimossi e assenza riletta dal backend. Il file picker nativo non è stato automatizzato.
- Suite Rust e formato passati. Clippy completato con due suggerimenti preesistenti in `cloud_fonts.rs`; nessun refactor collaterale. Graphify codice aggiornato con backup verificato; semantica documenti/CSS e naming community non certificati. Nessun nuovo commit o push.

## Verifica integrazione iniziale, prima della revisione UX

- Suite UI: **45 test passati**; suite Rust workspace passata, inclusi **8 test Ispirazione**. Test rete ordinari ignorati come previsto.
- Build web e Tauri release `target/release/open-font-manager-app.exe` riuscite.
- Smoke reale WebView2/Tauri: composizione e board salvate e rilette dopo reload, cinque tipi elemento, immagine disponibile dopo eliminazione della sorgente e ricerca con cataloghi reali. Nessun errore JavaScript osservato.
- Quattro sezioni provate a 1440/1024/640/390px senza overflow orizzontale. Screenshot e risultati in scratch Hermes `ofm-new-features-smoke`.
- Limiti: controlli assi variable non esposti senza metadati assi reali; originali desktop Google/Fontsource ancora aperti. Asset immagine orfani conservati per non perdere import pendenti o condivisi, garbage collection sicura futura. Fontshare FFL resta un gate prima della distribuzione. Graphify codice aggiornato con backup; semantica documenti/naming community non certificati.

## Passaggio all'esecuzione

L'utente ha richiesto di proseguire dopo il completamento del round di subagent corrente: prima verificare personalmente integrazione, suite, build e comportamento reale, correggendo eventuali problemi; quindi procedere in ordine di coda con il piano del modulo successivo. Non anticipare nuovi writer sui file ancora posseduti da un subagent. Mantenere le correzioni della tranche corrente separate dalle nuove funzionalità; DaFont e gli altri provider non verificati restano approfondimenti, non adapter automaticamente approvati.
