# Valutazione e limiti

Questa distribuzione non include il dataset di addestramento o di valutazione,
i documenti originali, le trascrizioni di riferimento o identificatori dei
partecipanti. Le schermate usano contenuti sintetici e non costituiscono un
benchmark dell'OCR.

## Metodo

Le valutazioni interne confrontano il modello base con il fine-tuning su
pagine italiane e osservano errori sui caratteri e sulle parole (CER e WER),
pagine incomplete, ripetizioni e latenza dell'intera pipeline. Preparazione
della pagina, backend, quantizzazione e strategia di decodifica influenzano
il risultato e devono essere indicati insieme a qualsiasi misura.

Il set di verifica finale ? stato consultato pi? volte durante lo sviluppo:
non ? un test indipendente incontaminato. Le misure interne vanno quindi
interpretate come descrittive. Non viene pubblicata una classifica o una
percentuale universale di accuratezza.

## Verifiche del pacchetto

La build Windows ? stata collaudata con GPU AMD Radeon RX 7900 XT e Vulkan.
Questo verifica che la pipeline possa completare un documento su quella
configurazione, non certifica tutte le GPU n? la qualit? di ogni trascrizione.
La suite UI usa un ponte simulato e verifica le interazioni dell'app.

## Limiti noti

- Scrittura complessa, impaginazione irregolare, formule e tabelle possono
  causare omissioni, sostituzioni, ripetizioni o contenuti inventati.
- La correzione da dizionario pu? introdurre errori: le sostituzioni restano
  evidenziate e il risultato va confrontato con l'originale.
- Piccole variazioni nella decodifica GPU possono produrre risultati diversi.
- Il riconoscimento su CPU pu? essere sensibilmente pi? lento.
- I risultati ottenuti con risorse lessicali private non sono trasferibili
  automaticamente al pacchetto pubblico, che usa solo dizionari pubblici.

Per una riproduzione pubblica usa esclusivamente documenti sintetici o dati
che puoi distribuire e documenta ambiente, parametri e criteri di misura.
