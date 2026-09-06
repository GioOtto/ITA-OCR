# Building ITA-OCR on Linux

*[Versione italiana](../it/LEGGIMI-LINUX.md)*

Three scripts, in this order: `prepara.sh` fetches the binary dependencies,
`costruisci.sh` compiles the engine and the application, `impacchetta.sh`
assembles the AppImage. They are meant to be re-run: whatever is already in
place is skipped.

## Prerequisites

You need Git, Rust (`rustup`), CMake, Ninja or Make and a C++ compiler.
The interface needs the development libraries of GTK 3, WebKitGTK 4.1 and
libsoup 3; if they are absent, `prepara.sh` extracts them into a local sysroot
without installing anything system-wide (see below).

On Debian and Ubuntu:

```bash
sudo apt install build-essential cmake ninja-build pkg-config curl patchelf \
                 libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev
```

On Fedora: `webkit2gtk4.1-devel gtk3-devel libsoup3-devel` with
`@development-tools`, `cmake` and `patchelf`. On Arch: `webkit2gtk-4.1 gtk3
libsoup3 base-devel cmake patchelf`.

The CUDA backend is optional and only builds where the NVIDIA Toolkit is
present: `costruisci.sh` enables it by itself when it finds `nvcc` in the PATH,
and otherwise compiles Vulkan and CPU without a word. `nvcc` does not
cross-compile, so CUDA is only obtained by building on the machine that will
use it.

## Building

```bash
git clone --recurse-submodules https://github.com/GioOtto/ITA-OCR.git
cd ITA-OCR
bash ocr-desktop/scripts/prepara.sh
bash ocr-desktop/scripts/costruisci.sh
bash ocr-desktop/scripts/impacchetta.sh
```

The result is `ocr-desktop/dist/ITA-OCR-v1.0.0-x86_64.AppImage`, with its
`.sha256` next to it. If the clone was made without `--recurse-submodules`,
the scripts stop and name the missing submodule; fix it with
`git submodule update --init --recursive`.

The first build takes a while: llama.cpp generates and compiles every Vulkan
shader and every CPU variant. Later ones reuse `ocr-desktop/build/`.

### What `prepara.sh` does

It downloads into `ocr-desktop/thirdparty/`, which Git ignores, four things
the repository cannot contain:

| Dependency | What it is for | If missing |
| --- | --- | --- |
| **PDFium** (`bblanchon/pdfium-binaries` build) | opening PDFs | `impacchetta.sh` stops |
| **appimagetool** and the **type2 runtime** | producing the AppImage | only the portable folder is left |
| **glslc** and SPIR-V headers | compiling the Vulkan shaders | `cmake` does not configure |
| **GTK/WebKit sysroot** | interface headers | only when the `-dev` packages are absent |

The type2 runtime is needed because appimagetool's default one demands
`libfuse2`, which recent distributions no longer ship. The sysroot is a way
out, not the main road: if you can install the system `-dev` packages, do, and
`prepara.sh` will skip that step.

glslc and the SPIR-V headers can also come from LunarG's Vulkan SDK if you
already have it installed: `prepara.sh` notices and downloads nothing. They are
passed to CMake explicitly because `ggml-vulkan` includes
`<spirv/unified1/spirv.hpp>` but does not link the `SPIRV-Headers` target: with
the SDK this goes unnoticed, because it keeps `spirv/` next to `vulkan/`.

## Building only the application

With the engine already built:

```bash
cd ocr-desktop/app/src-tauri
cargo build --release --locked
cargo test --locked
```

Then `bash ocr-desktop/scripts/impacchetta.sh` to reassemble the package. The
application finds its resources relative to itself, or through
`OCR_ITA_RESOURCES` and `OCR_ITA_MODELS`: no path from the development machine
ends up in the binary.

## What goes into the AppImage

The application, `llama-server` with its libraries, every CPU variant, the
Vulkan backend, `libpdfium.so`, the public dictionaries and their licences.

The two GGUF files **stay out**: they weigh 1.3 GB and the AppImage would be
over GitHub's limit for a release asset. `impacchetta.sh` links them into
`ocr-desktop/dist/models/` when it finds them in `ocr-ita/models/gguf/`, so the
AppImage sees them next to itself while testing. To really include them in the
package, useful for a USB stick but not for a release:

```bash
MODELLI_NEL_PACCHETTO=1 bash ocr-desktop/scripts/impacchetta.sh
```

The CUDA backend ends up in the package only if it was present at build time.
The AppImage published by CI carries CPU and Vulkan: `ggml` loads backends at
runtime, so the package is valid either way.

The dataset and the lexical resources derived from training are not part of
any package.

## Portability

The AppImage carries the engine but **not** GTK, WebKitGTK and libsoup: those
are system libraries and come from the distribution. The binary also requires a
glibc no older than the one on the machine that compiled it, because glibc
guarantees forward compatibility, not backward.

For a package that also runs on older distributions, build on the oldest one
you want to support. This is why the
[`build-linux.yml`](../../.github/workflows/build-linux.yml) workflow uses the
`ubuntu-22.04` runner rather than the latest available.

To find out which glibc an already built package demands:

```bash
objdump -T ocr-desktop/dist/ITA-OCR.AppDir/usr/bin/ocr-ita-desktop \
  | grep -o 'GLIBC_[0-9.]*' | sort -Vu | tail -1
```

### Changing the version

The version has a single source: the `version` field of
`ocr-desktop/app/src-tauri/tauri.conf.json`. The AppImage name, the Windows
installer name and the label on the site are copies of it. After raising it:

```bash
python scripts/verifica-pubblicazione.py
```

The script lists the copies left behind, with file name and line. It exists
because the site's download buttons point at the files by name: if a name stays
old, `releases/latest` answers 404 and nobody notices.

## Checks

```bash
cd ocr-desktop/app/src-tauri && cargo test --locked && cd -
python scripts/verifica-pubblicazione.py
```

The interface suite needs Playwright and Pillow:

```bash
python -m pip install playwright Pillow
python -m playwright install chromium
python ocr-desktop/smoke/ui.py
```

It simulates the Tauri bridge: it checks the interface, not the model. To try
real OCR on a synthetic document created locally:

```bash
./ocr-desktop/dist/ITA-OCR-v1.0.0-x86_64.AppImage --prova --backend vulkan --pagine 1 document.pdf
```
