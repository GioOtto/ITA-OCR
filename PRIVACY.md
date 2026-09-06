# Privacy e dati

L’inferenza OCR avviene localmente tramite llama.cpp. Il server del motore
ascolta sull’interfaccia di loopback, non su tutte le interfacce di rete.
Non è necessario caricare i documenti su un servizio remoto per trascriverli.

L’app conserva archivio, impostazioni e log nel profilo utente. Questi dati
possono contenere testo dei documenti, nomi dei file e percorsi scelti
dall’utente. La disinstallazione non elimina automaticamente l’archivio.

Il sito di presentazione è statico e non offre caricamento o trascrizione
dei documenti nel browser. Non contiene analytics o moduli di raccolta dati.
Il fornitore di hosting può raccogliere log tecnici delle richieste HTTP.
I collegamenti a GitHub e Hugging Face aprono servizi esterni con proprie
condizioni e informative.

Il dataset di addestramento e valutazione resta privato. Non vengono
distribuiti documenti originali, trascrizioni di riferimento, manifest,
identificatori personali o risorse lessicali derivate dal corpus privato.
Gli esempi e le immagini pubbliche sono sintetici.

Un modello può comunque commettere errori o memorizzare parti dei dati di
addestramento: non viene dichiarata una garanzia di assenza di memorizzazione.

Prima di condividere log, schermate o documenti in una issue, sostituisci il
contenuto con un esempio sintetico e rimuovi metadati e percorsi personali.
