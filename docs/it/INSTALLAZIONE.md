# Installare su Windows

*[English version](../en/INSTALL.md)*

## Requisiti

- Windows x64 con runtime WebView2 disponibile.
- Spazio per applicazione, runtime e due pesi GGUF (circa 1,2 GB per i soli pesi).
- Memoria sufficiente per modello e pagine: non è stata certificata una
  configurazione minima universale. Una GPU compatibile accelera il lavoro;
  la CPU è un’alternativa più lenta.

Il backend effettivamente usato compare nell’interfaccia. Vulkan serve GPU
compatibili AMD, Intel e NVIDIA; CUDA richiede hardware NVIDIA e driver
compatibili con il runtime incluso.

## Procedura

1. Apri la [pagina della release](https://github.com/GioOtto/ITA-OCR/releases/latest).
2. Scarica l’installer Windows e confrontane l’hash con `SHA256SUMS.txt`.
3. Esegui l’installer nel tuo profilo utente.
4. Segui [MODELLO.md](MODELLO.md) per scaricare i due pesi GGUF da Hugging Face.
5. Copiali nella directory `models` accanto a `ocr-ita-desktop.exe`, oppure
   imposta `OCR_ITA_MODELS` sulla cartella scelta.
6. Avvia ITA-OCR, importa una pagina e controlla la trascrizione sull’originale.

Per verificare il checksum con PowerShell, dalla cartella del download:

```powershell
Get-FileHash .\ITA-OCR-setup_v1.0.0.exe -Algorithm SHA256
```

Il pacchetto attuale non è firmato con Authenticode. Se Windows segnala un
editore non riconosciuto, verifica origine e checksum prima di decidere se
eseguirlo. Non occorre disattivare le protezioni di Windows.

## Problemi comuni

**Modello non trovato:** servono entrambi i file indicati nella guida, con i
nomi esatti. L’installer dell’app non contiene i pesi.

**GPU non usata:** controlla il backend mostrato dall’app e i driver. Prova
Vulkan o CPU nelle impostazioni; avere una DLL CUDA non significa avere una GPU NVIDIA.

**PDF senza OCR:** se esiste un livello di testo, l’app può estrarlo direttamente.
Per rileggere l’immagine, abilita l’opzione apposita nelle impostazioni.

**Icona non aggiornata:** chiudi l’app durante la sostituzione del pacchetto.
Gli sviluppatori possono usare `aggiorna-icone-windows.ps1` dopo la build.

**Correzione contestuale:** richiede risorse locali opzionali. La distribuzione
pubblica usa i dizionari pubblici per la correzione lessicale standard.

Archivio, impostazioni e log restano nel profilo utente dopo la disinstallazione.
Controlla i log prima di condividerli: possono contenere percorsi e testo.
