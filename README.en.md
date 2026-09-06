<div align="center">
  <img src="docs/assets/logo.png" width="112" height="112" alt="ITA-OCR: a document between four scanner brackets" />
  <h1>ITA-OCR</h1>
  <p><strong>OCR for Italian handwriting, on your own machine.</strong></p>
  <p>A Windows and Linux app built on a fine-tune of GLM-OCR. Recognition runs locally.</p>

  <p>
    <a href="https://GioOtto.github.io/ITA-OCR/en/"><img alt="Visit the website: GioOtto.github.io/ITA-OCR" src="https://img.shields.io/badge/Website-GioOtto.github.io%2FITA--OCR-111111?style=for-the-badge" /></a>
    <a href="https://github.com/GioOtto/ITA-OCR/releases/latest"><img alt="Download for Windows x64" src="https://img.shields.io/badge/Download-Windows%20x64-111111?style=for-the-badge&logo=windows&logoColor=white" /></a>
    <a href="https://github.com/GioOtto/ITA-OCR/releases/latest"><img alt="Download for Linux, x86-64 AppImage" src="https://img.shields.io/badge/Download-Linux%20AppImage-111111?style=for-the-badge&logo=linux&logoColor=white" /></a>
  </p>

  <p>
    <a href="docs/assets/ITA-OCR-report-tecnico.pdf"><img alt="Read the technical report: Teaching a Vision Model When to Stop (PDF)" src="https://img.shields.io/badge/Technical%20report-Teaching%20a%20Vision%20Model%20When%20to%20Stop%20(PDF)-8B1A1A?style=for-the-badge&logo=adobeacrobatreader&logoColor=white" /></a>
  </p>

  <p>
    <a href="docs/en/INSTALL.md">Windows install</a> &nbsp;&nbsp;
    <a href="docs/en/INSTALL-LINUX.md">Linux install</a> &nbsp;&nbsp;
    <a href="docs/en/MODEL.md">Model</a> &nbsp;&nbsp;
    <a href="docs/en/BENCHMARKS.md">Evaluation</a> &nbsp;&nbsp;
    <a href="https://github.com/GioOtto/ITA-OCR/issues">Issues</a>
  </p>
  <p><img alt="Windows x64" src="https://img.shields.io/badge/Windows-x64-181818" /> <img alt="Linux x86-64" src="https://img.shields.io/badge/Linux-x86--64-181818" /> <img alt="MIT code" src="https://img.shields.io/badge/code-MIT-181818" /> <img alt="Local inference" src="https://img.shields.io/badge/inference-local-181818" /> <img alt="No telemetry" src="https://img.shields.io/badge/telemetry-none-181818" /></p>
  <p><a href="README.md">Italiano</a> &nbsp; <strong>English</strong></p>
</div>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/app-dark.png">
  <img alt="The ITA-OCR interface: document and transcription side by side, synthetic example" src="docs/assets/app-light.png">
</picture>

*Demonstration screenshot with synthetic content. It is not a measure of OCR accuracy.*

## What it is

