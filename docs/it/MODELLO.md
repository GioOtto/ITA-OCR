# Scaricare il modello

*[English version](../en/MODEL.md)*

I pesi non sono nel repository né nell'installer: sono due file GGUF per circa
1,2 GB complessivi, oltre il limite per file di GitHub. Si scaricano da
Hugging Face e si mettono accanto all'applicazione.

**Repository dei pesi:** <https://huggingface.co/ueuegio/ITA-OCR>

## I due file

| File | Ruolo | Dimensione |
| --- | --- | --- |
| `glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf` | modello linguistico, fine-tuning per la scrittura italiana | 686 MiB |
| `glm-ocr-base-mmproj-q8_0.gguf` | proiettore visivo, converte la pagina in token immagine | 462 MiB |

Servono **entrambi**, con i nomi esatti: l'applicazione li cerca così. Il
proiettore da solo non trascrive, il modello da solo non vede la pagina.

Quantizzazione Q8_0 per tutti e due.

## Dove metterli

Nella cartella `models` accanto all'eseguibile:

```text
ITA-OCR\
  ocr-ita-desktop.exe
  models\
    glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf
    glm-ocr-base-mmproj-q8_0.gguf
```

Con l'installer standard la cartella è
`%LOCALAPPDATA%\Programs\ITA-OCR\models`.

Per tenerli altrove — su un altro disco, o condivisi fra installazioni — imposta
la variabile d'ambiente `OCR_ITA_MODELS` sulla cartella che li contiene.

## Come scaricarli

Dalla pagina del modello, scheda **Files**, oppure con il client ufficiale:

```powershell
python -m pip install huggingface_hub
hf download ueuegio/ITA-OCR --local-dir models
```

Verifica l'integrità dopo il download:

```powershell
Get-FileHash .\models\glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf -Algorithm SHA256
Get-FileHash .\models\glm-ocr-base-mmproj-q8_0.gguf -Algorithm SHA256
```

```text
95c6a7b7f318293c4a5276a5dab773d0ef55d49ad357e24f89f9d4a701a5be8a  glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf
2f83f69e7e5268474c8e257606ace1c1c28ff3546e1308a63fb15d3dc2ebf459  glm-ocr-base-mmproj-q8_0.gguf
```

## Usarli fuori dall'app

I GGUF funzionano con llama.cpp senza ITA-OCR. Il prompt di riconoscimento è
quello di GLM-OCR, `Text Recognition:`, senza prompt di sistema.

```bash
llama-mtmd-cli -m glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf \
  --mmproj glm-ocr-base-mmproj-q8_0.gguf \
  --image pagina.png -p "Text Recognition:"
```

L'applicazione avvia invece `llama-server` in loopback e applica una cascata di
tentativi quando la prima decodifica finisce incompleta. Il risultato dalla riga
di comando può quindi differire da quello dell'app sulla stessa pagina.

## Licenza e limiti

I pesi derivano da [GLM-OCR](https://huggingface.co/zai-org/GLM-OCR) di Z.ai,
dichiarato MIT nella model card upstream; il fine-tuning mantiene la stessa
licenza. Vedi [TERZE-PARTI.md](TERZE-PARTI.md).

Il dataset di addestramento resta privato e non è necessario per usare il
modello. Il riconoscimento può omettere, ripetere o inventare testo: vedi
[BENCHMARKS.md](BENCHMARKS.md) per metodo e limiti dichiarati.
