<div align="center">
  <img src="docs/assets/logo.png" width="112" height="112" alt="ITA-OCR: a document between four scanner brackets" />
  <h1>ITA-OCR</h1>
  <p><strong>From the page to the text. On your own machine.</strong></p>
  <p>Local OCR for Italian handwriting, built on a fine-tune of GLM-OCR.</p>
  <p>
    <a href="https://GioOtto.github.io/ITA-OCR/en/">Website</a> ·
    <a href="https://github.com/GioOtto/ITA-OCR/releases/latest">Windows download</a> ·
    <a href="docs/en/MODEL.md">Model</a> ·
    <a href="docs/assets/ITA-OCR-report-tecnico.pdf">Technical report</a> ·
    <a href="BUILDING.md">Building</a> ·
    <a href="https://github.com/GioOtto/ITA-OCR/issues">Issues</a>
  </p>
  <p><img alt="Windows x64" src="https://img.shields.io/badge/Windows-x64-181818" /> <img alt="MIT code" src="https://img.shields.io/badge/code-MIT-27674c" /> <img alt="Local inference" src="https://img.shields.io/badge/inference-local-27674c" /> <img alt="No telemetry" src="https://img.shields.io/badge/telemetry-none-27674c" /></p>
  <p><a href="README.md">Italiano</a> · <strong>English</strong></p>
</div>

![The ITA-OCR interface: document and transcription side by side, synthetic example](docs/assets/app-light.png)

*Demonstration screenshot with synthetic content. It is not a measure of OCR accuracy.*

## What it is

