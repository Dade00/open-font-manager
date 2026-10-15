# Ponytail technical debt

Review items deferred after commit `c49f168`:

- [x] Query installati: `db::list_installed_fonts` applica i filtri SQL e legge in streaming, senza accumulare l'intera libreria nel backend. Conservati font Windows esterni, conteggio e paginazione; regressione `installed_stream_keeps_filters_pagination_and_live_external_state` passata. La verifica dei file resta proporzionale ai candidati filtrati.
- [x] Digest installer: `FontInstaller::install_names` calcola una sola volta il digest per nome file e valore del Registro; test installazione/disinstallazione passati.
- [x] Scheduler: FIFO a concorrenza fissa con deduplicazione, cancellazione dei job in attesa e cleanup. Suite `adaptivePreviewQueue.test.ts` passata; conservato il nome del file per evitare una rinomina superflua.
- [x] Fixture UI: condivisi font, snapshot e API; estratto `googleDisplayCatalog` dai setup ripetuti. Suite UI: 31 test passati.

The review’s suggestion to replace the startup installed-font count with `db::count_active_fonts` was not retained: that count omits fonts installed outside OFM, which the UI must include.
