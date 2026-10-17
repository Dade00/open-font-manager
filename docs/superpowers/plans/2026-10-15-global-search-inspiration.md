# Ricerca Globale e Ispirazione Implementation Plan

> **For agentic workers:** usare esecuzione per tranche verificabili e ownership esclusiva dei file. Prima funzionamento reale, poi regressioni, come richiesto dall'utente.

**Goal:** integrare Ricerca Globale, quattro sezioni primarie, editor abbinamenti e moodboard persistenti.
**Architecture:** riutilizzare gli adapter FontManagerApi; ricerca cache-first frontend senza nuove chiamate arbitrarie. Persistenza Ispirazione nel SQLite esistente tramite servizio Rust dedicato e IPC Tauri, asset immagine gestiti localmente. Componenti nuovi indipendenti; integrazione App e verifica finale del parent.
**Tech Stack:** React 19, TypeScript, DaisyUI 5/typebook, Tauri 2, Rust 2021, rusqlite.
**Spec:** ../specs/2026-10-15-global-font-search-design.md e ../specs/2026-10-15-inspiration-design.md.

## Global Constraints
- Commit parziale iniziale de4d142 già creato; niente push, NUL e graphify-out esclusi.
- Nessuno scraping o nuovo provider non verificato; esattamente i quattro provider correnti.
- Variable desktop originali Google/Fontsource resta limite esplicito, non prerequisito per bloccare le funzioni richieste ora dall'utente.
- CSP senza wildcard; niente HTML/SVG attivo, URL arbitrari scaricati o chiavi.
- UI italiana, categorie condivise, accessibilità tastiera, nessuna numerazione ornamentale.
- Image import: PNG/JPEG/WebP, massimo 20 MiB e 24 milioni pixel, decoder reale con limiti; copia atomica; asset content-addressed.

## Review Focus
- Risposte provider tardive non devono sovrascrivere query/filtri/selezione nuovi.
- Conflitto di revisione persistente non deve diventare falso salvataggio.
- Percorsi asset e link devono restare nei confini dell'archivio/app e schemi consentiti.
- Cancellazione board/composizione non deve cancellare immagini condivise/font Windows.
- Font non risolto resta riferito e dichiarato indisponibile, non sostituito di nascosto.

## Contratto Ispirazione (condiviso)
La definizione TypeScript autorevole sarà ui/src/inspirationTypes.ts, scritta prima dei worker. Snapshot {version:1, revision:number, boards:Board[], compositions:Composition[]}. Save usa expectedRevision e restituisce lo snapshot con revisione incrementata, transazione ottimistica. IPC: load_inspiration(), save_inspiration({document,expectedRevision}), import_inspiration_image({path}), get_inspiration_asset({assetId}), open_inspiration_link({url}). Gli asset non contengono path dal renderer, solo ID SHA256 e metadati decoder. Nessun salvataggio in localStorage come surrogato.

## Tranche
- [x] Commit della tranche cloud verificata, limiti annotati.
- [x] Servizio Rust Ispirazione + IPC, contratto JSON frontend verificato, riapertura/CAS/sicurezza/asset condivisi: 8 test passati e suite Rust workspace verde.
- [x] Ricerca Globale con quattro adapter, cache/provider indipendenti, filtri e paginazione, preview bounded e installazione; controlli testo/dimensione e stato per variante Google.
- [x] Ispirazione UI: ruoli/font/parametri, salvataggio, duplicazione, cinque tipi board, riordino tastiera/drag, immagini e preview. Regressione locale oltre100/provider guasto/campione solo corpo/stabilità durante digitazione passata.
- [x] Navigazione a quattro sezioni e collegamenti font verso Ispirazione, preferiti/collezioni interni a Locali, Impostazioni separata e styling responsive.
- [x] 51 test UI passati dopo revisione UX; build web/Tauri release riuscite. Smoke reale anche sull'eseguibile release con due font, salvataggio/rilettura composizione, cinque tipi board, sorgente immagine eliminata, nessun errore JS/overflow nelle viewport provate. Immagine importata tramite IPC reale; file picker nativo non automatizzato.
- [x] FEATURE-QUEUE riallineata e Graphify codice aggiornato dopo tutti i writer con backup; semantica documenti e naming community restano esplicitamente non certificati.

## Revisione richiesta dall'utente dopo il primo smoke

- [x] Riprodotto e corretto il difetto Grid dei cataloghi: selettore alto 823 px, elenco compresso a 24 px; aggiunta la quarta riga esplicita. Regressione reale ripetibile in `scripts/check_catalog_layout.py`.
- [x] Query vuota già popolata, ordine sempre alfabetico, raggruppamento visuale con fonti selezionabili. Tutte le pagine reali confrontate alle cache: 2249 voci, 2202 famiglie, 47 gruppi con più fonti, nessuna omissione o duplicazione.
- [x] Ispirazione guidata e contenuti prima dei form. Su richiesta esplicita aggiuntiva, editor Abbinamenti inline in sidebar tramite subagent, non modal; preview principale separata, campi avanzati progressivi.
- [x] Integrate regressioni parent su prima modifica dirty, annullamento/conferma cambio abbinamento, seed con sidebar aperta, nome moodboard e nota contestuale, conservazione bozza al cambio board.
- [x] Verificata la release `http://tauri.localhost/` con asset `index-C7XOk8be.js`, poi cleanup dei soli contenuti di prova. Il vecchio smoke modale fallito è sostituito dal nuovo smoke inline riuscito, non dichiarato retroattivamente verde.
