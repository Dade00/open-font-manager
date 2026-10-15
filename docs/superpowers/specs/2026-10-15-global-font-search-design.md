# Ricerca Globale e navigazione unificata

Stato: specifica scritta, in coda; non implementare in questa attività di documentazione.
Origine: richiesta utente di ricerca unica fra cataloghi e navigazione esplicita; proposta cache-first approvata come direzione. La revisione di questa specifica precede il piano esecutivo.

## Obiettivo

Cercare una famiglia una sola volta, confrontarla con campioni reali e installarla senza conoscere provider o formato di distribuzione. Riutilizzare cataloghi, cache, preview e installer esistenti; non costruire un motore di ricerca web generico.

## Navigazione primaria

Le tab, nell'ordine e con le etichette richieste, sono:

- **Locali**: libreria locale, filtri, preferiti e collezioni come strumenti interni, non tab primarie aggiuntive.
- **Ricerca Globale**: ricerca federata nei cataloghi abilitati.
- **Ispirazione**: abbinamenti e moodboard, specifica separata.
- **Cataloghi vari**: contenitore dei cataloghi individuali, con selettore interno Google Fonts, Fontsource Extra, Fontshare e Nerd Fonts, più eventuali adapter approvati.

Impostazioni è un'azione di servizio fuori dalle quattro tab, non una quinta tab primaria. Nessuna tab per funzionalità ancora in coda prima della loro effettiva implementazione. Conservare la navigazione esistente fino al rilascio del modulo, senza anticipare controlli morti.

## Prerequisiti bloccanti

Completare e verificare la tranche già richiesta: cataloghi allineati a Locali/Google Fonts, categorie canoniche con radio e glifo, installazione download+attivazione in un click, stato installato sincronizzato, ritorno a Locali con selezione esatta, supporto dei file variable originali e rimozione numerazioni decorative. Non considerare queste condizioni soddisfatte per il solo passaggio dei test preesistenti.

## Esperienza di ricerca

- Ricerca testuale comune su famiglia, stile e metadati effettivamente disponibili. Etichetta visibile del campo, non soltanto placeholder.
- Filtri categoria identici ovunque: tutte, sans serif, serif, display, scrittura/handwriting, monospace, non classificati. I valori canonici restano distinti dai nomi localizzati.
- Selettore provider multiplo; filtri installati/non installati e variable quando il dato è disponibile. Dato variable sconosciuto non equivale a statico.
- Un solo elenco a campioni, con stessa tipografia, controlli testo/dimensione, gerarchia e inspector delle altre viste. Niente numeri decorativi né badge ZIP/TTF.
- Riga con nome, categoria, varianti reali, provenienza, eventuale stato Variable e azione **Installa** oppure **Installato**. Più fonti equivalenti vengono mostrate come provenienze della stessa famiglia solo dopo deduplicazione verificata.
- La query vuota mostra un invito a cercare e lo stato dei provider, non scarica automaticamente tutti i font e non mostra metriche inventate.
- Il risultato può essere salvato in Ispirazione quando tale modulo esiste; fino ad allora nessun pulsante inattivo.

## Cache, progressione ed errori

Ricerca cache-first sui cataloghi abilitati. Le cache presenti permettono una ricerca offline; cache mancanti vengono caricate quando si avvia la ricerca globale o tramite azione esplicita. Non precaricare l'intera rete all'avvio dell'app.

Ogni provider ha stato indipendente: non caricato, caricamento, pronto, cache non aggiornata, errore, disabilitato, credenziale richiesta. Un provider lento o guasto non blocca risultati e installazioni degli altri. Distinguere assenza di risultati da catalogo non interrogato o errore.

Risultati progressivi con contatore qualificato come parziale finché esistono richieste pendenti. Un conteggio aggregato conta famiglie deduplicate, non la somma cieca dei provider. Ogni query ha una generazione; query o filtri cambiati invalidano risposte precedenti. Concorrenza limitata e debounce breve, senza ricerca duplicata a ogni render. L'ordine dei risultati è stabile: corrispondenza esatta, prefisso, altre corrispondenze, quindi nome e ID stabile. Non confrontare punteggi di popolarità incompatibili fra provider.

La paginazione/virtualizzazione avviene dopo filtro e deduplicazione. Preservare selezione se il risultato esiste ancora; non sostituire l'inspector con una risposta tardiva. Cambiamenti del totale non lasciano pagine vuote non valide.

## Identità e deduplicazione

Identità primaria: provider + ID famiglia + variante/versione. Mai fondere automaticamente famiglie solo perché hanno lo stesso nome normalizzato. Collegare equivalenze solo tramite provenienza autorevole o identità font verificata. Fontsource Extra mantiene il filtro type=other e non ripubblica Google. Nerd Fonts è una versione modificata con glifi aggiuntivi: resta distinto dall'originale.

Separare famiglia remota, file/variante e record locale. Lo stato installato deriva dal backend e dai file/registrazioni attuali, non da una preferenza frontend o dalla presenza di una cache. Specificare se è installata una sola variante o tutta la famiglia; non dichiarare l'intera famiglia completa dopo un successo parziale.

