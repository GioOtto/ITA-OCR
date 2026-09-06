<div align="center">
  <img src="docs/assets/logo.png" width="112" height="112" alt="ITA-OCR: un documento tra quattro staffe di scansione" />
  <h1>ITA-OCR</h1>
  <p><strong>Dalla pagina al testo. Sul tuo computer.</strong></p>
  <p>OCR locale per la scrittura italiana, con un fine-tuning di GLM-OCR.</p>
  <p>
    <a href="https://GioOtto.github.io/ITA-OCR/">Sito</a> ·
    <a href="https://github.com/GioOtto/ITA-OCR/releases/latest">Download Windows</a> ·
    <a href="docs/it/MODELLO.md">Modello</a> ·
    <a href="docs/assets/ITA-OCR-report-tecnico.pdf">Report tecnico</a> ·
    <a href="docs/it/LEGGIMI-WINDOWS.md">Compilare</a> ·
    <a href="https://github.com/GioOtto/ITA-OCR/issues">Segnalazioni</a>
  </p>
  <p><img alt="Windows x64" src="https://img.shields.io/badge/Windows-x64-181818" /> <img alt="Codice MIT" src="https://img.shields.io/badge/codice-MIT-27674c" /> <img alt="Inferenza locale" src="https://img.shields.io/badge/inferenza-locale-27674c" /> <img alt="Nessuna telemetria" src="https://img.shields.io/badge/telemetria-nessuna-27674c" /></p>
  <p><strong>Italiano</strong> · <a href="README.en.md">English</a></p>
</div>

![Interfaccia di ITA-OCR: documento e trascrizione affiancati, esempio sintetico](docs/assets/app-light.png)

*Schermata dimostrativa dell’interfaccia con contenuti sintetici. Non è una misura dell’accuratezza OCR.*

## Che cos’è

