# Building ITA-OCR on Windows

*[Versione italiana](../it/LEGGIMI-WINDOWS.md)*

## Prerequisites

Install Git, Rust with the MSVC toolchain, Visual Studio Build Tools with the
C++ workload, CMake, Ninja and the Vulkan SDK. CUDA additionally needs the
NVIDIA Toolkit matching your toolchain. Python, Pillow and numpy are only
needed to regenerate the icons.

Open the **x64 Native Tools Command Prompt for VS** and move to the root of the
clone. CMake, Ninja, Cargo and the GPU tools must be on the PATH.

```powershell
git submodule update --init --recursive
powershell -ExecutionPolicy Bypass -File ocr-desktop\scripts\costruisci-windows.ps1
```

For a Vulkan and CPU build without CUDA:

```powershell
powershell -ExecutionPolicy Bypass -File ocr-desktop\scripts\costruisci-windows.ps1 -SaltaCuda
```

The result is `ocr-desktop/dist/ITA-OCR-windows`. The script logs the build to
`ocr-desktop/dist/costruzione-windows-<date>.log`. Close the app before
replacing the executables.

## Building the app alone

With the engine and resources already in place:

```powershell
cd ocr-desktop/app/src-tauri
cargo build --release --locked
cargo test --locked
```

Copy the executable into the existing portable folder. Resources are located
relative to the executable, or through `OCR_ITA_RESOURCES` and
`OCR_ITA_MODELS`. The app never requires a path from the development machine.

## Installer

Install Inno Setup 6 and make `ISCC` available on the PATH. From the root:

```powershell
ISCC /DSORGENTE="ocr-desktop\dist\ITA-OCR-windows" /DUSCITA="ocr-desktop\dist" ocr-desktop\scripts\installer.iss
```

Inno needs absolute paths when the prompt and the script have different base
directories: use `Resolve-Path` and pass the results, rather than hard-coding
drive letters or user profiles in the scripts.

The public installer ships the application, the runtime, the dictionaries and
the licences. The GGUF weights are downloaded separately from Hugging Face:
[MODEL.md](MODEL.md). The dataset and the private lexical resources are
not part of the package.

## Icons

```powershell
python -m pip install Pillow numpy
python ocr-desktop/scripts/genera-icone.py
python ocr-desktop/scripts/genera-icone.py --verifica
```

The source logo is `ocr-desktop/scripts/marchio/logo.png`. Every resolution
keeps the full mark. The generator also produces the `app/ui/marchio.png` mask,
so the interface and the icons stay consistent. `build.rs` watches the icon and
interface directories, so there is no need to clean Cargo after each change.
Once the app and the installer are built:

```powershell
powershell -ExecutionPolicy Bypass -File ocr-desktop/scripts/aggiorna-icone-windows.ps1
```

## Documentation assets

Diagrams and screenshots are generated, never edited by hand:

```powershell
python scripts/genera-diagrammi.py
python scripts/genera-schermate.py
```

The first writes the eight SVG diagrams (Italian and English, light and dark).
The second drives the interface with a simulated Tauri bridge and captures the
screenshots used by the README and the website, using synthetic content only.

## Checks

```powershell
python -m pip install playwright Pillow
python -m playwright install chromium
python ocr-desktop/smoke/ui.py
python scripts/verifica-pubblicazione.py
```

To try the OCR itself, use a synthetic document created locally with
`ocr-ita-desktop.exe --prova --backend vulkan --pagine 1 <document>`.
The UI suite simulates the Tauri bridge: it checks the interface, not the model.

The Linux scripts are experimental; this release ships Windows only.
