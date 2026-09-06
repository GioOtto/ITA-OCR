# Contribuire a ITA-OCR

Apri una issue per descrivere il problema o la modifica desiderata, poi una
pull request circoscritta con motivazione e verifiche eseguite.

- Usa solo esempi sintetici creati per la riproduzione.
- Non aggiungere dataset, pesi, log, documenti reali o percorsi della tua macchina.
- Mantieni l’inferenza locale e documenta qualsiasi nuova connessione di rete.
- Conserva licenze e attribuzioni delle dipendenze.
- Segnala nella pull request l’eventuale uso di strumenti AI e controllane l’output.

Per il backend esegui `cargo test --locked` nella directory del crate Tauri.
Per l’interfaccia esegui `python ocr-desktop/smoke/ui.py` dopo aver installato
Playwright e Chromium. La guida Windows descrive la compilazione completa.
Esegui anche `python scripts/verifica-pubblicazione.py` prima di proporre un rilascio.

I contributi al codice originale vengono proposti sotto la licenza MIT del
progetto. Il dataset privato non è necessario per contribuire all’applicazione.
