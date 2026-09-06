# Componenti di terze parti

*[English version](../en/THIRD-PARTY.md)*

Il codice originale di ITA-OCR è distribuito sotto [licenza MIT](../../LICENSE).
Questa licenza copre il codice di questo repository e **non sostituisce** quelle
dei componenti elencati qui sotto, che restano in vigore per le rispettive parti.
I testi completi si trovano in [licenses/](../../licenses).

## In breve

| Componente | Ruolo | Licenza |
| --- | --- | --- |
| [GLM-OCR](https://huggingface.co/zai-org/GLM-OCR) | modello base del fine-tuning | MIT (pesi), Apache-2.0 (codice del repository) |
| [llama.cpp](https://github.com/ggml-org/llama.cpp) | motore di inferenza | MIT |
| [Tauri](https://tauri.app) | involucro desktop | MIT o Apache-2.0 |
| [spellbook](https://github.com/helix-editor/spellbook) | motore di correzione ortografica | MPL-2.0 |
| [PDFium](https://pdfium.googlesource.com/pdfium/) | rendering dei PDF | BSD-3-Clause |
| [KaTeX](https://katex.org) | formule nell'interfaccia | MIT |
| Dizionario italiano `it_IT` | correzione lessicale | **GPL-3** |
| Dizionario inglese `en_US` | riconoscimento dei termini da non correggere | SCOWL (permissiva) |
| Crate Rust (494) | dipendenze dell'applicazione | in prevalenza MIT o Apache-2.0 |
| Runtime Visual C++ | esecuzione su Windows | termini di ridistribuzione Microsoft |

## Modello

**GLM-OCR** — Z.ai / zai-org. I pesi sul repository Hugging Face sono dichiarati
MIT; il codice del repository GitHub è Apache-2.0. Il fine-tuning per la
scrittura italiana distribuito da ITA-OCR è un'opera derivata di quei pesi e
mantiene la licenza MIT.

Né MIT né Apache-2.0 impongono obblighi sul nome dell'opera derivata. Il nome
«ITA-OCR» è indipendente e non indica un'affiliazione con Z.ai o con il progetto
GLM. Apache-2.0 §6 esclude esplicitamente la concessione di diritti sui marchi:
i riferimenti a GLM-OCR nella documentazione sono descrittivi dell'origine.

## Motore e involucro

**llama.cpp** — MIT, copyright ggml authors. Incluso come sottomodulo in
`ocr-ita/vendor/llama.cpp`; i binari `llama-server` e le librerie dei backend
sono ridistribuiti nel pacchetto Windows. Testo in
[licenses/llama.cpp-MIT.txt](../../licenses/llama.cpp-MIT.txt).

**Tauri**, con WRY e TAO — MIT o Apache-2.0, a scelta di chi ridistribuisce.
Compilato dentro l'eseguibile.

**PDFium** — BSD-3-Clause, copyright The PDFium Authors e Google. Ridistribuito
come `pdfium.dll`, dai binari di
[bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries).
Testo in [licenses/PDFium-BSD-3-Clause.txt](../../licenses/PDFium-BSD-3-Clause.txt).

**KaTeX** — MIT, copyright Khan Academy. Incluso in `ocr-desktop/app/ui/vendor/katex`
con i propri font. Testo in [licenses/KaTeX-LICENSE.txt](../../licenses/KaTeX-LICENSE.txt).

**spellbook** — MPL-2.0. È una libreria Rust, quindi il suo codice è collegato
staticamente nell'eseguibile. MPL-2.0 §3.2 consente di distribuire l'eseguibile
sotto una licenza diversa purché il codice sorgente della libreria resti
disponibile e gli avvisi non vengano rimossi: il sorgente è pubblico all'indirizzo
sopra e il testo della licenza è in
[licenses/spellbook-MPL-2.0.txt](../../licenses/spellbook-MPL-2.0.txt).

## Dizionari

**Italiano — `resources/dictionaries/it_IT`, licenza GPL-3.**

Copyright (C) 2001-2003 Gianluca Turconi; 2002-2007 Davide Prina;
2010-2015 Andrea Pescetti; 2020-2022 LibreItalia — Marina Latini.
Parte della «Estensione linguistica italiana / Italian Writing Aids extension»,
distribuita con le `libreoffice-dictionaries`.

Sono file di dati: l'applicazione li legge a runtime da disco con spellbook e
non li incorpora nell'eseguibile. Restano file distinti nella cartella
`dictionaries`, insieme al proprio `COPYING`, al README dell'autore e al testo
completo della licenza in `LICENSE-GPL-3.txt`. Chi ridistribuisce ITA-OCR deve
conservare quei file e i loro avvisi di copyright: il codice MIT
dell'applicazione e il dizionario GPL-3 viaggiano insieme senza fondersi.

**Inglese — `resources/dictionaries/en_US`, SCOWL.**

Derivato da SCOWL (Kevin Atkinson e collaboratori), con termini di
ridistribuzione permissivi riportati in `README_en_US.txt`. Serve soltanto a
riconoscere le parole inglesi e lasciarle intatte, mai a correggerle.

## Dipendenze Rust

L'applicazione dipende da 494 crate. La distribuzione delle licenze è in netta
prevalenza MIT o Apache-2.0, con alcuni componenti Unicode-3.0, MPL-2.0 e Zlib.
L'elenco completo, crate per crate, è in
[licenses/rust-dependencies.json](../../licenses/rust-dependencies.json) e in
[licenses/RUST.md](../../licenses/RUST.md); i testi delle licenze raccolte sono in
[licenses/rust/](../../licenses/rust).

## Componenti di sistema

Le librerie `msvcp140.dll`, `vcruntime140.dll` e `vcruntime140_1.dll` sono
redistributable di Microsoft Visual C++, incluse accanto all'eseguibile secondo
i termini di ridistribuzione di Visual Studio. Il runtime **WebView2** non è
incluso: viene fornito da Microsoft e deve essere presente sul sistema.

## Materiale non incluso

Il dataset di addestramento e valutazione, i documenti originali, le
trascrizioni di riferimento e le risorse lessicali derivate dal corpus non
fanno parte di questa distribuzione, in nessuna forma.

## Segnalazioni

Se un'attribuzione risulta mancante o inesatta, apri una
[segnalazione](https://github.com/GioOtto/ITA-OCR/issues): viene corretta.
Questo documento riporta i termini dichiarati dai rispettivi progetti e non
costituisce consulenza legale.
