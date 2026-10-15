# Coda evolutiva Open Font Manager

Aggiornamento: 2026-10-15.
Questo file è la coda durevole delle richieste, non una dichiarazione di implementazione completata. In questa attività vengono scritte solo specifiche e ricerca web. Nessun commit/push, installazione di nuovi provider o modifica della navigazione è incluso.

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

Stato: **in coda, specifica scritta; non implementata**.
Specifica: [Ricerca Globale](superpowers/specs/2026-10-15-global-font-search-design.md).
Dipende dalla tranche corrente e dalla verifica dei diritti dei provider.
Consegna: quattro tab concordate, ricerca cache-first federata, risultati progressivi, categorie comuni, identità/deduplicazione affidabile, stati provider, azione Installa e selezione locale esatta.

### Ispirazione: Abbinamenti e Moodboard

Stato: **in coda, specifica scritta; non implementata**.
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

## Passaggio all'esecuzione

L'utente ha richiesto di proseguire dopo il completamento del round di subagent corrente: prima verificare personalmente integrazione, suite, build e comportamento reale, correggendo eventuali problemi; quindi procedere in ordine di coda con il piano del modulo successivo. Non anticipare nuovi writer sui file ancora posseduti da un subagent. Mantenere le correzioni della tranche corrente separate dalle nuove funzionalità; DaFont e gli altri provider non verificati restano approfondimenti, non adapter automaticamente approvati.
