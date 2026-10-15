# Piano di implementazione: provider cloud e Impostazioni

Spec: `docs/superpowers/specs/2026-10-14-cloud-font-providers-design.md`
Base: commit pushato `c49f168`.

## Vincoli globali

- Rust 2021, MSRV 1.80, Tauri 2, React 19/TypeScript; riusare reqwest, zip-rs, SQLite, scanner e installatore presenti.
- Non sovrascrivere o eliminare i tre elementi untracked già presenti (`NUL`, `graphify-out/`, spec); non fare commit o push.
- Google Fonts resta compatibile. La migrazione DB conserva i record `local`/`google` e i record delle vecchie versioni.
- Gli ZIP cloud vivono solo in RAM; validare host HTTPS, ID, limiti bytes/entry/estrazione e bytes dei font.
- UI coerente con tema `typebook`, tastiera/focus, stati loading/error/empty e cleanup di `FontFace`/observer.

## Task 1 - Contratto remoto e persistenza provider

Files: `src/models.rs`, `src/db.rs`, `src/lib.rs`, `tests/db_tests.rs`, `tests/model_tests.rs`.

1. Aggiungere `RemoteFontProvider`, `RemoteFontCatalogItem`, varianti, strategia download e report serializzabili. Provider UI: `google`, `fontsource-extra`, `fontshare`, `nerd-fonts`.
2. Estendere `FontSource` mantenendo esattamente la serializzazione di `local` e `google`; aggiungere i tre valori cloud.
3. Migrare lo schema v2 a v3 ricostruendo `fonts` con il nuovo CHECK, senza perdere riferimenti, collection items o flag. Aggiungere mapping round-trip esplicito dei provider.
4. Aggiungere operazioni DB per enable/disable/remove scan root; remove elimina solo la configurazione. Verificare che list/update esponga dati coerenti.

RED/GREEN: test migrazione v2 con record google/local e collezioni, round-trip per ogni nuovo source, root enable/disable/remove. `cargo test`.

Interfaces: produce `RemoteFontProvider` + source DB; Task 2/3 li consumano.

## Task 2 - Adapter catalogo e client cloud

Files: `src/cloud_fonts.rs` (nuovo), `src/lib.rs`, test `tests/cloud_fonts_tests.rs` (nuovo); dipendenze solo se già presenti non bastano.

1. Implementare parser puri per le tre risposte API e conversione Google al contratto remoto comune.
2. Client HTTPS con timeout, `User-Agent` per GitHub, redirect HTTPS-only, limiti JSON/download e validazione hostname per preview/download.
3. Fontsource: caricare `type=other`, default subset/peso; dettaglio variants e TTF. Fontshare: supportare sia `fonts` (campo osservato live) sia `response` previsto dal brief, `slug`, regular non italic fallback, URL `//` normalizzato. Nerd Fonts: ZIP release assets soltanto, escludere `FontPatcher.zip`; tenere conto di archivi ZIP validi oltre 250 MiB senza superare un limite RAM esplicito.
4. Isolare errori per provider; refresh esplicito, cache/metadati provider e snapshot stale senza richiesta di rete bloccante all'avvio.

RED/GREEN: fixture JSON (campi mancanti, array vuoti, fallbacks, URL invalidi e asset esclusi), limiti e host validation. Test live separati/ignored. `cargo test`.

Interfaces: produce catalog item e API provider; Task 3 consuma id/strategy/download target, Task 4 li espone.

## Task 3 - Download, import atomico e attivazione

Files: `src/services/zip_import.rs`, nuovo servizio cloud sotto `src/services/`, `src/services.rs`, `src/services/local_library.rs`, `src/db.rs`, test ZIP/install/scanner pertinenti.

1. Aggiungere ZIP in-memory con `Cursor<Vec<u8>>`; filtrare solo TTF/OTF, nomi sicuri, no symlink/traversal/metadata, limiti decompressione, filtri Nerd Fonts. Appiattire in directory famiglia e validare prima della scrittura atomica.
2. Fontsource: scaricare in parallelo i TTF con limite concorrenza e dimensioni. Fontshare/Nerd: ZIP interamente in RAM, mai file ZIP temporaneo.
3. Scansionare i file nel vault attraverso il parser/scanner esistente, preservando provider/family/variant nell'indice e nell'ID stabile.
4. Attivare ogni font salvato tramite il flusso `FontInstaller` e registrare stato; report distingue salvati, indicizzati, attivati e fallimenti parziali.