ITA-OCR is a desktop application, for Windows and Linux, that transcribes
handwritten Italian pages and PDF or image documents. Recognition runs on your machine through
[llama.cpp](https://github.com/ggml-org/llama.cpp) with a model derived from
[GLM-OCR](https://huggingface.co/zai-org/GLM-OCR), fine-tuned for Italian
handwriting. No cloud OCR service and no API key are involved.

The interface puts the original next to the result, lets you inspect every
correction and export the text. PDFs that already carry a text layer can be
read directly, without going through the model.

## Download and start

The quickest route is the website: **<https://GioOtto.github.io/ITA-OCR/en/>**,
which has the download, the screenshots and the results on a single page.

### Windows

1. Download **ITA-OCR-setup_v1.0.0.exe** from the [release](https://github.com/GioOtto/ITA-OCR/releases/latest).
2. Install it into your user profile: no administrator privileges required.
3. Keep the complete installation, the default one: it already includes the models.
4. Open ITA-OCR, import a document, run the transcription and compare it with the original.

The complete installation ships everything you need (application, engine,
runtime, public dictionaries and both GGUF weights) and asks for no further
download. During setup you can pick a lighter installation **without the
models**: in that case the weights come
[from Hugging Face](https://huggingface.co/ueuegio/ITA-OCR).
Full procedure: [Windows guide](docs/en/INSTALL.md).

### Linux

1. Download **ITA-OCR-v1.0.0-x86_64.AppImage** from the [release](https://github.com/GioOtto/ITA-OCR/releases/latest).
2. `chmod +x ITA-OCR-v1.0.0-x86_64.AppImage`
3. Get the two GGUF weights [from Hugging Face](https://huggingface.co/ueuegio/ITA-OCR)
   and put them in a `models` folder next to the AppImage.
4. Run the file. Nothing is installed; to remove it, delete it.

The AppImage carries the application, the engine, the CPU and Vulkan backends,
PDFium and the public dictionaries; **the models stay out**, because another
1.3 GB would exceed GitHub's limit for a release asset. It needs GTK 3,
WebKitGTK 4.1 and libsoup 3 from the distribution, and glibc 2.39 or newer.
Full procedure and known problems: [Linux guide](docs/en/INSTALL-LINUX.md).

**The training dataset stays private**, on both platforms.

## Windows will flag the installer (Windows only)

The package is not signed with an Authenticode certificate. On first run
SmartScreen shows **"Windows protected your PC"**, and some antivirus products
may flag the file as suspicious: that is the standard reaction to an uncommon,
unsigned executable, not a detection of malicious code. To continue:
*More info* → *Run anyway*.

Nothing here has to be taken on trust: the code is all in this repository, the
installer is built from these sources with [the included scripts](docs/en/BUILDING.md),
and the SHA-256 hashes are published in `SHA256SUMS.txt` next to the release.
If you would rather not run a binary nobody signed, you can build it yourself.

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

![A handwritten page and its transcription side by side, synthetic example](docs/assets/app-manoscritto.png)

*The real use case: a handwritten page on the left, the recognised text on the
right, with the formulas typeset. This page is synthetic too.*

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
allow it. Local testing was carried out on an AMD Radeon RX 7900 XT with
Vulkan, on Windows and on Linux; that does not imply equivalent performance on
every GPU. The published AppImage carries the CPU and Vulkan backends; CUDA has
to be compiled on the machine that will use it, because `nvcc` does not
cross-compile.

## How much better than the base model

The fine-tune lowers the error on every evaluation set available, on both axes.
The figures come from the [technical report](docs/assets/ITA-OCR-report-tecnico.pdf),
each with the set it was measured on, because a percentage without its set
means nothing.

| Evaluation set | Character error | Word error |
| --- | --- | --- |
| Holdout, 164 pages, writers never seen in training | 30.9 → 25.7 (**−16.7%**) | 54.7 → 45.5 (**−16.7%**) |
| Sealed benchmark, 67 readable pages | 29.3 → 18.8 (**−35.9%**) | 56.7 → 39.0 (**−31.3%**) |
| Subset declared easy, 61 pages | 12.1 → 11.3 (−6.5%) | 37.0 → 30.7 (−17.0%) |

A second effect matters as much, and a percentage hides it: **pages lost drop
from 24 to 5** on a 48-page panel of problematic pages. That is the cascade at
work, recognising a decode that ended badly and retrying. And it costs less than
not having it: on the holdout the whole cascaded run is faster than a single
plain pass, because runaway pages are cut short instead of running to the token
ceiling.

These are relative reductions on those sets, not a universal accuracy figure.
The final set was consulted several times during development: the measurements
are descriptive, not an independent benchmark. Full method and limits in
[BENCHMARKS.md](docs/en/BENCHMARKS.md).

## The technical report

> ### [**Teaching a Vision Model When to Stop**](docs/assets/ITA-OCR-report-tecnico.pdf)
>
> How you teach a vision model to stop generating: the problem behind every
> number above.
>
> GLM-OCR has **two** stopping conditions, and fine-tuning breaks one of them:
> the model learns Italian handwriting and forgets when to stop, so one page in
> three ends in a loop that only exhausts itself at the token limit. The report
> covers why this happens, why it cannot be fixed in training, and how the
> **inference cascade** works around it by recognising a decode that has gone
> wrong and retrying, which takes lost pages from 24 down to 5.
>
> It also covers the construction of the corpus, the evaluation protocol with
> disjoint writers, quantisation, and the limits of all of it.
>
> **[→ Read the technical report (PDF)](docs/assets/ITA-OCR-report-tecnico.pdf)**

## Do I need a GPU?

No. The model also runs on the CPU, at the same quality: only the time changes.

On the test machine a page goes from a little over a second and a half on the
GPU to roughly fifteen seconds on the CPU, about an order of magnitude. The
absolute values depend on the processor, the graphics card, the drivers and the
complexity of the page, so read them as a ratio rather than a promise: another
machine will produce other numbers with the same gap.

On the CPU the time goes mostly into encoding the image rather than generating
the text, and quantisation buys memory rather than latency: around 2.4 GB
resident with the Q8_0 weights that ship.

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
retention periods, device security and the handling of the local history,
which ITA-OCR stores in the user profile and which uninstalling does not
remove. See [PRIVACY.md](docs/en/PRIVACY.md) and [SECURITY.md](.github/SECURITY.md).

## Model, data and limits

- **Base model:** GLM-OCR by Z.ai; the upstream model card declares the base weights MIT.
- **Adaptation:** fine-tuned for Italian handwriting, distributed in GGUF together with the matching vision projector, on [`ueuegio/ITA-OCR`](https://huggingface.co/ueuegio/ITA-OCR).
- **Dataset:** private. It is not part of the repository, the website, the installer or the screenshots.
- **Technical report:** [*Teaching a Vision Model When to Stop*](docs/assets/ITA-OCR-report-tecnico.pdf). It covers the fine-tune, the termination collapse and the inference cascade.
- **Evaluation:** [method and limits](docs/en/BENCHMARKS.md); no real examples, identifiers or per-person results are published.
- **Accuracy:** OCR can omit, repeat or invent text, especially on complex layouts, formulas or difficult handwriting. Always check the result against the original.

Advanced context-aware correction needs additional local lexical resources that
are not part of this distribution. The standard corrector works with the public
dictionaries alone.

## Development

```bash
git clone --recurse-submodules https://github.com/GioOtto/ITA-OCR.git
cd ITA-OCR
```

Then [BUILDING.md](docs/en/BUILDING.md) or
[BUILDING-LINUX.md](docs/en/BUILDING-LINUX.md) for the toolchain and the build.
On Linux it is three commands: `prepara.sh` fetches the binary dependencies,
`costruisci.sh` compiles the engine and the application, `impacchetta.sh`
assembles the AppImage. Both platforms also have a workflow in
[.github/workflows/](.github/workflows).

```text
docs/                       static website: index.html, en/index.html, assets/
docs/en/                    guides in English
docs/it/                    guides in Italian
.github/workflows/          Windows and Linux builds
ocr-desktop/app/src-tauri/  Rust backend and Tauri configuration
ocr-desktop/app/ui/         interface and assets
ocr-desktop/resources/      public dictionaries and their licences
ocr-desktop/scripts/        icons, build, packaging and installer
ocr-desktop/smoke/          UI checks with synthetic content
ocr-ita/vendor/             submodules: llama.cpp and Vulkan-Headers
scripts/                    diagrams, screenshots and publication check
licenses/                   third-party notices and licences
```

Contributions and reports: [CONTRIBUTING.md](.github/CONTRIBUTING.md).

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
See [THIRD-PARTY.md](docs/en/THIRD-PARTY.md) and [licenses/](licenses).

## Contact

Reports and proposals: [repository issues](https://github.com/GioOtto/ITA-OCR/issues).
Direct contact: **giorgio.ottoboni@proton.me**.
For vulnerabilities, follow [SECURITY.md](.github/SECURITY.md) first.

If ITA-OCR was useful to you or you liked the project, consider leaving a
[star on GitHub](https://github.com/GioOtto/ITA-OCR). It helps others discover
the project.
