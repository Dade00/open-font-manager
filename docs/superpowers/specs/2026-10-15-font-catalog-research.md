# Ricerca aggiuntiva: cataloghi font e API configurabili

Verifica: 2026-10-15, ricerca web e lettura delle fonti ufficiali. Questo documento è una valutazione di fattibilità, non una certificazione legale né un elenco di integrazioni già funzionanti.

## Sintesi delle decisioni

- Priorità a cataloghi con API/documentazione o repository ufficiali e licenze verificabili per famiglia.
- Font Squirrel è il candidato aggiuntivo più concreto per ampiezza oltre Google, ma l'API storica NON è stata verificata funzionante oggi.
- Font Library e Velvetyne sono piste di contenuto libero, non adapter pronti.
- Bunny Fonts è soprattutto una sorgente web alternativa per font già presenti nell'ecosistema Google, non automaticamente nuove famiglie desktop.
- Google Developer API con chiave utente è una modalità opzionale utile, non un quinto catalogo Google.
- Monotype è una possibile integrazione commerciale con contratto e credenziali, non un catalogo gratuito generalista.
- DaFont: collegamento al sito/importazione manuale come percorso prudente; nessun adapter di scraping/installazione automatica approvato.

## Confronto

| Fonte | Accesso verificato | Gratuità e vincoli | Decisione proposta |
|---|---|---|---|
| Font Squirrel | Annuncio ufficiale API JSON del 2010: classifications, fontlist, familyinfo e fontfacekit. Test HTTPS di classifications e fontlist/all: HTTP 202 con corpo vuoto e content-type HTML, non JSON. | FAQ: font scelti per licenza desktop commerciale gratuita, ma verificare ciascuna licenza; embedding web/app distinto; alcuni download solo dal sito dell'autore. Kit storico descritto come subset MacRoman, non copia completa garantita. | Spike tecnico/licenze in coda. Non dichiarare adapter operativo; niente aggiramento dei controlli del sito. Non installare un subset al posto del file completo senza segnalarlo. |
| Font Library | Pagina ufficiale licenze letta via HTTP 200. Nessuna API pubblica stabile documentata individuata in questa ricerca. | Supporta uso anche commerciale, con molte licenze diverse: OFL, Apache, GPL con/senza eccezioni e altre. Non tutte equivalenti. | Candidato successivo via manifest/repository autorizzato, o link al sito. Prima verificare contratto di accesso e metadati. |
| Velvetyne | Risultati indicizzati del sito ufficiale dichiarano licenze aperte; richieste dirette About/FAQ hanno restituito HTTP 500. Nessuna API stabile verificata. | Foundry di font liberi, con licenza da verificare sul pacchetto/repository di ogni famiglia. | Fonte interessante per ispirazione e famiglie distintive; verificare repository ufficiali e manifest prima di proporre download automatici. |
| Bunny Fonts | Pagina About ufficiale: compatibilità con Google Fonts CSS v1, cambiando hostname. Catalogo web raggiungibile. | Distribuzione web; questa prova non dimostra un endpoint TTF/OTF desktop completo né un catalogo aggiuntivo distinto. | Possibile alternativa di serving web; bassa priorità per aumentare i risultati globali. Non convertire automaticamente WOFF/WOFF2 in font desktop. |
| Google Fonts Developer API | Documentazione ufficiale REST JSON, richiede API key; espone categorie, file e capability VF con assi. Non testata con una chiave utente. | Font con licenza propria; chiave API identifica il client e non sostituisce la licenza. | Adapter metadati Google opzionale configurabile in Impostazioni; riusa identità Google esistente ed evita duplicati. Nessuna chiave obbligatoria per gli altri provider. |
| Monotype APIs | Pagine ufficiali prodotto/supporto individuate e pagina prodotto raggiunta HTTP 200 nel primo probe; una lettura successiva ha restituito 403. Documenta ricerca/preview/download tramite prodotti API. Nessuna sessione autenticata o prova download. | Offerta commerciale, diritti e disponibilità dipendono dal contratto. Non assumere che un abbonamento desktop qualunque abiliti API o ridistribuzione. | Tranche opzionale dopo verifica account/contratto, schema auth e diritti di download/attivazione. Non promettere supporto con il semplice inserimento di un token. |
| Adobe Fonts | FAQ ufficiali licenze: accesso legato ad abbonamento Creative Cloud attivo, aggiunta desktop tramite app Creative Cloud. | Non è una raccolta di file desktop liberamente redistribuibili; web kit/API non equivalgono a download TTF per installer indipendente. | Al massimo apertura/attivazione attraverso percorso ufficiale dopo integrazione dedicata. Nessun download generico dei font Adobe nel vault OFM. |
| DaFont | FAQ ufficiale e discussione API del 2012 lette. Nessuna API pubblica ufficiale documentata individuata; il forum storico dice che l'integrazione proposta non era possibile. Non è prova definitiva della politica attuale. | Freeware, shareware, demo e pubblico dominio; l'etichetta sopra Download è solo indicativa. Mancanza di autore/licenza NON significa gratis. | Collegamento esterno e import manuale consentito all'utente, con verifica licenza. Ricerca federata automatica solo dopo accordo/documentazione ufficiale attuale. Niente scraping basato sulla sola accessibilità delle pagine. |

