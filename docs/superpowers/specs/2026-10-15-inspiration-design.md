# Ispirazione: abbinamenti e moodboard

Stato: specifica scritta, in coda; nessuna implementazione autorizzata da questa attività di documentazione.
Direzione approvata: due viste collegate, Abbinamenti e Moodboard; dati locali persistenti, griglia riordinabile, niente canvas infinita o AI nel primo rilascio. La revisione di questa specifica precede il piano esecutivo.

## Obiettivo

Raccogliere riferimenti visivi per progetto e trasformarli in composizioni tipografiche provabili. Un abbinamento salvato in una board resta modificabile e conserva il riferimento ai font originali.

Ispirazione è la terza delle quattro tab primarie: Locali, Ricerca Globale, Ispirazione, Cataloghi vari. Le due viste interne non aggiungono tab primarie. Riutilizzare tema typebook, categorie e controlli condivisi, senza numerazioni ornamentali.

## Abbinamenti

Tre ruoli: titolo, sottotitolo facoltativo e corpo. Ogni ruolo permette scelta di famiglia e variante fra font locali e cataloghi abilitati; modifica testo, dimensione, interlinea, spaziatura e colore. Sfondo comune e campione confrontabile a larghezza desktop o ridotta, senza introdurre un page builder.

I file remoti vengono caricati solo quando necessari alla composizione e solo quando le capability/licenze lo consentono. Anteprima non equivale a installazione Windows. Per font senza preview disponibile mostrare lo stato e proporre il percorso consentito, non fingere di renderizzare il carattere usando un fallback indistinguibile.

Per variable si usa il file originale. Conservare i valori degli assi eventualmente esposti dal motore di preview; limitare i valori agli intervalli dichiarati dal font. Nessuna generazione di istanze statiche e nessuna modifica ai bytes del file. I controlli degli assi richiedono metadati reali, non inferiti dal nome della variante.

Azioni: salvare composizione, duplicare, aggiornare, assegnare a una board e installare i font mancanti. Installazione multipla con un comando e report per font; non riscaricare quelli effettivamente installati. Riutilizzare l'installer e il contratto di stato condivisi. Per una singola installazione applicare ritorno a Locali con selezione esatta; per il comando multiplo restare nella composizione con riepilogo e azione esplicita per vedere i font, evitando navigazioni concorrenti.

Non chiamare gli abbinamenti raccomandati, curati o scientificamente compatibili senza una fonte/editorializzazione reale. La prima versione è uno strumento di prova, con stato vuoto guidato e composizioni dell'utente, non una collezione di coppie inventate.

## Moodboard

Board con nome e descrizione facoltativa, creazione, rinomina, duplicazione e cancellazione con conferma. Tipi di elemento:

- immagine importata dal computer;
- link di riferimento con titolo e nota modificabili;
- nota testuale;
- riferimento a una famiglia/variante;
- composizione tipografica salvata.

Visualizzazione a griglia riordinabile, dettaglio/edit dell'elemento e riordino sia con trascinamento sia con comandi da tastiera. Non affidare operazioni essenziali solo al drag-and-drop. L'ordine è persistente e deterministico dopo riavvio.

Le immagini vengono copiate nell'archivio dell'app: spostare o eliminare l'originale non rompe la board. La rimozione di un elemento non elimina la sorgente dell'utente. Gli asset condivisi da più board non vengono rimossi finché esiste un riferimento. Cancellare una board o una composizione non disinstalla font e non cancella file dalla libreria.

Link come riferimenti cliccabili; niente download automatico del sito, screenshot remoto, scraping, iframe o import HTML. Una miniatura va aggiunta dall'utente, non inventata. Non richiedere permessi di rete per una nota locale.

## Collegamento fra viste e cataloghi

Da Locali, Ricerca Globale e Cataloghi vari l'utente può aggiungere un font a una board o provarlo in un ruolo tipografico. Da una composizione salvata può tornare all'editor con font, testi e impostazioni ricostruiti.

Il riferimento al font comprende fonte/provider, ID famiglia, ID variante, ID locale se disponibile, nome snapshot e stato variable. Non basare tutto sul percorso assoluto o sul solo nome famiglia. Se il provider non è disponibile o il font è stato rimosso, il contenuto resta salvato e mostra un riferimento non risolto con sostituzione esplicita; nessun cambio silenzioso di carattere.