RED/GREEN: ZIP validi/non-font/traversal/duplicati/symlink/limiti; ingest con errori parziali; nessun file ZIP temporaneo. `cargo test`.

Interfaces: consuma provider API/DB da Task 1-2; produce report per Tauri Task 4.

## Task 4 - Comandi Tauri e Impostazioni backend

Files: `src-tauri/src/backend.rs`, `commands.rs`, `lib.rs`, `tauri.conf.json`, `src/db.rs`, test `src-tauri/tests/backend_contract.rs`.

1. Esporre comandi comuni per catalogo (provider esplicito) e installazione famiglia; preservare comandi Google finché tutti i test/consumer non usano l'API nuova.
2. Esporre SettingsSnapshot: scan roots, directory dati/cache/vault/installazione, stato/errori provider; non caricare cataloghi di rete allo startup.
3. Esporre CRUD sicuro delle scan roots e rescan. Validare directory, deduplicare path canonici e non cancellare font/files durante remove.
4. Aggiornare CSP soltanto con origin CDN HTTPS realmente usate; niente wildcard. Se preview cross-origin/CORS non è affidabile, servire bytes preview via comando Tauri invece di allargare CSP.

RED/GREEN: contratti comandi, validazione path, errore provider isolato, snapshot e gestione roots. `cargo test --workspace`.

## Task 5 - UI cataloghi, preview lazy e Impostazioni

Files: `ui/src/App.tsx`, `ui/src/api.ts`, `ui/src/types.ts`, `ui/src/styles.css`, `ui/src/App.test.tsx` e test nuovi se necessario.

1. Aggiungere selettore per Google Fonts, Fontsource Extra, Fontshare e Nerd Fonts con un'unica card remote; non mostrare doppioni Google da Fontsource.
2. `IntersectionObserver` + `FontFace` solo per righe visibili con preview valida; cleanup observer/font-face/object URL e fallback monospace Nerd Fonts.
3. Download one-click genera report, aggiorna libreria e mostra chiaramente parziali/attivati.
4. Aggiungere sezione Impostazioni nella nav. Riutilizzare dialog nativo e `scan_roots` per aggiungere cartelle, rescan, enable/disable e remove; mostrare ultimo scan/errori e path completi dei dati/cache/vault/installazione.
5. Mostrare cache/ultima riuscita/errore per provider con azione di refresh; evitare dipendenze nuove e mantenere token typebook.

RED/GREEN: test UI provider switching, no duplicate catalog, loading/error/empty, lazy preview cleanup, Nerd fallback e gestioni roots/settings. `npm run test` poi `npm run build`.

## Task 6 - Verifica completa e chiusura

1. `cargo test --workspace`, `npm run test`, `npm run build`, `cargo tauri build`/check disponibile; leggere esiti integrali.
2. Eseguire test live limitati sugli endpoint; distinguere problemi provider/CORS dai test deterministici.
3. Ispezionare UI renderizzata e tastiera/focus, stati rete, CSP e layout settings; correggere difetti bloccanti con test prima.
4. `git diff --check`, `graphify update .`, verificare che gli untracked preesistenti restino invariati e il remote non sia stato alterato.

## Decisioni di ambito

- API live 2026-10-14: Fontsource include entrambe le voci `type=google` e `type=other`; Fontshare v2 usa `fonts` e `count`, con `styles[].file` su `cdn.fontshare.com`; Nerd Fonts latest include ZIP e TAR.XZ, alcuni asset ZIP superano 250 MiB. Il parser deve gestire entrambi i formati Fontshare; scaricare solo ZIP.
- “Altri sistemi/info” nelle Impostazioni è interpretato come percorsi locali e stato dei provider, non integrazione di account/servizi autenticati: i cataloghi richiesti sono pubblici e senza chiavi.
- Rimuovere una scan root non disinstalla né elimina font: l'azione cambia solo la configurazione come richiesto dalla specifica.