ITA-OCR is a desktop application that transcribes handwritten Italian pages
and PDF or image documents. Recognition runs on your machine through
[llama.cpp](https://github.com/ggml-org/llama.cpp) with a model derived from
[GLM-OCR](https://huggingface.co/zai-org/GLM-OCR), fine-tuned for Italian
handwriting. No cloud OCR service and no API key are involved.

The interface puts the original next to the result, lets you inspect every
correction and export the text. PDFs that already carry a text layer can be
read directly, without going through the model.

## Download and start

1. Download **ITA-OCR-setup_v1.0.0.exe** from the [Windows release](https://github.com/GioOtto/ITA-OCR/releases/latest).
2. Install it into your user profile: no administrator privileges required.
3. Download the **two GGUF files** listed in the [model guide](docs/en/MODEL.md) separately and place them in the `models` folder next to the application.
4. Open ITA-OCR, import a document, run the transcription and compare it with the original.

The installer ships the application, the engine and the public dictionaries.
**The weights are distributed separately on Hugging Face; the dataset stays
private.** For the full procedure, requirements and troubleshooting see the
[Windows guide](docs/en/INSTALL.md).

## What it does

| Feature | Behaviour |
| --- | --- |
| PDFs and images | Imports PDF, PNG, JPEG, TIFF, BMP, WebP and GIF |
| Comparison | Original and text side by side, page by page |
| PDFs with text | Direct extraction; OCR can be forced in the settings |
| Spelling correction | Public Italian dictionary, corrections stay highlighted; English terms are protected |
| Formulas and tables | Recognised formulas and tables are rendered in the text |
| Export | Copy to clipboard, export to TXT, Markdown and DOCX |
| Local history | Sessions are stored on your machine |
| Theme | Light, dark or automatic |

![ITA-OCR in dark theme, synthetic example](docs/assets/app-dark.png)

## How it works

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/pipeline-en-dark.svg">
  <img alt="Local pipeline: document, page preparation, recognition with GLM-OCR on llama.cpp, review and export. Everything runs on the user's machine." src="docs/assets/pipeline-en-light.svg">
</picture>

The pipeline prepares each page, chooses between text extraction and OCR, then
runs a cascade of retries whenever the model returns an incomplete or repetitive
result. Spelling correction and export happen locally. Tauri connects the
HTML/CSS/JavaScript interface to the Rust backend.

The engine supports CPU, Vulkan and CUDA where hardware, drivers and package
allow it. Local Windows testing was carried out on an AMD Radeon RX 7900 XT
with Vulkan; that does not imply equivalent performance on every GPU.

## Local processing and personal data

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/privacy-en-dark.svg">
  <img alt="Documents, transcriptions, history, model and dictionaries stay on the machine. Only the installer and the model weights come in from the network, once." src="docs/assets/privacy-en-light.svg">
</picture>

Pages never leave the device. The engine listens on `127.0.0.1` only, there is
no telemetry, no account is required, and once the application and the weights
are installed recognition works with no connection at all.

For anyone handling personal data this narrows the perimeter in a concrete way:
there is no external processor involved in the OCR step, no transfer to third
countries, and the documents stay under the control of whoever holds them. That
is the substantive difference from a cloud OCR service.

**This is not a certification.** GDPR compliance remains the responsibility of
the data controller and depends on the legal basis, the privacy notice,
retention periods, device security and the handling of the local history —
which ITA-OCR stores in the user profile and which uninstalling does not
remove. See [PRIVACY.en.md](PRIVACY.en.md) and [SECURITY.md](SECURITY.md).

## Model, data and limits

- **Base model:** GLM-OCR by Z.ai; the upstream model card declares the base weights MIT.
- **Adaptation:** fine-tuned for Italian handwriting, distributed in GGUF together with the matching vision projector — [`ueuegio/ITA-OCR`](https://huggingface.co/ueuegio/ITA-OCR).
- **Dataset:** private. It is not part of the repository, the website, the installer or the screenshots.
- **Technical report:** [*Teaching a Vision Model When to Stop*](docs/assets/ITA-OCR-report-tecnico.pdf) — the fine-tune, the termination collapse and the inference cascade.
- **Evaluation:** [method and limits](BENCHMARKS.en.md); no real examples, identifiers or per-person results are published.
- **Accuracy:** OCR can omit, repeat or invent text, especially on complex layouts, formulas or difficult handwriting. Always check the result against the original.

Advanced context-aware correction needs additional local lexical resources that
are not part of this distribution. The standard corrector works with the public
dictionaries alone.

## Development

```powershell
git clone --recurse-submodules https://github.com/GioOtto/ITA-OCR.git
cd ITA-OCR
```

See [BUILDING.md](BUILDING.md) for the toolchain and the build.

```text
ocr-desktop/app/src-tauri/     Rust backend and Tauri configuration
ocr-desktop/app/ui/            interface and assets
ocr-desktop/resources/         public dictionaries and their licences
ocr-desktop/scripts/           icon generation, build and installer
ocr-desktop/smoke/             UI checks with synthetic content
ocr-ita/vendor/llama.cpp/      engine submodule
docs/                          static website, images and guides
scripts/                       diagrams, screenshots and publication check
licenses/                      third-party notices and licences
```

Contributions and reports: [CONTRIBUTING.md](CONTRIBUTING.md).

## Statement on the use of AI

**The code, part of the documentation and the graphic assets were produced with
the help of artificial-intelligence tools.** That assistance is not a guarantee
of correctness, security or accuracy. The software comes with no warranty;
changes require review and testing, and transcriptions must be verified.

## Licences

The original ITA-OCR code is released under the [MIT licence](LICENSE).
Third-party dependencies and data keep their own licences: in particular the
Italian dictionary is GPL-3.0 and `spellbook` is MPL-2.0. The project's MIT
licence does not replace the licences of the bundled components.
See [THIRD-PARTY.md](THIRD-PARTY.md) and [licenses/](licenses/).

## Contact

Reports and proposals: [repository issues](https://github.com/GioOtto/ITA-OCR/issues).
Direct contact: **giorgio.ottoboni@proton.me**.
For vulnerabilities, follow [SECURITY.md](SECURITY.md) first.
