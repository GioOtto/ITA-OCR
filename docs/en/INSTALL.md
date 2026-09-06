# Installing on Windows

*[Versione italiana](../INSTALLAZIONE.md)*

## Requirements

- Windows x64 with the WebView2 runtime available.
- Room for the application, the runtime and two GGUF weights (about 1.2 GB for
  the weights alone).
- Enough memory for the model and the pages: no universal minimum configuration
  has been certified. A compatible GPU speeds the work up; the CPU is a slower
  alternative.

The backend actually in use is shown in the interface. Vulkan needs a
compatible AMD, Intel or NVIDIA GPU; CUDA needs NVIDIA hardware and drivers
compatible with the bundled runtime.

## Procedure

1. Open the [release page](https://github.com/GioOtto/ITA-OCR/releases/latest).
2. Download the Windows installer and compare its hash with `SHA256SUMS.txt`.
3. Run the installer in your user profile.
4. Follow [MODEL.md](MODEL.md) to download the two GGUF weights from Hugging Face.
5. Copy them into the `models` directory next to `ocr-ita-desktop.exe`, or set
   `OCR_ITA_MODELS` to the folder of your choice.
6. Start ITA-OCR, import a page and check the transcription against the original.

To verify the checksum with PowerShell, from the download folder:

```powershell
Get-FileHash .\ITA-OCR-setup_v1.0.0.exe -Algorithm SHA256
```

The current package is not signed with Authenticode. If Windows reports an
unknown publisher, verify the origin and the checksum before deciding whether
to run it. There is no need to disable Windows protections.

## Common problems

**Model not found:** both files listed in the guide are required, with their
exact names. The application installer does not contain the weights.

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
