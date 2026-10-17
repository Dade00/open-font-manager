# Contribuire a Open Font Manager

Sono benvenuti bug report, test, documentazione e pull request. Italiano e inglese sono entrambi accettati. Prima di implementare una funzionalità ampia, apri una issue per discuterne l'ambito.

## Ambiente

Il target supportato è Windows 11 x64. Installa Git, Node.js 22.12+ con npm, Rust stable MSVC, Visual Studio Build Tools (C++ e Windows SDK) e WebView2 Runtime. Segui il [README](README.md) per clonare ed eseguire l'app. Usa `npm ci`, non aggiornare indiscriminatamente i lockfile.

`npm run dev` avvia soltanto il frontend: le operazioni native richiedono `npm run tauri -- dev`.

## Verifiche prima di una pull request

```powershell
cargo fmt --all -- --check
cargo test --workspace --locked --target x86_64-pc-windows-msvc
cargo clippy --workspace --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings
npm test -- --reporter=dot
npm run build
```

I test live che richiedono servizi esterni sono ignorati di default. Alcuni test Windows registrano temporaneamente una copia di un font per l'utente corrente e la rimuovono: eseguili con un account di sviluppo e non interromperli durante la registrazione. Non sono necessari privilegi amministrativi.

## Pull request

- Crea un branch dal `main` aggiornato e mantieni le modifiche circoscritte.
- Aggiungi test per bug e nuovi comportamenti; descrivi problema, soluzione e verifiche reali.
- Per modifiche UI allega screenshot e verifica tastiera, stato vuoto, errori e finestra stretta.
- Non includere font proprietari, database, immagini personali, credenziali, build o cache.
- Conserva la compatibilità dei dati locali o aggiungi migrazioni e relativi test.
- Per rete, ZIP, immagini e percorsi considera limiti di risorse, traversal e input non attendibili.

I contributi originali vengono forniti sotto la [licenza MIT](LICENSE) del progetto. Invia soltanto materiale che hai il diritto di contribuire e conserva gli avvisi delle dipendenze di terze parti. Non è richiesto un CLA.

## Grafo del codice

`graphify-out/` contiene una mappa generata per navigare il progetto, non è necessaria per compilare. Cache e percorsi macchina non vanno versionati. Dopo modifiche strutturali può essere aggiornata con Graphify; consulta il report per copertura e limitazioni. Non servono account o chiavi per l'estrazione AST.
