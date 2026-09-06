# Contribuire a ITA-OCR · Contributing

## Italiano

Apri una issue per descrivere il problema o la modifica desiderata, poi una
pull request circoscritta con motivazione e verifiche eseguite.

### Regole del progetto

- Usa solo esempi sintetici creati per la riproduzione.
- Non aggiungere dataset, pesi, log, documenti reali o percorsi della tua macchina.
- Mantieni l’inferenza locale e documenta qualsiasi nuova connessione di rete.
- Conserva licenze e attribuzioni delle dipendenze.
- Segnala nella pull request l’eventuale uso di strumenti AI e controllane l’output.
- Il codice e i commenti del progetto sono in italiano: mantieni quella lingua nei sorgenti.

### Verifiche prima di proporre

```powershell
cd ocr-desktop/app/src-tauri; cargo test --locked
python ocr-desktop/smoke/ui.py
python scripts/verifica-pubblicazione.py
```

La prima riguarda il backend, la seconda l’interfaccia con Playwright e
Chromium, la terza controlla che non stia per uscire materiale privato.
La [guida Windows](../docs/it/LEGGIMI-WINDOWS.md) descrive la compilazione completa.

Se cambi schemi o schermate, rigenerali invece di modificarli a mano:
`python scripts/genera-diagrammi.py` e `python scripts/genera-schermate.py`.

I contributi al codice originale vengono proposti sotto la licenza MIT del
progetto. Il dataset privato non è necessario per contribuire all’applicazione.

## English

Open an issue describing the problem or the change you have in mind, then a
focused pull request with your reasoning and the checks you ran.

### Project rules

- Use only synthetic examples created for the reproduction.
- Never add datasets, weights, logs, real documents or paths from your machine.
- Keep inference local, and document any new network connection.
- Preserve dependency licences and attributions.
- Disclose any use of AI tools in the pull request, and review their output.
- The project's code and comments are written in Italian: keep that language in the sources.

### Checks before proposing

```powershell
cd ocr-desktop/app/src-tauri; cargo test --locked
python ocr-desktop/smoke/ui.py
python scripts/verifica-pubblicazione.py
```

The first covers the backend, the second the interface through Playwright and
Chromium, the third makes sure no private material is about to be published.
The [build guide](../docs/en/BUILDING.md) covers the full compilation.

If you change diagrams or screenshots, regenerate them instead of editing them
by hand: `python scripts/genera-diagrammi.py` and
`python scripts/genera-schermate.py`.

Contributions to the original code are offered under the project's MIT licence.
The private dataset is not needed to contribute to the application.

---

Contatto diretto · Direct contact: **giorgio.ottoboni@proton.me**
