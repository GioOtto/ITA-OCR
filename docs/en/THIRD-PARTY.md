# Third-party components

*[Versione italiana](../it/TERZE-PARTI.md)*

The original ITA-OCR code is released under the [MIT licence](../../LICENSE). That
licence covers the code in this repository and **does not replace** the
licences of the components listed below, which remain in force for their
respective parts. Full texts are in [licenses/](../../licenses).

## At a glance

| Component | Role | Licence |
| --- | --- | --- |
| [GLM-OCR](https://huggingface.co/zai-org/GLM-OCR) | base model of the fine-tune | MIT (weights), Apache-2.0 (repository code) |
| [llama.cpp](https://github.com/ggml-org/llama.cpp) | inference engine | MIT |
| [Tauri](https://tauri.app) | desktop shell | MIT or Apache-2.0 |
| [spellbook](https://github.com/helix-editor/spellbook) | spell-checking engine | MPL-2.0 |
| [PDFium](https://pdfium.googlesource.com/pdfium/) | PDF rendering | BSD-3-Clause |
| [KaTeX](https://katex.org) | formulas in the interface | MIT |
| Italian dictionary `it_IT` | spelling correction | **GPL-3** |
| English dictionary `en_US` | recognising terms to leave alone | SCOWL (permissive) |
| Rust crates (494) | application dependencies | predominantly MIT or Apache-2.0 |
| Visual C++ runtime | running on Windows | Microsoft redistribution terms |
| [Inter](https://rsms.me/inter/) | typeface of the website | OFL-1.1 |

## Model

**GLM-OCR** — Z.ai / zai-org. The weights on the Hugging Face repository are
declared MIT; the GitHub repository code is Apache-2.0. The Italian-handwriting
fine-tune distributed by ITA-OCR is a derivative of those weights and keeps the
MIT licence.

Neither MIT nor Apache-2.0 imposes any obligation on the name of a derivative
work. The name "ITA-OCR" is independent and implies no affiliation with Z.ai or
the GLM project. Apache-2.0 §6 explicitly withholds trademark rights:
references to GLM-OCR in the documentation are descriptive of origin.

## Engine and shell

**llama.cpp** — MIT, copyright the ggml authors. Included as a submodule under
`ocr-ita/vendor/llama.cpp`; the `llama-server` binaries and the backend
libraries are redistributed in the Windows package. Text in
[licenses/llama.cpp-MIT.txt](../../licenses/llama.cpp-MIT.txt).

**Tauri**, with WRY and TAO — MIT or Apache-2.0, at the redistributor's choice.
Compiled into the executable.

**PDFium** — BSD-3-Clause, copyright The PDFium Authors and Google.
Redistributed as `pdfium.dll`, from the
[bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries)
builds. Text in [licenses/PDFium-BSD-3-Clause.txt](../../licenses/PDFium-BSD-3-Clause.txt).

**KaTeX** — MIT, copyright Khan Academy. Included under
`ocr-desktop/app/ui/vendor/katex` with its fonts. Text in
[licenses/KaTeX-LICENSE.txt](../../licenses/KaTeX-LICENSE.txt).

**spellbook** — MPL-2.0. It is a Rust library, so its code is statically linked
into the executable. MPL-2.0 §3.2 allows distributing the executable under a
different licence provided the library's source stays available and the notices
are not removed: the source is public at the address above and the licence text
is in [licenses/spellbook-MPL-2.0.txt](../../licenses/spellbook-MPL-2.0.txt).

## Dictionaries

**Italian — `resources/dictionaries/it_IT`, GPL-3 licence.**

Copyright (C) 2001-2003 Gianluca Turconi; 2002-2007 Davide Prina;
2010-2015 Andrea Pescetti; 2020-2022 LibreItalia — Marina Latini.
Part of the "Estensione linguistica italiana / Italian Writing Aids extension",
distributed with the `libreoffice-dictionaries`.

These are data files: the application reads them from disk at runtime through
spellbook and never embeds them in the executable. They stay as separate files
in the `dictionaries` folder, together with their own `COPYING`, the upstream
README and the full licence text in `LICENSE-GPL-3.txt`. Anyone redistributing
ITA-OCR must keep those files and their copyright notices: the MIT-licensed
application code and the GPL-3 dictionary travel together without merging.

**English — `resources/dictionaries/en_US`, SCOWL.**

Derived from SCOWL (Kevin Atkinson and contributors), with permissive
redistribution terms stated in `README_en_US.txt`. It is used only to recognise
English words and leave them untouched, never to correct them.

## Rust dependencies

The application depends on 494 crates. The licence distribution is
overwhelmingly MIT or Apache-2.0, with a few Unicode-3.0, MPL-2.0 and Zlib
components. The complete crate-by-crate list is in
[licenses/rust-dependencies.json](../../licenses/rust-dependencies.json) and
[licenses/RUST.md](../../licenses/RUST.md); the collected licence texts are in
[licenses/rust/](../../licenses/rust).

## System components

The `msvcp140.dll`, `vcruntime140.dll` and `vcruntime140_1.dll` libraries are
Microsoft Visual C++ redistributables, shipped next to the executable under the
Visual Studio redistribution terms. The **WebView2** runtime is not included:
it is supplied by Microsoft and must be present on the system.

## Website typeface

The website uses **Inter**, distributed under the SIL Open Font License 1.1:
the licence text is in [licenses/Inter-OFL-1.1.txt](../../licenses/Inter-OFL-1.1.txt).
The font file is served by the website itself rather than by a third-party
CDN, so opening the page does not send the visitor's IP address anywhere
else. The typeface concerns the website only: the application uses system
fonts.

## Not included

The training and evaluation dataset, the original documents, the reference
transcriptions and the lexical resources derived from the corpus are not part
of this distribution, in any form.

## Reporting

If an attribution is missing or inaccurate, open an
[issue](https://github.com/GioOtto/ITA-OCR/issues) and it will be corrected.
This document reports the terms declared by the respective projects and is not
legal advice.
