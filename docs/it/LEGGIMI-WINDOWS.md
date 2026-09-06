# Compilare ITA-OCR su Windows

*[English version](../en/BUILDING.md)*

## Prerequisiti

Installa Git, Rust con toolchain MSVC, Visual Studio Build Tools con il
carico di lavoro C++, CMake, Ninja e Vulkan SDK. Per CUDA serve anche il
Toolkit NVIDIA compatibile con la tua toolchain. Python, Pillow e numpy
servono soltanto per rigenerare le icone.

Apri il prompt **x64 Native Tools Command Prompt for VS** e raggiungi la
radice del clone. CMake, Ninja, Cargo e gli strumenti GPU devono essere nel PATH.

```powershell
git submodule update --init --recursive
powershell -ExecutionPolicy Bypass -File ocr-desktop\scripts\costruisci-windows.ps1
```

Per una build Vulkan e CPU senza CUDA:

```powershell
powershell -ExecutionPolicy Bypass -File ocr-desktop\scripts\costruisci-windows.ps1 -SaltaCuda
```

Il risultato è `ocr-desktop/dist/ITA-OCR-windows`. Lo script registra la
costruzione in `ocr-desktop/dist/costruzione-windows-<data>.log`.
Chiudi l'app prima di sostituire gli eseguibili.

## Compilare solo l'app

Con motore e risorse già disponibili:

```powershell
cd ocr-desktop/app/src-tauri
cargo build --release --locked
cargo test --locked
```

Copia l'eseguibile nella cartella portabile esistente. Le risorse si trovano
rispetto all'eseguibile, oppure tramite `OCR_ITA_RESOURCES` e `OCR_ITA_MODELS`.
Nessun percorso della macchina di sviluppo è richiesto dall'app.

## Installer

Installa Inno Setup 6 e rendi `ISCC` disponibile nel PATH. Dalla radice:

```powershell
ISCC /DSORGENTE="ocr-desktop\dist\ITA-OCR-windows" /DUSCITA="ocr-desktop\dist" ocr-desktop\scripts\installer.iss
```

Inno richiede percorsi assoluti se il prompt e lo script hanno directory
base diverse: usa `Resolve-Path` e passa i risultati, senza fissare lettere
unità o profili utente negli script.

Cosa finisce nell'installer lo decide la cartella passata a `SORGENTE`:
`costruisci-windows.ps1` ci copia i due GGUF se li trova in `dist\models`, e
`installer.iss` li mette nel componente `modelli`, incluso nell'installazione
completa e assente da quella leggera. L'installer pubblicato per la v1.0.0 è
costruito con i pesi dentro.

La cartella portabile prodotta dalla CI, invece, li lascia fuori di proposito:
li si aggiunge dopo, da Hugging Face, seguendo [MODELLO.md](MODELLO.md).

Il dataset e le risorse lessicali personali non fanno parte di nessun pacchetto.

### Cambiare versione

La versione ha una sola fonte: il campo `version` di
`ocr-desktop/app/src-tauri/tauri.conf.json`. Il nome dell'installer e
l'etichetta sul sito ne sono copie. Dopo averla alzata:

```powershell
python scripts/verifica-pubblicazione.py
```

Lo script confronta le copie con il manifesto ed elenca quelle rimaste
indietro, nome di file e riga. Serve perche' il bottone di download del sito
punta all'installer per nome: se il nome resta vecchio, `releases/latest`
risponde 404 e non se ne accorge nessuno.

## Icone

```powershell
python -m pip install Pillow numpy
python ocr-desktop/scripts/genera-icone.py
python ocr-desktop/scripts/genera-icone.py --verifica
```

Il logo sorgente è `ocr-desktop/scripts/marchio/logo.png`. Tutte le risoluzioni
conservano il marchio completo. Il generatore produce anche la maschera
`app/ui/marchio.png`, così interfaccia e icone restano coerenti.
`build.rs` segue le directory delle icone e dell'interfaccia: non serve pulire
Cargo a ogni modifica. Dopo app e installer:

```powershell
powershell -ExecutionPolicy Bypass -File ocr-desktop/scripts/aggiorna-icone-windows.ps1
```

## Verifiche

```powershell
python -m pip install playwright Pillow
python -m playwright install chromium
python ocr-desktop/smoke/ui.py
python scripts/verifica-pubblicazione.py
```

Per provare l'OCR usa un documento sintetico creato localmente con
`ocr-ita-desktop.exe --prova --backend vulkan --pagine 1 <documento>`.
La suite UI simula il ponte Tauri: verifica l'interfaccia, non il modello.

Gli script Linux sono sperimentali; questa release distribuisce solo Windows.
