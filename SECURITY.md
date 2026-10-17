# Sicurezza

## Versioni supportate

Il progetto è in sviluppo iniziale: le correzioni vengono applicate al branch `main`. Non sono promessi backport o tempi di risposta garantiti. Non considerare questa versione sottoposta a un audit di sicurezza completo.

## Segnalare una vulnerabilità

Usa la [segnalazione privata di GitHub](https://github.com/Dade00/open-font-manager/security/advisories/new). Non pubblicare exploit, font malevoli, percorsi personali o dati sensibili nelle issue pubbliche.

Includi commit/versione, versione Windows, passaggi di riproduzione, impatto e, se possibile, un caso minimo privo di dati privati. Concorda la pubblicazione dei dettagli con il maintainer dopo la correzione.

## Superficie rilevante

L'app legge font, ZIP e immagini, conserva dati in SQLite, scarica contenuti da provider esterni e può installare font nel profilo Windows corrente. Parser, IPC, path traversal, decompressione e limiti di download sono aree particolarmente rilevanti. L'allowlist di un host non garantisce l'affidabilità di ogni contenuto remoto.