## DaFont: risposta operativa

### Controllo della FAQ italiana fornita dall'utente

Fonte aggiuntiva: https://www.dafont.com/it/faq.php, verificata sul testo allegato dall'utente. Non aggiunge endpoint/API, condizioni per client di terze parti o autorizzazioni all'aggregazione automatica. Conferma che l'indicazione sopra Download è orientativa e rimanda al readme nello ZIP o all'autore per la licenza effettiva. La mancanza di autore/licenza non rende il font libero da licenza.

La frase italiana finale «non sono né freeware, né shareware, né versioni demo né di dominio pubblico» è in contraddizione con il paragrafo italiano precedente, che parla di freeware/shareware/demo, e con l'inglese «are either freeware, shareware, demo versions or public domain». Segnalata come incongruenza della traduzione, non interpretata come nuova restrizione generale. La guida tecnica contiene anche riferimenti a Windows XP/MSN e una stima storica di 1000 font: non usarla come limite tecnico attuale del prodotto.

Esito: nessun nuovo elemento che sblocchi l'integrazione automatica; resta in coda come approfondimento/API ufficiale oppure collegamento esterno e importazione manuale.

Usabile come fonte di scoperta e riferimento esterno, non va trattato come API free commerciale. L'utente può aprire il sito, ottenere il font e importare il file/ZIP attraverso il flusso esistente, rispettando la licenza dell'autore. Non inserire tutti i risultati di DaFont fra i font commercialmente gratuiti. Non assumere che l'etichetta 100% Free conceda automaticamente distribuzione nell'app, embedding web o modifica del file.

Non è stata trovata una prova ufficiale attuale che autorizzi l'aggregazione automatizzata nel prodotto. La discussione del 2012 è evidenza storica, non un parere legale o un contratto vigente. Per un'integrazione diretta contattare il sito e definire limiti, API e diritti prima di scrivere uno scraper.

## Attenzione: provider già presente Fontshare

La licenza ITF Free Font License v2.0 del 17 agosto 2026 è stata letta integralmente. Consente all'utilizzatore finale download, installazione e uso personale/commerciale, ma vieta redistribuzione tramite font library/download service/app/piattaforma e fornitura del font come opzione per utenti terzi di strumenti di creazione. L'API è una convenience e può essere limitata o sospesa.

