# Installing on Windows

*[Versione italiana](../it/INSTALLAZIONE.md)*

## Requirements

- Windows x64 with the WebView2 runtime available.
- Disk space: about 2.1 GB for the complete installation, of which 1.2 GB are
  the two GGUF weights. The installation without the models takes about 1 GB.
- Enough memory for the model and the pages: no universal minimum configuration
  has been certified. A compatible GPU speeds the work up; the CPU is a slower
  alternative.

The backend actually in use is shown in the interface. Vulkan needs a
compatible AMD, Intel or NVIDIA GPU; CUDA needs NVIDIA hardware and drivers
compatible with the bundled runtime.

## Procedure

1. Open the [release page](https://github.com/GioOtto/ITA-OCR/releases/latest).
2. Download the Windows installer and compare its hash with `SHA256SUMS.txt`.
3. Run it: the installation lives in your user profile and does not ask for
   administrator privileges.
4. Pick the installation type (see below). The default one is complete and
   already includes the models.
5. Start ITA-OCR, import a page and check the transcription against the original.

To verify the checksum with PowerShell, from the download folder:

```powershell
Get-FileHash .\ITA-OCR-setup_v1.0.0.exe -Algorithm SHA256
```

The current package is not signed with Authenticode. If Windows reports an
unknown publisher, verify the origin and the checksum before deciding whether
to run it. There is no need to disable Windows protections.

## Installation type

The installer offers three choices. Only the weights differ: the application,
the engine, the runtime, the dictionaries and the licences are always there.

| Choice | What it installs | Space |
| --- | --- | --- |
| **Complete installation** (default) | Everything, GGUF models included | ~2.1 GB |
| **Without the models** | Everything except the two GGUF files | ~1 GB |
| **Manual choice** | You pick the components | varies |

With the complete installation nothing else needs downloading: the two GGUF
files land in `models`, next to the executable, and the app finds them by itself
on first start.

Choose **Without the models** if you already have the weights from another
installation, if you want to keep them on a different drive, or if you would
rather download them from Hugging Face. In that case put them in `models` next
to `ocr-ita-desktop.exe`, or point `OCR_ITA_MODELS` at the folder holding them:
see [MODEL.md](MODEL.md).

## Common problems

**Model not found:** this happens with the installation without the models,
with the portable folder, or when `OCR_ITA_MODELS` points somewhere else. Both
files listed in [MODEL.md](MODEL.md) are required, with their exact names. With
the complete installation the weights are already in place.

**GPU not used:** check the backend reported by the app and your drivers. Try
Vulkan or CPU in the settings; having a CUDA DLL does not mean having an NVIDIA
GPU.

**PDF without OCR:** if a text layer exists, the app can extract it directly.
To read the image instead, enable the corresponding option in the settings.

**Icon not updated:** close the app while the package is being replaced.
Developers can run `aggiorna-icone-windows.ps1` after the build.

**Context-aware correction:** it needs optional local resources. The public
distribution uses the public dictionaries for standard spelling correction.

History, settings and logs stay in the user profile after uninstalling. Review
the logs before sharing them: they can contain paths and text.
