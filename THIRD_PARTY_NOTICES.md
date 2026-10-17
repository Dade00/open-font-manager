# Dipendenze e contenuti di terze parti

La licenza MIT di Open Font Manager copre il codice originale del progetto, non tutti i file che l'app può leggere o scaricare.

## Dipendenze software

Le versioni risolte sono in `Cargo.lock` e `package-lock.json`; manifest, metadati e testi di licenza delle singole dipendenze restano autorevoli. Lo stack comprende Rust, Tauri, React, Vite, Tailwind CSS, daisyUI, SQLite/rusqlite, reqwest e Fontations, oltre alle dipendenze transitive. Questa pagina non è un inventario legale esaustivo.

Prima di distribuire binari, genera un inventario delle dipendenze e raccogli i testi di licenza e gli avvisi richiesti dalle versioni effettivamente incluse. La pubblicazione del sorgente non sostituisce questo controllo per gli artefatti binari.

## Font e cataloghi

- [Google Fonts](https://fonts.google.com/): consulta la licenza della famiglia scaricata; accesso al catalogo tramite il servizio terzo [google-webfonts-helper](https://github.com/majodev/google-webfonts-helper).
- [Fontsource](https://fontsource.org/): ogni famiglia conserva la propria licenza.
- [Fontshare](https://www.fontshare.com/): verifica la licenza specifica e le condizioni del servizio; non presumere che tutti i font siano open source.
- [Nerd Fonts](https://github.com/ryanoasis/nerd-fonts): i font patchati e i glifi possono avere avvisi e licenze distinti.

Il progetto non rivendica affiliazioni con questi servizi. Disponibilità e condizioni possono cambiare. Verifica i diritti prima di incorporare o ridistribuire font scaricati. I font di sistema impiegati nei test vengono letti sul dispositivo e non sono inclusi nel repository.

## Moodboard e altri contenuti

Immagini, note, link e font aggiunti dall'utente restano soggetti ai diritti dei rispettivi titolari. Importarli nell'app non concede diritti di pubblicazione o distribuzione.
