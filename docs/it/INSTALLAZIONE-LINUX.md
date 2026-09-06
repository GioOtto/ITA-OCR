# Installare su Linux

*[English version](../en/INSTALL-LINUX.md)*

La distribuzione per Linux è un'**AppImage**: un file unico, eseguibile, che
non si installa e non tocca il sistema. Si scarica, gli si dà il permesso di
esecuzione e si avvia. Per rimuoverla si cancella il file.

## Requisiti

- Distribuzione x86-64 con **glibc 2.39 o successiva**: Ubuntu 24.04 e più
  recenti, Debian 13, Fedora 40+ e derivate. È il limite del binario
  pubblicato, non del programma: glibc garantisce la compatibilità in avanti e
  non all'indietro, quindi su distribuzioni più anziane si compila in proprio
  seguendo [LEGGIMI-LINUX.md](LEGGIMI-LINUX.md). Per controllare la tua:
  `ldd --version`.
- **GTK 3, WebKitGTK 4.1 e libsoup 3** installati: l'interfaccia è una finestra
  WebKit e l'AppImage non li porta con sé. Quasi tutti i desktop li hanno già;
  in caso contrario, vedi *Problemi comuni*.
- FUSE per montare l'AppImage. Se manca, si può usare `--appimage-extract-and-run`.
- Circa 1,3 GB per i due pesi GGUF, che restano **fuori** dall'AppImage.
- Memoria sufficiente per modello e pagine. Una GPU compatibile con Vulkan
  accelera il lavoro; la CPU è un'alternativa più lenta a parità di qualità.

## Procedura

1. Apri la [pagina della release](https://github.com/GioOtto/ITA-OCR/releases/latest)
   e scarica `ITA-OCR-v1.0.0-x86_64.AppImage`.
2. Confronta l'hash con quello pubblicato in `SHA256SUMS.txt`:

   ```bash
   sha256sum ITA-OCR-v1.0.0-x86_64.AppImage
   ```

3. Rendi il file eseguibile:

   ```bash
   chmod +x ITA-OCR-v1.0.0-x86_64.AppImage
   ```

4. Scarica i due pesi GGUF da
   [Hugging Face](https://huggingface.co/ueuegio/ITA-OCR) e mettili in una
   cartella `models` **accanto** all'AppImage:

   ```text
   ~/Applicazioni/
     ITA-OCR-v1.0.0-x86_64.AppImage
     models/
       glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf
       glm-ocr-base-mmproj-q8_0.gguf
   ```

   In alternativa, tienili dove preferisci e indica la cartella con
   `OCR_ITA_MODELS`. I nomi dei file devono essere esatti: vedi
   [MODELLO.md](MODELLO.md).

5. Avvia l'applicazione:

   ```bash
   ./ITA-OCR-v1.0.0-x86_64.AppImage
   ```

A differenza dell'installer Windows, l'AppImage **non contiene i modelli**: da
sola pesa una novantina di megabyte e i due GGUF ne aggiungerebbero 1,3 GB, che
GitHub non accetta come allegato singolo di una release. È lo stesso motivo per
cui la cartella portabile di Windows li lascia fuori.

## Dove mette i suoi file

L'AppImage non scrive niente accanto a sé. Archivio, impostazioni e log stanno
nel profilo utente, secondo le convenzioni XDG:

| Contenuto | Percorso |
| --- | --- |
| Archivio delle sessioni | `~/.local/share/ocr-ita-desktop/` |
| Impostazioni | `~/.config/ocr-ita-desktop/` |
| Log | `~/.local/share/ocr-ita-desktop/logs/` |
| Modelli, se non stanno accanto all'AppImage | `~/.local/share/ocr-ita-desktop/models/` |

Se `XDG_DATA_HOME` o `XDG_CONFIG_HOME` sono impostate, l'app le rispetta.

Cancellare l'AppImage non rimuove queste cartelle: vanno eliminate a mano se
si vuole togliere ogni traccia. Controlla i log prima di condividerli, perché
possono contenere percorsi e frammenti di testo dei documenti.

## Integrazione nel menu applicazioni

L'AppImage non si registra da sola nel menu. Il modo più comodo è
[AppImageLauncher](https://github.com/TheAssassin/AppImageLauncher), che al
primo avvio propone di integrarla e se ne occupa da solo.

A mano, si scrive una voce `.desktop` che punta al file dove lo hai messo:

```bash
mkdir -p ~/.local/share/applications ~/.local/share/icons
APPIMAGE=$HOME/Applicazioni/ITA-OCR-v1.0.0-x86_64.AppImage
"$APPIMAGE" --appimage-extract usr/share/icons/hicolor/256x256/apps/ocr-ita-desktop.png >/dev/null
cp squashfs-root/usr/share/icons/hicolor/256x256/apps/ocr-ita-desktop.png ~/.local/share/icons/
rm -rf squashfs-root
cat > ~/.local/share/applications/ita-ocr.desktop <<FINE
[Desktop Entry]
Type=Application
Name=ITA-OCR
Comment=Trascrizione di manoscritti italiani, in locale
Exec=$APPIMAGE
Icon=ocr-ita-desktop
Categories=Office;Utility;
Terminal=false
StartupWMClass=ocr-ita-desktop
FINE
```

Nessuna delle due cose è necessaria per usare l'applicazione.

## Problemi comuni

**`cannot open shared object file: libwebkit2gtk-4.1.so.0`.** Mancano le
librerie dell'interfaccia. Su Debian e Ubuntu:

```bash
sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0 libsoup-3.0-0
```

Su Fedora: `sudo dnf install webkit2gtk4.1 gtk3 libsoup3`. Su Arch:
`sudo pacman -S webkit2gtk-4.1 gtk3 libsoup3`.

**`dlopen(): error loading libfuse.so.2`.** Il sistema non ha più FUSE 2.
L'AppImage è costruita con il runtime type2, che non dovrebbe richiederlo; se
succede lo stesso, si può sempre eseguire senza montarla:

```bash
./ITA-OCR-v1.0.0-x86_64.AppImage --appimage-extract-and-run
```

**Modello non trovato.** I due GGUF non sono in `models` accanto all'AppImage,
oppure hanno nomi diversi da quelli attesi. Controlla
[MODELLO.md](MODELLO.md), o punta `OCR_ITA_MODELS` alla cartella giusta:

```bash
OCR_ITA_MODELS=/percorso/ai/pesi ./ITA-OCR-v1.0.0-x86_64.AppImage
```

**GPU non usata.** L'app mostra il backend in uso. Vulkan richiede driver
funzionanti: verifica con `vulkaninfo --summary` che il sistema veda la scheda.
L'AppImage pubblicata include i backend CPU e Vulkan; **non include CUDA**,
perché va compilato sulla macchina che ha il toolkit NVIDIA. Chi ha una GPU
NVIDIA e vuole CUDA se la compila seguendo
[LEGGIMI-LINUX.md](LEGGIMI-LINUX.md): lo script lo abilita da solo se trova `nvcc`.

**Wayland.** La finestra gira attraverso il livello di compatibilità X11. Se
incontri problemi di ridimensionamento o di input, prova a forzare X11 con
`GDK_BACKEND=x11`.

**Correzione contestuale.** Richiede risorse lessicali locali opzionali che non
fanno parte della distribuzione. Il correttore standard funziona con i soli
dizionari pubblici inclusi.