ITA-OCR è un’app desktop per trascrivere pagine scritte a mano in italiano e
documenti PDF o immagine. Il riconoscimento gira sul computer con
[llama.cpp](https://github.com/ggml-org/llama.cpp) e un modello derivato da
[GLM-OCR](https://huggingface.co/zai-org/GLM-OCR), adattato con un fine-tuning
alla scrittura italiana. Non serve un servizio OCR cloud né una chiave API.

L’interfaccia affianca originale e risultato, permette di controllare le
correzioni e di esportare il testo. I PDF con testo già presente possono
essere letti direttamente, senza passare dal modello.

## Scarica e inizia

1. Scarica **ITA-OCR-setup_v1.0.0.exe** dalla [release Windows](https://github.com/GioOtto/ITA-OCR/releases/latest).
2. Installalo nel tuo profilo utente: non richiede privilegi di amministratore.
3. Tieni l’installazione completa, quella predefinita: comprende già i modelli.
4. Apri ITA-OCR, importa un documento, avvia la trascrizione e confrontala con l’originale.

L’installazione completa contiene tutto quello che serve — applicazione, motore,
runtime, dizionari pubblici e i due pesi GGUF — e non richiede altri download.
Durante il setup puoi scegliere un’installazione **senza i modelli**, più
leggera: in quel caso i pesi si prendono
[su Hugging Face](https://huggingface.co/ueuegio/ITA-OCR), come per la cartella
portabile o per tenerli su un altro disco con `OCR_ITA_MODELS`.
**Il dataset di addestramento resta privato.** Per la procedura completa, i
requisiti e la risoluzione dei problemi vedi la
[guida Windows](docs/it/INSTALLAZIONE.md).

## Windows segnalerà l'installer

Il pacchetto non è firmato con un certificato Authenticode. Al primo avvio
SmartScreen mostra **«Windows ha protetto il PC»**, e qualche antivirus può
segnalare il file come sospetto: è la reazione standard a un eseguibile poco
diffuso e non firmato, non il rilevamento di codice malevolo. Per procedere:
*Ulteriori informazioni* → *Esegui comunque*.

Non c'è niente da prendere sulla fiducia: il codice è tutto in questo
repository, l'installer è costruito da questi sorgenti con
[gli script inclusi](docs/it/LEGGIMI-WINDOWS.md), e gli hash SHA-256 sono
pubblicati in `SHA256SUMS.txt` accanto alla release. Se preferisci non
eseguire un binario firmato da nessuno, puoi compilarlo tu.

## Cosa puoi fare

| Funzione | Comportamento |
| --- | --- |
| PDF e immagini | Importazione di PDF, PNG, JPEG, TIFF, BMP, WebP e GIF |
| Confronto | Originale e testo affiancati, con navigazione per pagina |
| PDF con testo | Estrazione diretta; OCR forzabile nelle impostazioni |
| Correzione lessicale | Dizionario italiano pubblico, correzioni evidenziate; protezione dei termini inglesi |
| Formule e tabelle | Visualizzazione delle formule riconosciute e delle tabelle nel testo |
| Esportazione | Copia negli appunti ed esportazione TXT, Markdown e DOCX |
| Archivio locale | Sessioni salvate sul computer |
| Tema | Chiaro, scuro o automatico |

![ITA-OCR in tema scuro, esempio sintetico](docs/assets/app-dark.png)

![Una pagina manoscritta e la sua trascrizione affiancate, esempio sintetico](docs/assets/app-manoscritto.png)

*Il caso d’uso vero: una pagina scritta a mano a sinistra, il testo riconosciuto
a destra, con le formule composte. Anche questa pagina è sintetica.*

## Come funziona

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/pipeline-it-dark.svg">
  <img alt="Pipeline locale: documento, preparazione della pagina, riconoscimento con GLM-OCR su llama.cpp, verifica ed esportazione. Tutto sul computer dell'utente." src="docs/assets/pipeline-it-light.svg">
</picture>

La pipeline prepara le pagine, sceglie fra estrazione del testo e OCR, poi
esegue una cascata di tentativi quando il modello produce un risultato
incompleto o ripetitivo. La correzione lessicale e l’esportazione avvengono
localmente. Tauri collega l’interfaccia HTML/CSS/JavaScript al backend Rust.

Il motore supporta CPU, Vulkan e CUDA quando hardware, driver e pacchetto
sono compatibili. Il collaudo Windows locale è stato eseguito su AMD Radeon
RX 7900 XT con Vulkan. Non implica prestazioni equivalenti su ogni GPU.

## Quanto migliora rispetto al modello base

Il fine-tuning riduce l’errore su tutti gli insiemi di valutazione disponibili,
su entrambi gli assi. Le cifre sono quelle del
[report tecnico](docs/assets/ITA-OCR-report-tecnico.pdf), con l’insieme su cui
sono state misurate — perché una percentuale senza il suo insieme non vuole
dire niente.

| Insieme di valutazione | Errore sui caratteri | Errore sulle parole |
| --- | --- | --- |
| Holdout, 164 pagine, scriventi mai visti in addestramento | 30,9 → 25,7 (**−16,7%**) | 54,7 → 45,5 (**−16,7%**) |
| Benchmark sigillato, 67 pagine leggibili | 29,3 → 18,8 (**−35,9%**) | 56,7 → 39,0 (**−31,3%**) |
| Sottoinsieme dichiarato facile, 61 pagine | 12,1 → 11,3 (−6,5%) | 37,0 → 30,7 (−17,0%) |

Conta anche un secondo effetto, meno visibile in una percentuale: **le pagine
perse passano da 24 a 5** su un pannello di 48 pagine problematiche. È il lavoro
della cascata, che riconosce una decodifica finita male e ritenta. E costa meno
del non averla: sull’holdout l’intera passata con cascata è più rapida di una
passata sola senza, perché le pagine in fuga vengono interrotte invece di
correre fino al limite di token.

Le percentuali sono riduzioni relative su quegli insiemi, non una percentuale
universale di accuratezza. Il set finale è stato consultato più volte durante lo
sviluppo: le misure sono descrittive, non un benchmark indipendente. Metodo
completo e limiti in [BENCHMARKS.md](docs/it/BENCHMARKS.md).

## Serve una GPU?

No. Il modello gira anche su CPU, ed è la stessa qualità: cambia solo il tempo.

Sulla macchina di collaudo una pagina passa da poco più di un secondo e mezzo
su GPU a una quindicina di secondi su CPU — circa un ordine di grandezza. I
valori assoluti dipendono da processore, scheda video, driver e complessità
della pagina, quindi vanno presi come rapporto e non come promessa: su un’altra
macchina saranno altri numeri, con lo stesso divario.

Su CPU il tempo se ne va soprattutto nel codificare l’immagine, non nel generare
il testo, e la quantizzazione fa risparmiare memoria più che tempo: circa 2,4 GB
residenti con i pesi Q8_0 distribuiti.

## Elaborazione locale e dati personali

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/privacy-it-dark.svg">
  <img alt="Documenti, trascrizioni, archivio, modello e dizionari restano sul computer. Dalla rete arrivano soltanto l'installer e i pesi del modello, una volta sola." src="docs/assets/privacy-it-light.svg">
</picture>

Le pagine non lasciano il dispositivo. Il motore ascolta solo su `127.0.0.1`,
non c’è telemetria, non serve un account e, una volta installati applicazione
e pesi, il riconoscimento funziona senza connessione.

Per chi tratta dati personali questo riduce il perimetro in modo concreto:
non c’è un responsabile esterno del trattamento per l’OCR, non c’è
trasferimento verso paesi terzi e i documenti restano sotto il controllo di
chi li detiene. È la differenza sostanziale rispetto a un OCR cloud.

**Questo non equivale a una certificazione.** La conformità al GDPR resta in
capo al titolare del trattamento e dipende da base giuridica, informativa,
tempi di conservazione, sicurezza del dispositivo e gestione dell’archivio
locale — che ITA-OCR salva nel profilo utente e che la disinstallazione non
rimuove. Vedi [PRIVACY.md](docs/it/PRIVACY.md) e [SECURITY.md](.github/SECURITY.md).

## Modello, dati e limiti

- **Modello base:** GLM-OCR di Z.ai; pesi base dichiarati MIT nella model card upstream.
- **Adattamento:** fine-tuning per la scrittura italiana, distribuito in GGUF insieme al proiettore visivo compatibile — [`ueuegio/ITA-OCR`](https://huggingface.co/ueuegio/ITA-OCR).
- **Dataset:** privato, non incluso nel repository, nel sito, nell’installer o nelle schermate.
- **Report tecnico:** [*Teaching a Vision Model When to Stop*](docs/assets/ITA-OCR-report-tecnico.pdf) — fine-tuning, collasso della terminazione e cascata di inferenza.
- **Valutazione:** [metodo e limiti](docs/it/BENCHMARKS.md); non vengono pubblicati esempi reali, identificatori o risultati per persona.
- **Accuratezza:** l’OCR può omettere, ripetere o inventare testo, soprattutto con pagine complesse, formule o scrittura difficile. Verifica ogni risultato sull’originale.

La correzione contestuale avanzata richiede risorse lessicali locali aggiuntive
che non sono incluse nella distribuzione. Il correttore standard funziona
con i soli dizionari pubblici.

## Sviluppo

```powershell
git clone --recurse-submodules https://github.com/GioOtto/ITA-OCR.git
cd ITA-OCR
```

Segui [LEGGIMI-WINDOWS.md](docs/it/LEGGIMI-WINDOWS.md) per toolchain e build.

```text
docs/                       sito statico: index.html, en/index.html, assets/
docs/it/                    guide in italiano
docs/en/                    guide in inglese
.github/                    contribuire, sicurezza e workflow di build
ocr-desktop/app/src-tauri/  backend Rust e configurazione Tauri
ocr-desktop/app/ui/         interfaccia e asset
ocr-desktop/resources/      dizionari pubblici e relative licenze
ocr-desktop/scripts/        generazione icone, build e installer
ocr-desktop/smoke/          prove UI con contenuti sintetici
ocr-ita/vendor/llama.cpp/   submodule del motore
scripts/                    schemi, schermate e verifica di pubblicazione
licenses/                   avvisi e licenze delle dipendenze
```

Contributi e segnalazioni: [CONTRIBUTING.md](.github/CONTRIBUTING.md).

## Dichiarazione sull’uso dell’AI

**Il codice, parte della documentazione e gli asset grafici sono stati
realizzati con il supporto di strumenti di intelligenza artificiale.**
Questo supporto non costituisce una garanzia di correttezza, sicurezza o
accuratezza. Il software è fornito senza garanzie; le modifiche richiedono
revisione e verifiche, e le trascrizioni devono essere controllate.

## Licenze

Il codice originale di ITA-OCR è distribuito sotto [licenza MIT](LICENSE).
Le dipendenze e i dati di terze parti conservano le rispettive licenze:
in particolare, il dizionario italiano è GPL-3.0 e `spellbook` è MPL-2.0.
La licenza MIT del progetto non sostituisce quelle dei componenti inclusi.
Vedi [TERZE-PARTI.md](docs/it/TERZE-PARTI.md) e [licenses/](licenses).

## Contatti

Segnalazioni e proposte: [issue del repository](https://github.com/GioOtto/ITA-OCR/issues).
Per contatto diretto: **giorgio.ottoboni@proton.me**.
Per le vulnerabilità segui prima [SECURITY.md](.github/SECURITY.md).

Se ITA-OCR ti è stato utile o il progetto ti è piaciuto, puoi lasciare una
[stella su GitHub](https://github.com/GioOtto/ITA-OCR): aiuta il progetto a
farsi conoscere.
