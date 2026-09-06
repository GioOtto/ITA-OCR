# Installing on Linux

*[Versione italiana](../it/INSTALLAZIONE-LINUX.md)*

The Linux build is an **AppImage**: a single executable file that installs
nothing and touches nothing in the system. Download it, make it executable,
run it. To remove it, delete the file.

## Requirements

- x86-64 distribution with **glibc 2.39 or newer**: Ubuntu 24.04 and later,
  Debian 13, Fedora 40+ and derivatives. That is a limit of the published
  binary, not of the program: glibc guarantees forward compatibility, not
  backward, so on older distributions you build it yourself following
  [BUILDING-LINUX.md](BUILDING-LINUX.md). Check yours with `ldd --version`.
- **GTK 3, WebKitGTK 4.1 and libsoup 3** installed: the interface is a WebKit
  window and the AppImage does not carry them. Most desktops already have
  them; if not, see *Common problems*.
- FUSE, to mount the AppImage. Without it, `--appimage-extract-and-run` works.
- About 1.3 GB for the two GGUF weights, which stay **outside** the AppImage.
- Enough memory for the model and the pages. A Vulkan-capable GPU speeds the
  work up; the CPU is a slower alternative at the same quality.

## Steps

1. Open the [release page](https://github.com/GioOtto/ITA-OCR/releases/latest)
   and download `ITA-OCR-v1.0.0-x86_64.AppImage`.
2. Compare the hash with the one published in `SHA256SUMS.txt`:

   ```bash
   sha256sum ITA-OCR-v1.0.0-x86_64.AppImage
   ```

3. Make the file executable:

   ```bash
   chmod +x ITA-OCR-v1.0.0-x86_64.AppImage
   ```

4. Download the two GGUF weights from
   [Hugging Face](https://huggingface.co/ueuegio/ITA-OCR) and put them in a
   `models` folder **next to** the AppImage:

   ```text
   ~/Applications/
     ITA-OCR-v1.0.0-x86_64.AppImage
     models/
       glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf
       glm-ocr-base-mmproj-q8_0.gguf
   ```

   Alternatively, keep them wherever you like and point `OCR_ITA_MODELS` at
   that folder. The file names must be exact: see [MODEL.md](MODEL.md).

5. Start the application:

   ```bash
   ./ITA-OCR-v1.0.0-x86_64.AppImage
   ```

Unlike the Windows installer, the AppImage **does not contain the models**: on
its own it weighs about ninety megabytes, and the two GGUF files would add
1.3 GB, more than GitHub accepts as a single release asset. It is the same
reason the Windows portable folder leaves them out.

## Where it keeps its files

The AppImage writes nothing next to itself. History, settings and logs live in
the user profile, following the XDG conventions:

| Content | Path |
| --- | --- |
| Session history | `~/.local/share/ocr-ita-desktop/` |
| Settings | `~/.config/ocr-ita-desktop/` |
| Logs | `~/.local/share/ocr-ita-desktop/logs/` |
| Models, when not next to the AppImage | `~/.local/share/ocr-ita-desktop/models/` |

If `XDG_DATA_HOME` or `XDG_CONFIG_HOME` are set, the app honours them.

Deleting the AppImage does not remove those folders: remove them by hand to
leave no trace. Check the logs before sharing them — they can contain paths
and fragments of document text.

## Adding it to the application menu

The AppImage does not register itself. The easiest route is
[AppImageLauncher](https://github.com/TheAssassin/AppImageLauncher), which
offers to integrate it on first run and handles this for you.

By hand, write a `.desktop` entry pointing at wherever you put the file:

```bash
mkdir -p ~/.local/share/applications ~/.local/share/icons
APPIMAGE=$HOME/Applications/ITA-OCR-v1.0.0-x86_64.AppImage
"$APPIMAGE" --appimage-extract usr/share/icons/hicolor/256x256/apps/ocr-ita-desktop.png >/dev/null
cp squashfs-root/usr/share/icons/hicolor/256x256/apps/ocr-ita-desktop.png ~/.local/share/icons/
rm -rf squashfs-root
cat > ~/.local/share/applications/ita-ocr.desktop <<END
[Desktop Entry]
Type=Application
Name=ITA-OCR
Comment=Italian handwriting transcription, locally
Exec=$APPIMAGE
Icon=ocr-ita-desktop
Categories=Office;Utility;
Terminal=false
StartupWMClass=ocr-ita-desktop
END
```

Neither step is required to use the application.

## Common problems

**`cannot open shared object file: libwebkit2gtk-4.1.so.0`** — the interface
libraries are missing. On Debian and Ubuntu:

```bash
sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0 libsoup-3.0-0
```

On Fedora: `sudo dnf install webkit2gtk4.1 gtk3 libsoup3`. On Arch:
`sudo pacman -S webkit2gtk-4.1 gtk3 libsoup3`.

**`dlopen(): error loading libfuse.so.2`** — the system no longer ships FUSE 2.
The AppImage is built with the type2 runtime, which should not need it; if it
happens anyway, you can always run without mounting:

```bash
./ITA-OCR-v1.0.0-x86_64.AppImage --appimage-extract-and-run
```

**Model not found** — the two GGUF files are not in `models` next to the
AppImage, or their names differ from the expected ones. Check
[MODEL.md](MODEL.md), or point `OCR_ITA_MODELS` at the right folder:

```bash
OCR_ITA_MODELS=/path/to/weights ./ITA-OCR-v1.0.0-x86_64.AppImage
```

**GPU not used** — the app shows the backend in use. Vulkan needs working
drivers: check with `vulkaninfo --summary` that the system sees the card.
The published AppImage ships the CPU and Vulkan backends; it **does not ship
CUDA**, which has to be compiled on a machine with the NVIDIA toolkit. If you
have an NVIDIA GPU and want CUDA, build it yourself following
[BUILDING-LINUX.md](BUILDING-LINUX.md): the script enables it by itself when
it finds `nvcc`.

**Wayland** — the window runs through the X11 compatibility layer. If you hit
resizing or input problems, try forcing X11 with `GDK_BACKEND=x11`.

**Contextual correction** — requires optional local lexical resources that are
not part of the distribution. The standard corrector works with the public
dictionaries that are included.
