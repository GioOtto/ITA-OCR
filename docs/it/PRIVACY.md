# Privacy e dati

*[English version](../en/PRIVACY.md)*

## Dove avviene il trattamento

L’inferenza OCR avviene localmente tramite llama.cpp. Il server del motore
ascolta sull’interfaccia di loopback `127.0.0.1`, non su tutte le interfacce
di rete. Non è necessario caricare i documenti su un servizio remoto per
trascriverli, e dopo l’installazione l’applicazione funziona offline.

L’app non contiene telemetria, non invia statistiche d’uso e non richiede un
account. La rete serve solo per scaricare l’installer e i pesi del modello,
una volta, e per le pagine web che apri volontariamente dall’interfaccia.

## Cosa resta sul tuo computer

L’app conserva archivio, impostazioni e log nel profilo utente
(`%APPDATA%\ocr-ita-desktop` su Windows). Questi dati possono contenere testo
dei documenti, nomi dei file e percorsi scelti dall’utente.
**La disinstallazione non elimina automaticamente l’archivio:** se contiene
dati personali, va rimosso a parte.

## ITA-OCR e il GDPR

L’elaborazione locale riduce il perimetro del trattamento in modo concreto:

- Per il passaggio OCR non interviene un responsabile esterno del trattamento.
- Non c’è trasferimento di dati verso paesi terzi.
- I documenti non escono dal dispositivo di chi li detiene.
- Non esiste una copia lato server da cui pretendere cancellazione o portabilità.

**Questo non equivale a una certificazione di conformità.** ITA-OCR è uno
strumento: la conformità resta in capo al titolare del trattamento e dipende
da base giuridica, informativa agli interessati, minimizzazione, tempi di
conservazione, sicurezza del dispositivo, cifratura del disco, gestione dei
backup e cancellazione dell’archivio locale. Chi tratta dati particolari, o
dati altrui in ambito professionale, dovrebbe valutare autonomamente la
propria situazione.

Il progetto non è un servizio: non c’è un titolare del trattamento che riceva
i tuoi documenti, perché non li riceve nessuno.

## Sito e servizi esterni

Il sito di presentazione è statico e non offre caricamento o trascrizione
dei documenti nel browser. Non contiene analytics o moduli di raccolta dati.
Il fornitore di hosting può raccogliere log tecnici delle richieste HTTP.
I collegamenti a GitHub e Hugging Face aprono servizi esterni con proprie
condizioni e informative.

## Dataset e modello

Il dataset di addestramento e valutazione resta privato. Non vengono
distribuiti documenti originali, trascrizioni di riferimento, manifest,
identificatori personali o risorse lessicali derivate dal corpus privato.
Gli esempi e le immagini pubbliche sono sintetici.

Un modello può comunque commettere errori o memorizzare parti dei dati di
addestramento: non viene dichiarata una garanzia di assenza di memorizzazione.

## Prima di condividere qualcosa

Prima di allegare log, schermate o documenti a una issue, sostituisci il
contenuto con un esempio sintetico e rimuovi metadati e percorsi personali.

Per domande su questo documento: **giorgio.ottoboni@proton.me**.