La composizione è un'entità salvata indipendente. Gli elementi board la referenziano: modificare e salvare la composizione aggiorna le sue occorrenze. Per creare una variazione indipendente l'utente usa Duplica. Eliminazione di una composizione referenziata richiede conferma e rimozione esplicita dei riferimenti, senza elementi fantasma.

## Persistenza e architettura

SQLite esistente, con migrazioni additive; niente secondo database o backend cloud. Entità logiche proposte, da adattare alle convenzioni reali nel piano:

- board: identità, titolo, descrizione e timestamp;
- composizione: identità, titolo, ruoli e impostazioni di preview serializzate con schema versionato;
- elemento board: board, tipo, ordine e payload validato o riferimento composizione;
- asset: identità derivata dal contenuto, tipo MIME validato, dimensioni e percorso relativo nell'archivio.

Il database non contiene bytes delle immagini né font duplicati. Asset sotto la directory dati dell'app; percorsi relativi validati e nessun accesso arbitrario a file tramite payload importati. Le associazioni usano foreign key e transazioni. Copia atomica prima della registrazione dell'asset; compensazione/recupero per una scrittura DB fallita senza cancellare asset già condivisi. Pulizia limitata agli asset gestiti e realmente orfani.

Autosave locale con indicazione salvataggio/in corso/errore, debounce e gestione coerente delle risposte tardive. Non cambiare board perdendo modifiche non salvate senza avviso. Un errore disco non deve diventare una falsa conferma di salvataggio.

## Sicurezza e limiti

- Primo rilascio immagini JPEG, PNG e WebP validate tramite decoder; limiti espliciti da fissare nel piano per byte, pixel e quantità per import. Rifiutare input corrotti e decompressioni eccessive. SVG e HTML non supportati come immagini attive.
- Note e titoli come testo, mai HTML eseguibile. Link HTTP/HTTPS aperti nel browser di sistema dopo validazione; rifiutare file:, javascript:, data: e schemi di comando.
- Nessun font proprietario o credenziale copiato dentro un payload board. Le board conservano riferimenti, non conferiscono diritti d'uso o redistribuzione.
- Le immagini importate restano dell'utente e con la loro provenienza, se annotata. Non dichiarare una licenza assente.
- Nessun upload, telemetria del contenuto o account. CSP senza wildcard e niente iframe di siti arbitrari.

## Stati UX e accessibilità

Stati vuoto, caricamento, salvataggio, errore, immagine mancante e font non disponibile distinti. Errori contestuali e riprova senza perdere i dati digitati. Controlli etichettati, focus visibile, Escape nei dialog, ritorno focus all'elemento originario e messaggi leggibili da screen reader.

Griglia adattiva e testi leggibili; inspector/editor usabili a larghezza ridotta senza overflow. Il campione di font non altera il carattere dei controlli UI. Animazione solo come feedback e rispetto reduced-motion.

## Sequenza di consegna

Prima il contratto font/installazione condiviso e Ricerca Globale. Poi un modulo Ispirazione in due tranche verificabili: editor abbinamenti con salvataggio, quindi board e riferimenti incrociati. Nessuna tranche è completa senza persistenza e test di riapertura.

## Criteri di accettazione

- Composizione creata, salvata, riaperta e modificata mantenendo font, varianti, testo e parametri.
- Font locale e remoto verificati, preview senza installazione implicita e risorse temporanee rilasciate.
- Variable originale invariato; valori assi conservati ove supportati; nessun font statico generato.
- Board con tutti i tipi elemento, ordine persistente e riordino a tastiera.
- Immagine disponibile dopo rimozione/spostamento della sorgente; asset condiviso non perso cancellando una sola board.
- Collegamento composizione/board e duplicazione indipendente verificati; font rimosso/offline non distrugge il contenuto.
- Cancellazioni non modificano libreria, installazioni Windows o file originali.
- Errori DB/disco e salvataggi tardivi coperti; nessuna conferma falsa.
- Test Rust/UI, migrazioni e foreign key, build frontend/Tauri e verifica render reale desktop/ridotto.

## Fuori perimetro iniziale

Canvas infinita, disegno libero, collaborazione/sync cloud, export con font incorporati, screenshot e import automatico di siti, riconoscimento font, suggerimenti AI e marketplace. Export immagini o pacchetti board è un'estensione futura, non una promessa del primo rilascio.
