# Downloading the model

*[Versione italiana](../MODELLO.md)*

The weights are neither in the repository nor in the installer: they are two
GGUF files totalling about 1.2 GB, beyond GitHub's per-file limit. They are
downloaded from Hugging Face and placed next to the application.

**Weights repository:** <https://huggingface.co/ueuegio/ITA-OCR>

## The two files

| File | Role | Size |
| --- | --- | --- |
| `glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf` | language model, fine-tuned for Italian handwriting | 686 MiB |
| `glm-ocr-base-mmproj-q8_0.gguf` | vision projector, turns the page into image tokens | 462 MiB |

**Both** are required, with exactly these names: that is how the application
looks them up. The projector alone cannot transcribe; the model alone cannot
see the page.

Both are quantised to Q8_0.

## Where to put them

In the `models` folder next to the executable:

```text
ITA-OCR\
  ocr-ita-desktop.exe
  models\
    glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf
    glm-ocr-base-mmproj-q8_0.gguf
```

With the standard installer that folder is
`%LOCALAPPDATA%\Programs\ITA-OCR\models`.

To keep them elsewhere — on another drive, or shared between installations —
set the `OCR_ITA_MODELS` environment variable to the folder holding them.

## How to download them

From the model page, **Files** tab, or with the official client:

```powershell
python -m pip install huggingface_hub
hf download ueuegio/ITA-OCR --local-dir models
```

Check the integrity after downloading:

```powershell
Get-FileHash .\models\glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf -Algorithm SHA256
Get-FileHash .\models\glm-ocr-base-mmproj-q8_0.gguf -Algorithm SHA256
```

```text
95c6a7b7f318293c4a5276a5dab773d0ef55d49ad357e24f89f9d4a701a5be8a  glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf
2f83f69e7e5268474c8e257606ace1c1c28ff3546e1308a63fb15d3dc2ebf459  glm-ocr-base-mmproj-q8_0.gguf
```

## Using them outside the app

The GGUF files work with llama.cpp on their own. The recognition prompt is
GLM-OCR's own, `Text Recognition:`, with no system prompt.

```bash
llama-mtmd-cli -m glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf \
  --mmproj glm-ocr-base-mmproj-q8_0.gguf \
  --image page.png -p "Text Recognition:"
```

The application instead starts `llama-server` on loopback and applies a cascade
of retries when the first decode ends up incomplete. Command-line output can
therefore differ from the app's on the same page.

## Licence and limits

The weights derive from [GLM-OCR](https://huggingface.co/zai-org/GLM-OCR) by
Z.ai, declared MIT in the upstream model card; the fine-tune keeps the same
licence. See [THIRD-PARTY.md](../../THIRD-PARTY.md).

The training dataset stays private and is not needed to use the model.
Recognition can omit, repeat or invent text: see
[BENCHMARKS.en.md](../../BENCHMARKS.en.md) for the stated method and limits.
