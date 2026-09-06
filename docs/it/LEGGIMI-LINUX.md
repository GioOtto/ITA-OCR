# Compilare ITA-OCR su Linux

*[English version](../en/BUILDING-LINUX.md)*

Tre script, in quest'ordine: `prepara.sh` procura le dipendenze binarie,
`costruisci.sh` compila motore e applicazione, `impacchetta.sh` monta
l'AppImage. Sono pensati per essere rilanciati: quello che è già a posto lo
saltano.

## Prerequisiti

Servono Git, Rust (`rustup`), CMake, Ninja o Make e un compilatore C++.
Per l'interfaccia servono le librerie di sviluppo di GTK 3, WebKitGTK 4.1 e
libsoup 3; se non ci sono, `prepara.sh` le estrae in un sysroot locale senza
installare niente nel sistema (vedi sotto).

Su Debian e Ubuntu:

```bash
sudo apt install build-essential cmake ninja-build pkg-config curl patchelf \
                 libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev
```

Su Fedora: `webkit2gtk4.1-devel gtk3-devel libsoup3-devel` con
`@development-tools`, `cmake` e `patchelf`. Su Arch: `webkit2gtk-4.1 gtk3
libsoup3 base-devel cmake patchelf`.

Il backend CUDA è facoltativo e si compila solo dove c'è il Toolkit NVIDIA:
`costruisci.sh` lo abilita da solo se trova `nvcc` nel PATH, e altrimenti
compila Vulkan e CPU senza dire niente. `nvcc` non cross-compila, quindi CUDA
lo si ottiene soltanto costruendo sulla macchina che lo userà.

## Costruire

```bash
git clone --recurse-submodules https://github.com/GioOtto/ITA-OCR.git
cd ITA-OCR
bash ocr-desktop/scripts/prepara.sh
bash ocr-desktop/scripts/costruisci.sh
bash ocr-desktop/scripts/impacchetta.sh
```

Il risultato è `ocr-desktop/dist/ITA-OCR-v1.0.0-x86_64.AppImage`, con accanto
il suo `.sha256`. Se il clone è stato fatto senza `--recurse-submodules`, gli
script si fermano dicendo quale submodule manca: si rimedia con
`git submodule update --init --recursive`.

La prima compilazione richiede parecchio tempo: llama.cpp genera e compila
tutti gli shader Vulkan e ogni variante CPU. Le successive riusano
`ocr-desktop/build/`.

### Cosa fa `prepara.sh`

Scarica in `ocr-desktop/thirdparty/`, che è ignorato da Git, quattro cose che
il repository non può contenere:

| Dipendenza | A cosa serve | Se manca |
| --- | --- | --- |
| **PDFium** (build di `bblanchon/pdfium-binaries`) | aprire i PDF | `impacchetta.sh` si ferma |
| **appimagetool** e **runtime type2** | produrre l'AppImage | resta solo la cartella portabile |
| **glslc** e intestazioni SPIR-V | compilare gli shader Vulkan | `cmake` non configura |
| **sysroot GTK/WebKit** | intestazioni dell'interfaccia | solo se i pacchetti `-dev` non sono installati |

Il runtime type2 serve perché quello predefinito di appimagetool pretende
`libfuse2`, che sulle distribuzioni recenti non c'è più. Il sysroot è una via
d'uscita, non la strada maestra: se puoi installare i pacchetti `-dev` di
sistema, fallo e `prepara.sh` salterà quel passo.

glslc e le intestazioni SPIR-V si possono anche prendere dal Vulkan SDK di
LunarG, se lo hai già installato: `prepara.sh` se ne accorge e non scarica
niente. Vengono passate a CMake esplicitamente perché `ggml-vulkan` include
`<spirv/unified1/spirv.hpp>` ma non collega il target `SPIRV-Headers`: con
l'SDK la cosa non si nota, perché tiene `spirv/` accanto a `vulkan/`.

## Compilare solo l'applicazione

Con il motore già costruito:

```bash
cd ocr-desktop/app/src-tauri
cargo build --release --locked
cargo test --locked
```

Poi `bash ocr-desktop/scripts/impacchetta.sh` per rimontare il pacchetto.
L'applicazione trova le risorse rispetto a se stessa, oppure tramite
`OCR_ITA_RESOURCES` e `OCR_ITA_MODELS`: nessun percorso della macchina di
sviluppo finisce nel binario.

## Cosa finisce nell'AppImage

Applicazione, `llama-server` con le sue librerie, tutte le varianti CPU, il
backend Vulkan, `libpdfium.so`, i dizionari pubblici e le loro licenze.

I due GGUF **restano fuori**: pesano 1,3 GB e l'AppImage sarebbe oltre il
limite di GitHub per un allegato di release. `impacchetta.sh` li collega in
`ocr-desktop/dist/models/` se li trova in `ocr-ita/models/gguf/`, così
l'AppImage li vede accanto a sé durante le prove. Per includerli davvero nel
pacchetto — utile per una chiavetta, non per una release:

```bash
MODELLI_NEL_PACCHETTO=1 bash ocr-desktop/scripts/impacchetta.sh
```

Il backend CUDA finisce nel pacchetto solo se era presente al momento della
build. L'AppImage pubblicata dalla CI ha CPU e Vulkan: `ggml` carica i backend
a runtime, quindi il pacchetto resta valido in entrambi i casi.

Il dataset e le risorse lessicali derivate dall'addestramento non fanno parte
di nessun pacchetto.

## Portabilità

L'AppImage porta con sé il motore ma **non** GTK, WebKitGTK e libsoup: sono
librerie di sistema e vanno prese dalla distribuzione. Il binario richiede
inoltre una glibc non più vecchia di quella della macchina che lo ha
compilato, perché glibc garantisce la compatibilità in avanti, non
all'indietro.

Per un pacchetto che giri anche su distribuzioni più anziane, compila sulla
più vecchia che vuoi supportare. Il workflow
[`build-linux.yml`](../../.github/workflows/build-linux.yml) usa per questo il
runner `ubuntu-22.04`, non l'ultimo disponibile.

Per sapere che glibc pretende un pacchetto già costruito:

```bash
objdump -T ocr-desktop/dist/ITA-OCR.AppDir/usr/bin/ocr-ita-desktop \
  | grep -o 'GLIBC_[0-9.]*' | sort -Vu | tail -1
```

### Cambiare versione

La versione ha una sola fonte: il campo `version` di
`ocr-desktop/app/src-tauri/tauri.conf.json`. Il nome dell'AppImage, quello
dell'installer Windows e l'etichetta sul sito ne sono copie. Dopo averla
alzata:

```bash
python scripts/verifica-pubblicazione.py
```

Lo script elenca le copie rimaste indietro con nome di file e riga. Serve
perché i bottoni di download del sito puntano ai file per nome: se un nome
resta vecchio, `releases/latest` risponde 404 e non se ne accorge nessuno.

## Verifiche

```bash
cd ocr-desktop/app/src-tauri && cargo test --locked && cd -
python scripts/verifica-pubblicazione.py
```

Per la suite dell'interfaccia servono Playwright e Pillow:

```bash
python -m pip install playwright Pillow
python -m playwright install chromium
python ocr-desktop/smoke/ui.py
```

Simula il ponte Tauri: verifica l'interfaccia, non il modello. Per provare
l'OCR vero su un documento sintetico creato in locale:

```bash
./ocr-desktop/dist/ITA-OCR-v1.0.0-x86_64.AppImage --prova --backend vulkan --pagine 1 documento.pdf
```
