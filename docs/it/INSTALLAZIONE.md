# Installare su Windows

*[English version](../en/INSTALL.md)*

## Requisiti

- Windows x64 con runtime WebView2 disponibile.
- Spazio su disco: circa 2,1 GB per l'installazione completa, di cui 1,2 GB
  sono i due pesi GGUF. L'installazione senza modelli occupa circa 1 GB.
- Memoria sufficiente per modello e pagine: non è stata certificata una
  configurazione minima universale. Una GPU compatibile accelera il lavoro;
  la CPU è un'alternativa più lenta.

Il backend effettivamente usato compare nell'interfaccia. Vulkan serve GPU
compatibili AMD, Intel e NVIDIA; CUDA richiede hardware NVIDIA e driver
compatibili con il runtime incluso.

## Procedura

1. Apri la [pagina della release](https://github.com/GioOtto/ITA-OCR/releases/latest).
2. Scarica l'installer Windows e confrontane l'hash con `SHA256SUMS.txt`.
3. Eseguilo: l'installazione è nel tuo profilo utente e non chiede
   l'amministratore.
4. Scegli il tipo di installazione (vedi sotto). Quella predefinita è completa
   e comprende già i modelli.
5. Avvia ITA-OCR, importa una pagina e controlla la trascrizione sull'originale.

Per verificare il checksum con PowerShell, dalla cartella del download:

```powershell
Get-FileHash .\ITA-OCR-setup_v1.0.0.exe -Algorithm SHA256
```

Il pacchetto non è firmato con Authenticode, quindi SmartScreen mostra
«Windows ha protetto il PC» e qualche antivirus può segnalare il file: è la
reazione a un eseguibile poco diffuso e non firmato, non il rilevamento di
codice malevolo. Per procedere: *Ulteriori informazioni* → *Esegui
comunque*. Non occorre disattivare le protezioni di Windows.

Il codice è pubblico e l'installer si compila dagli stessi sorgenti: vedi
[LEGGIMI-WINDOWS.md](LEGGIMI-WINDOWS.md).

## Tipo di installazione

L'installer propone tre scelte. La differenza sta solo nei pesi: applicazione,
motore, runtime, dizionari e licenze ci sono in tutti i casi.

| Scelta | Cosa installa | Spazio |
| --- | --- | --- |
| **Installazione completa** (predefinita) | Tutto, modelli GGUF compresi | ~2,1 GB |
| **Senza i modelli** | Tutto tranne i due GGUF | ~1 GB |
| **Scelta manuale** | Decidi tu i componenti | variabile |

Con l'installazione completa non serve scaricare altro: i due GGUF finiscono in
`models`, accanto all'eseguibile, e l'app li trova da sola al primo avvio.

Scegli **Senza i modelli** se i pesi li hai già da un'altra installazione, se
vuoi tenerli su un altro disco, o se preferisci scaricarli da Hugging Face. In
quel caso mettili in `models` accanto a `ocr-ita-desktop.exe`, oppure imposta
`OCR_ITA_MODELS` sulla cartella che li contiene: vedi [MODELLO.md](MODELLO.md).

## Problemi comuni

**Modello non trovato:** succede con l'installazione senza modelli, con la
cartella portabile o quando `OCR_ITA_MODELS` punta altrove. Servono entrambi i
file indicati in [MODELLO.md](MODELLO.md), con i nomi esatti. Con
l'installazione completa i pesi ci sono già.

**GPU non usata:** controlla il backend mostrato dall'app e i driver. Prova
Vulkan o CPU nelle impostazioni; avere una DLL CUDA non significa avere una GPU NVIDIA.

**PDF senza OCR:** se esiste un livello di testo, l'app può estrarlo direttamente.
Per rileggere l'immagine, abilita l'opzione apposita nelle impostazioni.

**Icona non aggiornata:** chiudi l'app durante la sostituzione del pacchetto.
Gli sviluppatori possono usare `aggiorna-icone-windows.ps1` dopo la build.

**Correzione contestuale:** richiede risorse locali opzionali. La distribuzione
pubblica usa i dizionari pubblici per la correzione lessicale standard.

Archivio, impostazioni e log restano nel profilo utente dopo la disinstallazione.
Controlla i log prima di condividerli: possono contenere percorsi e testo.