Questo NON dimostra da solo che il download diretto, effettuato dal computer dell'utente finale dal sito ufficiale senza proxy condiviso, sia vietato. Impone però un gate di verifica prima di distribuire OFM con catalogo/preview/installazione automatica di font FFL: distinguere licenza dei font, autorizzazione accesso API e ruolo dell'utente finale. I font OFL hanno diritti differenti. Non spacciare tutti i font Fontshare per open source e non offrire conversione/subsetting di quelli FFL.

Registrare la decisione per adapter/famiglia e non modificare silenziosamente l'attuale comportamento durante questa attività documentale. Non distribuire font preinclusi o un mirror di archivio; le board conservano riferimenti e non font riutilizzabili da terzi.

## API configurabili dall'utente finale

Separare tre livelli:

1. Adapter supportato con credenziale opzionale: prima scelta per Google Developer API e, dopo verifica commerciale, Monotype.
2. Catalogo custom con manifest JSON versionato OFM: eventuale estensione per foundry/team che controllano il proprio endpoint e i diritti dei font.
3. URL arbitrario con API key: NON supportato come promessa universale. Endpoint, auth, paginazione, ricerca, schema e diritti differiscono fra vendor.

Configurare provider/host, attivazione nella ricerca globale e test connessione. Salvare segreti nell'archivio sicuro Windows; mai nelle spec, board, query di ricerca o log. Backend con allowlist HTTPS per adapter, redirect controllati, timeout/limiti e nessuna wildcard CSP. Non eseguire codice/plugin scaricato dall'endpoint.

Ogni adapter dichiara capability separate: ricercabile, preview, download desktop, attivazione esterna, variable e stato licenza. Dove non c'è un file desktop autorizzato l'azione è Apri sul sito, non Installa. Le API a pagamento restano disabilitate senza credenziali/diritti appropriati.

## Ordine consigliato per estendere le fonti

- Consolidare i quattro provider esistenti e il gate Fontshare.
- Consentire Google Developer API come sorgente metadati opzionale.
- Spike Font Squirrel: contratto HTTPS/JSON, condizioni attuali, file desktop completi, licenze e rate limit. Se fallisce, mantenere solo link esterno.
- Valutare manifest/repository autorizzati per Font Library e Velvetyne, senza promettere una API inesistente.
- API commerciali e manifest custom in una tranche separata.
- DaFont e Adobe come riferimenti esterni fino a un'integrazione ufficialmente supportata.

## Fonti primarie

- Font Squirrel API: https://www.fontsquirrel.com/blog/2010/12/the-font-squirrel-api
- Font Squirrel FAQ/licenze e download offsite: https://www.fontsquirrel.com/faq
- Probe HTTPS: https://www.fontsquirrel.com/api/classifications e https://www.fontsquirrel.com/api/fontlist/all
- Font Library licenze: https://fontlibrary.org/en/guidebook/supported_licenses
- Velvetyne: https://velvetyne.fr/about/ e https://velvetyne.fr/about/faq/ (pagine non leggibili direttamente durante la verifica, HTTP 500)
- Bunny: https://fonts.bunny.net/about e https://fonts.bunny.net/
- Google Developer API: https://developers.google.com/fonts/docs/developer_api
- Fontsource API: https://fontsource.org/docs/api/introduction (read-only; fair use e rate limiting documentati)
- Monotype: https://www.monotype.com/fonts-api e https://support.monotype.com/en/articles/8979169-overview-of-monotype-apis
- Adobe: https://helpx.adobe.com/fonts/using/font-licensing.html
- DaFont: https://www.dafont.com/faq.php e https://www.dafont.com/forum/read/42997/api-integration
- Fontshare: https://www.fontshare.com/licenses/itf-ffl e https://www.fontshare.com/faq

## Limiti della verifica

Nessun account creato, credenziale utilizzata o installazione prodotto eseguita. Nessun test autenticato Monotype/Adobe/Google Developer API. Risposte vuote, 403 e 500 non sono stati interpretati come cataloghi funzionanti. Le fonti storiche sono marcate come tali. Rivalidare condizioni e endpoint prima di implementare ciascun nuovo adapter.