## Installazione e preview

**Installa** esegue una sola operazione: risoluzione asset, download, validazione, scrittura atomica, indicizzazione, attivazione Windows, lettura di conferma. Riutilizzare l'orchestrazione esistente, non duplicare Win32. Deduplicare richieste per identità e impedire doppi click durante il lavoro.

Al successo aggiornare lo stato in Locali, Ricerca Globale e catalogo individuale. Navigare a Locali con filtri incompatibili azzerati e selezione del record restituito, anche quando non è nella prima pagina alfabetica. Scorrere alla riga selezionata. Al fallimento di attivazione il file può restare disponibile localmente ma non appare Installato. Successi parziali indicano quali varianti sono attive e quali no.

Anteprime lazy, concorrenza limitata, cleanup FontFace e ObjectURL. Nessun download massivo degli archivi per righe non visibili. Provider che non consentono preview usano fallback dichiarato, non una falsa anteprima. Variable: conservare bytes e assi originali; non sintetizzare file statici.

Un provider solo consultabile espone **Apri sul sito**, non un falso **Installa**. Il requisito del click unico vale dove il download desktop e la licenza lo permettono. Non sostituire i file desktop con WOFF/WOFF2 senza dichiararlo e non convertirli automaticamente.

## Contratto dei provider

Adapter espliciti con capability indipendenti: catalog/search, preview, download desktop, attivazione esterna, autenticazione, variable. Licenza e provenienza accompagnano il risultato: identificativo/nome licenza, URL originale, autore se disponibile, usi desktop/web conosciuti, stato sconosciuto quando mancano dati. Free non significa open source né automaticamente uso commerciale/web/redistribuzione.

Prima versione: i provider già verificati. Nuovi cataloghi e API configurabili seguono la ricerca documentata in `2026-10-15-font-catalog-research.md`, senza considerarli già integrati.

Configurazione futura in Impostazioni: adapter supportato, nome, abilitazione alla ricerca globale, credenziale quando richiesta, test connessione, diagnostica redatta e rimozione della sola configurazione. Google Developer API opzionale con chiave dell'utente sostituisce la sorgente metadati Google, non crea un catalogo duplicato. Monotype richiede una tranche separata e verifica del contratto commerciale prima di qualsiasi promessa di download.

Eventuali endpoint personalizzati accettano solo un contratto JSON versionato e documentato; non pretendere che un URL/API key basti per parlare con ogni vendor. Niente codice JS, parser remoti o comandi eseguibili forniti dalla configurazione. Questa modalità non è obbligatoria per il primo rilascio.

## Sicurezza e licenze

- Richieste dal backend, host HTTPS espliciti per adapter, controllo redirect, timeout e dimensioni. CSP invariata senza wildcard.
- Per eventuali provider personalizzati validare host e destinazioni; bloccare loopback/reti private/link-local e credenziali negli URL salvo una futura modalità locale esplicita e separata. Non inoltrare token a host di redirect o asset diversi.
- Credenziali in archivio sicuro del sistema operativo, non in SQLite/config esportabile, log, screenshot o board. Nessuna chiave inserita nella chat.
- ZIP in memoria con limiti su entry e decompressione, niente estrazione generica dai nomi; font validati prima di scrittura atomica.
- Conservare provenienza e licenza associata; eventuale conservazione dei testi di licenza dal pacchetto usa un percorso di metadati limitato e validato, non indebolisce l'estrazione font.
- Nessuno scraping, aggiramento anti-bot o redistribuzione font implicita. Fontshare FFL e altri cataloghi freeware richiedono una verifica separata dei diritti di integrazione: la gratuità d'uso non autorizza una piattaforma di distribuzione.

## Criteri di accettazione

- Le quattro tab hanno esattamente etichette e ordine richiesti, cataloghi individuali ancora raggiungibili.
- Una query raccoglie risultati da tutti i provider abilitati; offline, provider guasto e risposta tardiva sono coperti da test isolati.
- Categorie/radio/glifi comuni; risultati deduplicati solo con evidenza; Nerd e originali non collassano.
- Preview lazy e risorse rilasciate; navigazione tastiera, focus, errori e layout ridotto verificati sul render reale.
- Installa attiva, sincronizza lo stato e seleziona esattamente il font locale anche oltre la prima pagina; fallimento/parzialità non appare come successo completo.
- Configurazioni provider non possono esfiltrare credenziali o aprire host arbitrari; test di input/redirect e log redatti.
- Suite Rust/UI, build frontend/Tauri e prova end-to-end con dati isolati. Test rete separati e ignored per la suite ordinaria.

## Fuori perimetro

Ricerca web generalista, riconoscimento font da screenshot, API arbitrarie senza adapter, checkout, acquisti automatici, bypass login/CAPTCHA, motore di raccomandazione AI, scraping DaFont e font cloud condivisi fra utenti.
